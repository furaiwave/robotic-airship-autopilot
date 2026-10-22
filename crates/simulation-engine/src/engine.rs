//! Fixed-step simulation loop.
//!
//! One physics step of length `dt` at time `t`:
//!
//! 1. advance the wind field (disturbance triggers, turbulence),
//! 2. evaluate forces at the current state (for telemetry and sensors),
//! 3. every `control_period / dt` steps: measure, run the autopilot, apply mooring commands,
//! 4. record telemetry every `telemetry_period / dt` steps (state at time `t`),
//! 5. advance actuators (lag, rate limits) and integrate energy,
//! 6. integrate the equations of motion with RK4 from `t` to `t + dt`,
//! 7. detect contact onset and run numerical and structural validity checks.
//!
//! The physical time step never depends on how the caller chunks execution
//! (`advance(n)`), on the replay speed or on wall-clock time, so runs with identical
//! configuration and seeds are bit-identical.

use std::time::{Duration, Instant};

use airship_model::math::{wind_from_direction_deg, wrap_pi, yaw_to_heading_deg};
use airship_model::{Vec2, Vec3};
use autopilot::{
    Autopilot, AutopilotOutput, FlightPhase, MooringCommand, SafetyConstraint, SafetyEvent,
    SafetyResponse, SafetySeverity, StateSnapshot,
};
use flight_dynamics::environment::DisturbanceActivation;
use flight_dynamics::{
    ActuatorCommand, ActuatorLimits, ActuatorState, AirshipDynamics, ForceBreakdown, SensorModel,
    TruthSnapshot, VehicleState, WindField, thruster_power,
};
use serde::{Deserialize, Serialize};

use crate::config::SimulationConfig;
use crate::landing::TouchdownRecord;
use crate::telemetry::TelemetrySample;

/// Vertical impact speed treated as a structural failure (m/s).
pub const CRASH_SINK_RATE_M_S: f64 = 2.5;
/// Velocity magnitude treated as numerical divergence (m/s).
pub const DIVERGENCE_SPEED_M_S: f64 = 60.0;
/// Attitude magnitude treated as loss of control / divergence (deg).
pub const DIVERGENCE_ATTITUDE_DEG: f64 = 75.0;

#[derive(Debug, thiserror::Error)]
pub enum SimulationError {
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    /// The autopilot reached `Landed`.
    Landed,
    /// The autopilot reached `Aborted`.
    Aborted,
    /// Simulated duration limit reached.
    Timeout,
    /// Fatal numerical or structural failure.
    Fatal,
    /// Stopped by the user.
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulationStatus {
    pub time_s: f64,
    pub duration_s: f64,
    pub steps: u64,
    pub phase: FlightPhase,
    pub guidance_mode: String,
    pub finished: Option<FinishReason>,
    pub finish_message: Option<String>,
    pub telemetry_count: usize,
    pub transitions: usize,
    pub safety_events: usize,
    pub wall_time_ms: f64,
}

pub struct Simulation {
    pub(crate) cfg: SimulationConfig,
    pub(crate) dynamics: AirshipDynamics,
    wind: WindField,
    sensors: SensorModel,
    pub(crate) autopilot: Autopilot,
    limits: ActuatorLimits,
    state: VehicleState,
    actuators: ActuatorState,
    command: ActuatorCommand,
    moored: bool,
    t: f64,
    step: u64,
    control_every: u64,
    telemetry_every: u64,
    energy_j: f64,
    power_w: f64,
    pub(crate) telemetry: Vec<TelemetrySample>,
    last_output: Option<AutopilotOutput>,
    pub(crate) disturbances: Vec<DisturbanceActivation>,
    pub(crate) touchdowns: Vec<TouchdownRecord>,
    in_contact: bool,
    been_airborne: bool,
    pub(crate) reached_target_area: bool,
    battery_empty_reported: bool,
    finished: Option<FinishReason>,
    finish_message: Option<String>,
    wall: Duration,
}

impl Simulation {
    pub fn new(cfg: SimulationConfig) -> Result<Self, SimulationError> {
        let report = cfg.validate();
        if report.has_errors() {
            let msg = report
                .errors()
                .map(|e| format!("{}: {}", e.field, e.message))
                .collect::<Vec<_>>()
                .join("; ");
            return Err(SimulationError::InvalidConfig(msg));
        }
        let env = &cfg.environment;
        let dynamics = AirshipDynamics::new(&cfg.airship, &env.atmosphere, env.ground_elevation_m);
        let init = &env.initial_state;
        let state = VehicleState {
            position: Vec3::new(
                init.position.x,
                init.position.y,
                env.ground_elevation_m + cfg.airship.gear_height_m + init.altitude_agl_m,
            ),
            velocity: init.velocity,
            yaw: airship_model::math::heading_deg_to_yaw(init.heading_deg),
            yaw_rate: 0.0,
            pitch: init.pitch_deg.to_radians(),
            pitch_rate: 0.0,
            roll: init.roll_deg.to_radians(),
            roll_rate: 0.0,
        };
        let s = &cfg.simulation;
        let control_every = ((1.0 / s.control_rate_hz) / s.dt_s).round().max(1.0) as u64;
        let telemetry_every = ((1.0 / s.telemetry_rate_hz) / s.dt_s).round().max(1.0) as u64;
        let autopilot = Autopilot::new(
            &cfg.autopilot,
            &cfg.controller,
            &cfg.airship,
            env,
            &cfg.flight_path,
        );
        Ok(Self {
            wind: WindField::new(&env.wind, env.landing_site),
            sensors: SensorModel::new(&env.sensor_noise),
            limits: ActuatorLimits::from_params(&cfg.airship),
            in_contact: init.altitude_agl_m <= 0.0,
            been_airborne: init.altitude_agl_m > 0.3,
            moored: init.moored,
            dynamics,
            autopilot,
            state,
            actuators: ActuatorState::default(),
            command: ActuatorCommand::default(),
            t: 0.0,
            step: 0,
            control_every,
            telemetry_every,
            energy_j: 0.0,
            power_w: 0.0,
            telemetry: Vec::new(),
            last_output: None,
            disturbances: Vec::new(),
            touchdowns: Vec::new(),
            reached_target_area: false,
            battery_empty_reported: false,
            finished: None,
            finish_message: None,
            wall: Duration::ZERO,
            cfg,
        })
    }

    pub fn config(&self) -> &SimulationConfig {
        &self.cfg
    }

    pub fn time(&self) -> f64 {
        self.t
    }

    pub fn state(&self) -> &VehicleState {
        &self.state
    }

    pub fn telemetry(&self) -> &[TelemetrySample] {
        &self.telemetry
    }

    pub fn autopilot(&self) -> &Autopilot {
        &self.autopilot
    }

    pub fn finished(&self) -> Option<FinishReason> {
        self.finished
    }

    pub fn finish_message(&self) -> Option<&str> {
        self.finish_message.as_deref()
    }

    pub fn wall_time(&self) -> Duration {
        self.wall
    }

    pub fn energy_wh(&self) -> f64 {
        self.energy_j / 3600.0
    }

    fn battery_soc(&self) -> f64 {
        (1.0 - self.energy_wh() / self.cfg.airship.propulsion.battery_capacity_wh).clamp(0.0, 1.0)
    }

    fn gear_agl(&self) -> f64 {
        self.dynamics.gear_agl(&self.state)
    }

    pub fn status(&self) -> SimulationStatus {
        SimulationStatus {
            time_s: self.t,
            duration_s: self.cfg.simulation.duration_s,
            steps: self.step,
            phase: self.autopilot.phase(),
            guidance_mode: self
                .last_output
                .as_ref()
                .map(|o| o.guidance.mode.clone())
                .unwrap_or_default(),
            finished: self.finished,
            finish_message: self.finish_message.clone(),
            telemetry_count: self.telemetry.len(),
            transitions: self.autopilot.transitions.len(),
            safety_events: self.autopilot.safety.events.len(),
            wall_time_ms: self.wall.as_secs_f64() * 1000.0,
        }
    }

    /// Stop the run at the current time (user request).
    pub fn stop(&mut self) {
        if self.finished.is_none() {
            self.finish(
                FinishReason::Stopped,
                format!("stopped by the user at t = {:.1} s", self.t),
            );
        }
    }

    fn finish(&mut self, reason: FinishReason, message: String) {
        self.finished = Some(reason);
        self.finish_message = Some(message);
        if self.telemetry.last().map(|s| s.t_s) != Some(self.t) {
            let wind = self
                .wind
                .wind_at(self.t, self.gear_agl() + self.cfg.airship.gear_height_m);
            let (_, forces) =
                self.dynamics
                    .evaluate(&self.state, &self.actuators, wind, self.moored);
            self.record(&forces, wind);
        }
    }

    fn fatal(&mut self, constraint: SafetyConstraint, message: String) {
        let dist = (self.state.horizontal_position() - self.cfg.environment.landing_site).norm();
        let snap = self.snapshot(dist);
        self.autopilot.safety.push(SafetyEvent {
            time_s: self.t,
            severity: SafetySeverity::Fatal,
            constraint,
            message: message.clone(),
            phase: self.autopilot.phase(),
            value: 0.0,
            limit: 0.0,
            response: SafetyResponse::Terminate,
            state: snap,
            cleared_at_s: None,
        });
        self.finish(FinishReason::Fatal, message);
    }

    fn snapshot(&self, dist: f64) -> StateSnapshot {
        let s = &self.state;
        StateSnapshot {
            x_m: s.position.x,
            y_m: s.position.y,
            altitude_agl_m: self.gear_agl(),
            ground_speed_m_s: s.ground_speed(),
            vertical_speed_m_s: s.velocity.z,
            airspeed_m_s: 0.0,
            heading_deg: yaw_to_heading_deg(s.yaw),
            pitch_deg: s.pitch.to_degrees(),
            roll_deg: s.roll.to_degrees(),
            distance_to_landing_m: dist,
        }
    }

    /// Advance by at most `n` physics steps. Returns the number of steps executed.
    pub fn advance(&mut self, n: u64) -> u64 {
        let start = Instant::now();
        let mut done = 0;
        while done < n && self.finished.is_none() {
            self.step_once();
            done += 1;
        }
        self.wall += start.elapsed();
        done
    }

    /// Advance by `seconds` of simulated time.
    pub fn advance_time(&mut self, seconds: f64) -> u64 {
        let n = (seconds / self.cfg.simulation.dt_s).round().max(0.0) as u64;
        self.advance(n)
    }

    pub fn run_to_completion(&mut self) {
        let n = (self.cfg.simulation.duration_s / self.cfg.simulation.dt_s).ceil() as u64 + 2;
        self.advance(n);
    }

    fn step_once(&mut self) {
        let dt = self.cfg.simulation.dt_s;
        let t = self.t;
        let gear_h = self.cfg.airship.gear_height_m;
        let landing = self.cfg.environment.landing_site;

        // 1. Wind field.
        let activated = self.wind.advance(t, dt, self.state.horizontal_position());
        self.disturbances.extend(activated);
        let wind = self.wind.wind_at(t, self.gear_agl() + gear_h);

        // 2. Forces at the current state.
        let (_, forces) = self
            .dynamics
            .evaluate(&self.state, &self.actuators, wind, self.moored);

        // 3. Autopilot.
        if self.step % self.control_every == 0 {
            let truth = TruthSnapshot {
                time_s: t,
                state: self.state,
                wind,
                ground_elevation_m: self.cfg.environment.ground_elevation_m,
                gear_height_m: gear_h,
                air_density: forces.air_density,
                ground_normal_n: forces.ground_normal_n,
                moored: self.moored,
                mooring_load_n: if self.moored { forces.total.z } else { 0.0 },
                battery_soc: self.battery_soc(),
            };
            let meas = self.sensors.measure(&truth);
            let control_dt = dt * self.control_every as f64;
            let out = self.autopilot.step(&meas, control_dt);
            self.command = out.command;
            match out.mooring {
                Some(MooringCommand::Release) => self.moored = false,
                Some(MooringCommand::Engage)
                    if forces.ground_normal_n > 0.0 && self.state.velocity.norm() < 1.0 =>
                {
                    self.moored = true;
                    self.state.velocity = Vec3::ZERO;
                    self.state.yaw_rate = 0.0;
                    self.state.pitch_rate = 0.0;
                    self.state.roll_rate = 0.0;
                }
                _ => {}
            }
            self.last_output = Some(out);
        }

        // 4. Telemetry (state at time t).
        if self.step % self.telemetry_every == 0 {
            self.record(&forces, wind);
        }

        // 5. Actuators and energy.
        if self.battery_soc() <= 0.0 {
            self.command = ActuatorCommand::default();
            if !self.battery_empty_reported {
                self.battery_empty_reported = true;
                let snap = self.snapshot((self.state.horizontal_position() - landing).norm());
                self.autopilot.safety.push(SafetyEvent {
                    time_s: t,
                    severity: SafetySeverity::Critical,
                    constraint: SafetyConstraint::Battery,
                    message: "battery depleted: propulsion unavailable".into(),
                    phase: self.autopilot.phase(),
                    value: 0.0,
                    limit: 0.0,
                    response: SafetyResponse::LogOnly,
                    state: snap,
                    cleared_at_s: None,
                });
            }
        }
        self.actuators.advance(&self.command, &self.limits, dt);
        self.power_w = self.propulsion_power(&forces);
        self.energy_j += self.power_w * dt;

        // 6. Integrate.
        let wind_field = &self.wind;
        self.state = self.dynamics.step_rk4(
            &self.state,
            &self.actuators,
            self.moored,
            t,
            dt,
            |ti, agl| wind_field.wind_at(ti, agl),
        );
        self.step += 1;
        self.t = self.step as f64 * dt;

        // 7. Contacts and validity.
        self.post_step_checks();
        if self.finished.is_none() {
            match self.autopilot.phase() {
                FlightPhase::Landed => {
                    let msg = self.autopilot.terminal_reason.clone().unwrap_or_default();
                    self.finish(
                        FinishReason::Landed,
                        format!("landing sequence completed: {msg}"),
                    );
                }
                FlightPhase::Aborted => {
                    let msg = self.autopilot.terminal_reason.clone().unwrap_or_default();
                    self.finish(FinishReason::Aborted, format!("mission aborted: {msg}"));
                }
                _ if self.t >= self.cfg.simulation.duration_s - 1e-9 => {
                    let phase = self.autopilot.phase().label();
                    self.finish(
                        FinishReason::Timeout,
                        format!(
                            "duration limit {:.0} s reached in phase {phase}",
                            self.cfg.simulation.duration_s
                        ),
                    );
                }
                _ => {}
            }
        }
    }

    fn propulsion_power(&self, f: &ForceBreakdown) -> f64 {
        let p = &self.cfg.airship.propulsion;
        let rho = f.air_density;
        let disk = std::f64::consts::PI * p.propeller_diameter_m.powi(2) / 4.0;
        let a = &self.actuators;
        let u = f.air_velocity_body.x;
        let w = f.air_velocity_body.z;
        let main = |t: f64| thruster_power(t, u * t.signum(), rho, disk, p.efficiency);
        main(a.thrust_left_n)
            + main(a.thrust_right_n)
            + thruster_power(
                a.thrust_heave_n,
                w * a.thrust_heave_n.signum(),
                rho,
                disk,
                p.efficiency,
            )
            + p.avionics_power_w
    }

    fn post_step_checks(&mut self) {
        let s = self.state;
        if !s.is_finite() {
            self.fatal(
                SafetyConstraint::NumericalValidity,
                format!(
                    "non-finite state at t = {:.2} s: numerical integration diverged",
                    self.t
                ),
            );
            return;
        }
        if s.velocity.norm() > DIVERGENCE_SPEED_M_S
            || s.pitch.to_degrees().abs() > DIVERGENCE_ATTITUDE_DEG
            || s.roll.to_degrees().abs() > DIVERGENCE_ATTITUDE_DEG
        {
            self.fatal(
                SafetyConstraint::NumericalValidity,
                format!(
                    "state outside the validity range of the model (speed {:.1} m/s, pitch {:.0} deg, roll {:.0} deg)",
                    s.velocity.norm(),
                    s.pitch.to_degrees(),
                    s.roll.to_degrees()
                ),
            );
            return;
        }
        let agl = self.gear_agl();
        let env = &self.cfg.environment;
        let landing = env.landing_site;
        let p = s.horizontal_position();
        let dist = (p - landing).norm();
        if agl > 0.3 {
            self.been_airborne = true;
        }
        let flare_h = self.cfg.autopilot.landing.flare_height_m;
        if agl <= flare_h + 1.0
            && dist <= self.cfg.autopilot.landing.landing_point_tolerance_m
            && self.been_airborne
        {
            self.reached_target_area = true;
        }
        if agl < -0.5 * self.cfg.airship.gear_height_m {
            self.fatal(
                SafetyConstraint::GroundCollision,
                format!(
                    "ground penetration of {:.2} m: hull struck the ground",
                    -agl
                ),
            );
            return;
        }
        let contact = agl <= 0.0;
        if contact && !self.in_contact && self.been_airborne && !self.moored {
            let (along, cross) = env.landing_area.local(landing, p);
            let rec = TouchdownRecord {
                time_s: self.t,
                phase: self.autopilot.phase(),
                x_m: p.x,
                y_m: p.y,
                distance_to_target_m: dist,
                along_axis_m: along,
                cross_axis_m: cross,
                horizontal_speed_m_s: s.ground_speed(),
                vertical_speed_m_s: s.velocity.z,
                inside_landing_area: env.landing_area.contains(landing, p),
            };
            self.assess_contact(&rec);
            self.touchdowns.push(rec);
            if -s.velocity.z > CRASH_SINK_RATE_M_S {
                self.fatal(
                    SafetyConstraint::GroundCollision,
                    format!(
                        "ground impact at {:.2} m/s exceeds the structural limit of {:.1} m/s",
                        -s.velocity.z, CRASH_SINK_RATE_M_S
                    ),
                );
                return;
            }
        }
        self.in_contact = contact;
    }

    fn assess_contact(&mut self, rec: &TouchdownRecord) {
        let l = self.cfg.autopilot.landing.clone();
        let snap = self.snapshot(rec.distance_to_target_m);
        let phase = self.autopilot.phase();
        let mut push = |constraint, message: String, value: f64, limit: f64| {
            self.autopilot.safety.push(SafetyEvent {
                time_s: rec.time_s,
                severity: SafetySeverity::Critical,
                constraint,
                message,
                phase,
                value,
                limit,
                response: SafetyResponse::LogOnly,
                state: snap,
                cleared_at_s: None,
            });
        };
        if -rec.vertical_speed_m_s > l.max_touchdown_vertical_speed_m_s {
            push(
                SafetyConstraint::TouchdownSpeed,
                format!(
                    "hard touchdown: sink rate {:.2} m/s > {:.2} m/s",
                    -rec.vertical_speed_m_s, l.max_touchdown_vertical_speed_m_s
                ),
                -rec.vertical_speed_m_s,
                l.max_touchdown_vertical_speed_m_s,
            );
        }
        if rec.horizontal_speed_m_s > l.max_touchdown_horizontal_speed_m_s {
            push(
                SafetyConstraint::TouchdownSpeed,
                format!(
                    "touchdown with horizontal speed {:.2} m/s > {:.2} m/s",
                    rec.horizontal_speed_m_s, l.max_touchdown_horizontal_speed_m_s
                ),
                rec.horizontal_speed_m_s,
                l.max_touchdown_horizontal_speed_m_s,
            );
        }
        if !rec.inside_landing_area {
            push(
                SafetyConstraint::LandingZone,
                format!(
                    "ground contact outside the prepared landing area (along {:+.1} m, cross {:+.1} m)",
                    rec.along_axis_m, rec.cross_axis_m
                ),
                rec.distance_to_target_m,
                0.0,
            );
        }
    }

    fn record(&mut self, f: &ForceBreakdown, wind: Vec3) {
        let s = &self.state;
        let gear_h = self.cfg.airship.gear_height_m;
        let agl = self.gear_agl();
        let heading = yaw_to_heading_deg(s.yaw);
        let out = self.last_output.as_ref();
        let (refs, cmd, alloc, gains, sat) = match out {
            Some(o) => (
                o.control.references,
                o.control.virtual_command,
                o.control.allocation,
                Some(o.control.gains),
                o.control.saturation,
            ),
            None => Default::default(),
        };
        let guidance = out.map(|o| o.guidance.clone()).unwrap_or_default();
        let lim = self.autopilot.force_limits();
        let norm = |v: f64, pos: f64, neg: f64| {
            if v >= 0.0 {
                if pos > 0.0 { v / pos } else { 0.0 }
            } else if neg < 0.0 {
                v / neg
            } else {
                0.0
            }
        };
        let v_ref = refs.ground_velocity;
        let horizontal_active = out.is_some_and(|o| {
            o.control.virtual_command.surge_n != 0.0
                || o.control.virtual_command.yaw_moment_nm != 0.0
                || refs.ground_velocity != Vec2::ZERO
        });
        let alt_ref = guidance.altitude_ref_m;
        let sample = TelemetrySample {
            t_s: self.t,
            phase: self.autopilot.phase(),
            x_m: s.position.x,
            y_m: s.position.y,
            altitude_msl_m: s.position.z - gear_h,
            altitude_agl_m: agl,
            vx_m_s: s.velocity.x,
            vy_m_s: s.velocity.y,
            vz_m_s: s.velocity.z,
            ground_speed_m_s: s.ground_speed(),
            airspeed_m_s: f.airspeed,
            airspeed_forward_m_s: f.air_velocity_body.x,
            heading_deg: heading,
            pitch_deg: s.pitch.to_degrees(),
            roll_deg: s.roll.to_degrees(),
            yaw_rate_deg_s: s.yaw_rate.to_degrees(),
            wind_x_m_s: wind.x,
            wind_y_m_s: wind.y,
            wind_z_m_s: wind.z,
            wind_speed_m_s: wind.horizontal().norm(),
            wind_from_deg: wind_from_direction_deg(wind.horizontal()),
            ref_altitude_agl_m: alt_ref,
            ref_vz_m_s: refs.vertical_speed,
            ref_heading_deg: yaw_to_heading_deg(refs.yaw),
            ref_yaw_rate_deg_s: refs.yaw_rate.to_degrees(),
            ref_airspeed_m_s: refs.airspeed,
            ref_vx_m_s: v_ref.x,
            ref_vy_m_s: v_ref.y,
            err_altitude_m: alt_ref.map(|r| r - agl),
            err_cross_track_m: guidance.cross_track_error_m,
            err_heading_deg: if horizontal_active {
                wrap_pi(refs.yaw - s.yaw).to_degrees()
            } else {
                0.0
            },
            err_vz_m_s: refs.vertical_speed - s.velocity.z,
            err_horizontal_velocity_m_s: if horizontal_active {
                (v_ref - s.velocity.horizontal()).norm()
            } else {
                0.0
            },
            distance_to_landing_m: guidance.distance_to_landing_m,
            distance_to_go_m: guidance.distance_to_go_m,
            active_waypoint: guidance.active_waypoint,
            landing_attempt: guidance.landing_attempt,
            cmd_surge_n: cmd.surge_n,
            cmd_heave_n: cmd.heave_n,
            cmd_yaw_moment_nm: cmd.yaw_moment_nm,
            thrust_left_n: self.actuators.thrust_left_n,
            thrust_right_n: self.actuators.thrust_right_n,
            thrust_heave_n: self.actuators.thrust_heave_n,
            rudder_deg: self.actuators.rudder_rad.to_degrees(),
            u_surge: norm(alloc.achieved.surge_n, lim.surge_max_n, lim.surge_min_n).abs(),
            u_heave: norm(alloc.achieved.heave_n, lim.heave_max_n, lim.heave_min_n).abs(),
            u_yaw: (alloc.achieved.yaw_moment_nm.abs() / lim.yaw_max_nm.max(1e-9)).min(1.0),
            sat_surge: sat.surge,
            sat_heave: sat.heave,
            sat_yaw: sat.yaw,
            buoyancy_n: f.buoyancy.z,
            weight_n: -f.gravity.z,
            aero_x_n: f.aero.x,
            aero_y_n: f.aero.y,
            aero_z_n: f.aero.z,
            thrust_z_n: f.thrust.z,
            ground_force_n: f.ground.z,
            mooring_load_n: if self.moored { f.total.z } else { 0.0 },
            contact: f.ground_normal_n > 0.0,
            moored: self.moored,
            power_w: self.power_w,
            energy_wh: self.energy_wh(),
            battery_soc: self.battery_soc(),
            air_density_kg_m3: f.air_density,
            gain_vz_kp: gains.map(|g| g.vertical_speed.kp).unwrap_or(0.0),
            gain_yaw_rate_kp: gains.map(|g| g.yaw_rate.kp).unwrap_or(0.0),
            gain_airspeed_kp: gains.map(|g| g.airspeed.kp).unwrap_or(0.0),
            gain_position: gains.map(|g| g.position).unwrap_or(0.0),
        };
        self.telemetry.push(sample);
    }
}
