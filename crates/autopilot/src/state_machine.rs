//! Explicit take-off and landing state machine.
//!
//! At every control step the autopilot
//! 1. filters the wind estimate,
//! 2. runs the safety monitor and applies requested responses,
//! 3. evaluates the exit conditions of the current phase (at most one transition per step),
//! 4. generates the phase references (guidance) and runs the selected flight controller.
//!
//! Every transition is logged with the simulation time, a human-readable reason that
//! quotes the measured values that satisfied the condition, and a state snapshot.
//! Dwell times appear only as *additional* conditions (e.g. "settled for 3 s"), and
//! timeouts only lead to failure handling, never to nominal progress.
//!
//! | Phase | Active loops | Exit (nominal) | Failure / abort |
//! |---|---|---|---|
//! | Preflight | none | all checks pass | any check fails -> Aborted |
//! | TakeoffPreparation | open-loop heave ramp | mooring load >= threshold and wind ok | timeout -> Aborted |
//! | Takeoff | vz, position hold, heading | gear height >= clearance and climbing | timeout -> Aborted |
//! | InitialClimb | vz, position hold, heading | gear height >= initial-climb height | - |
//! | Climb | altitude, path/hold, heading, speed | altitude captured | critical events logged |
//! | Cruise | altitude, path, heading, speed | all waypoints captured | battery critical -> Approach |
//! | Approach | altitude, approach line, heading, speed | established at the gate | not established at gate -> GoAround |
//! | Descent | glide path, line, heading, speed | low, slow and over the point | deviation/wind/overshoot -> GoAround |
//! | Flare | vz, position hold, heading | weight on gear | drift/wind above DH -> GoAround |
//! | Touchdown | open-loop hold-down | slow on the ground | bounce -> Flare |
//! | PostLanding | open-loop hold-down | settled for settle_time | bounce -> Flare |
//! | GoAround | altitude, course, heading, speed | approach altitude reached | attempts exhausted -> Aborted |

use airship_model::math::{heading_deg_to_yaw, wrap_pi, yaw_to_heading_deg};
use airship_model::{AirshipParams, DerivedProperties, G0, ValidationReport, Vec2};
use control_algorithms::{
    ControlInput, ControlMeasurement, ControlOutput, ControlReferences, ControllerConfig,
    FlightController, ForceLimits, HorizontalReference, LowPass, VerticalReference,
    build_controller,
};
use flight_dynamics::{ActuatorCommand, EnvironmentConfig, Measurement};
use serde::{Deserialize, Serialize};

use crate::config::AutopilotConfig;
use crate::flight_path::{FlightPath, validate_flight_path};
use crate::guidance::{ApproachGeometry, PathFollower, adaptive_lookahead};
use crate::phase::{FlightPhase, PhaseTransition, StateSnapshot};
use crate::preflight::{PreflightCheck, run_preflight};
use crate::safety::{
    SafetyConstraint, SafetyEvent, SafetyInputs, SafetyMonitor, SafetyResponse, SafetySeverity,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MooringCommand {
    Release,
    Engage,
}

/// Guidance quantities exposed to telemetry.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GuidanceStatus {
    pub mode: String,
    pub altitude_ref_m: Option<f64>,
    pub cross_track_error_m: Option<f64>,
    pub distance_to_landing_m: f64,
    pub distance_to_go_m: f64,
    pub active_waypoint: Option<usize>,
    pub landing_attempt: u32,
    pub heave_trim_n: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AutopilotOutput {
    pub command: ActuatorCommand,
    pub mooring: Option<MooringCommand>,
    pub phase: FlightPhase,
    pub guidance: GuidanceStatus,
    pub control: ControlOutput,
    pub new_events: Vec<SafetyEvent>,
    pub new_transition: Option<PhaseTransition>,
}

/// Fraction of the vehicle climb/descent limits used by the reference generation.
pub const REFERENCE_RATE_MARGIN: f64 = 0.8;

struct Plan {
    refs: ControlReferences,
    guidance: GuidanceStatus,
}

pub struct Autopilot {
    cfg: AutopilotConfig,
    vehicle: AirshipParams,
    derived: DerivedProperties,
    path: FlightPath,
    path_report: ValidationReport,
    takeoff_site: Vec2,
    landing_site: Vec2,
    pub approach: ApproachGeometry,
    controller: Box<dyn FlightController>,
    limits: ForceLimits,
    phase: FlightPhase,
    phase_since: f64,
    pub transitions: Vec<PhaseTransition>,
    pub safety: SafetyMonitor,
    follower: PathFollower,
    wind_x: LowPass,
    wind_y: LowPass,
    attempts: u32,
    bounces: u32,
    prep_heave_n: f64,
    settle_since: Option<f64>,
    on_final_line: bool,
    hold_point: Option<(Vec2, f64)>,
    initial_yaw: f64,
    last_saturated: bool,
    altitude_command: Option<f64>,
    flare_descending: bool,
    pending_mooring: Option<MooringCommand>,
    pub preflight: Vec<PreflightCheck>,
    pub terminal_reason: Option<String>,
}

impl Autopilot {
    pub fn new(
        cfg: &AutopilotConfig,
        controller_cfg: &ControllerConfig,
        vehicle: &AirshipParams,
        env: &EnvironmentConfig,
        path: &FlightPath,
    ) -> Self {
        let derived = DerivedProperties::compute(vehicle, env.ground_density());
        let fr = &cfg.control_limits;
        let limits = ForceLimits {
            surge_min_n: -derived.max_reverse_thrust_n * fr.surge,
            surge_max_n: derived.max_forward_thrust_n * fr.surge,
            heave_min_n: -derived.max_heave_down_n * fr.heave,
            heave_max_n: derived.max_heave_up_n * fr.heave,
            yaw_max_nm: (derived.max_differential_yaw_moment_nm
                + derived.rudder_moment_per_pa
                    * 0.5
                    * env.ground_density()
                    * vehicle.limits.max_airspeed_m_s.powi(2))
                * fr.yaw,
        };
        let controller = build_controller(controller_cfg, vehicle, &derived, limits);
        let approach =
            ApproachGeometry::from_config(env.landing_site, env.landing_area.heading_deg, cfg);
        let path_report = validate_flight_path(path, env.takeoff_site, vehicle, cfg);
        let start = env.initial_state.position;
        Self {
            cfg: cfg.clone(),
            vehicle: vehicle.clone(),
            derived,
            follower: PathFollower::new(path.waypoints.clone(), start, cfg.target_altitude_m),
            path: path.clone(),
            path_report,
            takeoff_site: env.takeoff_site,
            landing_site: env.landing_site,
            approach,
            controller,
            limits,
            phase: FlightPhase::Preflight,
            phase_since: 0.0,
            transitions: Vec::new(),
            safety: SafetyMonitor::new(),
            wind_x: LowPass::new(cfg.guidance.wind_filter_tau_s),
            wind_y: LowPass::new(cfg.guidance.wind_filter_tau_s),
            attempts: 0,
            bounces: 0,
            prep_heave_n: 0.0,
            settle_since: None,
            on_final_line: false,
            hold_point: None,
            initial_yaw: heading_deg_to_yaw(env.initial_state.heading_deg),
            last_saturated: false,
            altitude_command: None,
            flare_descending: false,
            pending_mooring: None,
            preflight: Vec::new(),
            terminal_reason: None,
        }
    }

    pub fn phase(&self) -> FlightPhase {
        self.phase
    }

    pub fn landing_attempts(&self) -> u32 {
        self.attempts
    }

    pub fn bounces(&self) -> u32 {
        self.bounces
    }

    pub fn path_report(&self) -> &ValidationReport {
        &self.path_report
    }

    pub fn flight_path(&self) -> &FlightPath {
        &self.path
    }

    pub fn force_limits(&self) -> ForceLimits {
        self.limits
    }

    /// Estimated static heaviness `W - B` from nominal mass (plus bias) and measured density (N).
    fn heave_trim(&self, m: &Measurement) -> f64 {
        let rho = if m.air_density > 0.0 {
            m.air_density
        } else {
            self.derived.reference_density_kg_m3
        };
        (self.vehicle.total_mass_kg + self.cfg.trim_mass_bias_kg) * G0
            - rho * self.vehicle.displaced_volume() * G0
    }

    fn transition(&mut self, m: &Measurement, to: FlightPhase, reason: String) -> PhaseTransition {
        let dist = (m.position.horizontal() - self.landing_site).norm();
        let tr = PhaseTransition {
            time_s: m.time_s,
            from: self.phase,
            to,
            reason,
            state: StateSnapshot::from_measurement(m, dist),
        };
        self.transitions.push(tr.clone());
        self.phase = to;
        self.phase_since = m.time_s;
        let p = m.position.horizontal();
        match to {
            FlightPhase::Takeoff => self.pending_mooring = Some(MooringCommand::Release),
            FlightPhase::Climb | FlightPhase::Cruise => {
                self.follower.anchor_altitude(p, m.altitude_agl_m)
            }
            FlightPhase::Approach => {
                self.attempts += 1;
                self.on_final_line = false;
            }
            FlightPhase::PostLanding => self.settle_since = None,
            FlightPhase::Flare => self.flare_descending = false,
            FlightPhase::Landed => {
                self.pending_mooring = Some(MooringCommand::Engage);
                self.terminal_reason = Some(tr.reason.clone());
            }
            FlightPhase::Aborted => {
                self.hold_point = Some((
                    p,
                    m.altitude_agl_m.max(self.cfg.safety.min_enroute_altitude_m),
                ));
                self.terminal_reason = Some(tr.reason.clone());
            }
            _ => {}
        }
        tr
    }

    /// Heading to adopt while station keeping: into a wind above 1 m/s; in calm air the
    /// take-off heading during take-off, otherwise none (the current heading is kept,
    /// which lets the wind triangle point the nose at a hold position).
    fn preferred_yaw(&self, wind: Vec2, takeoff: bool) -> Option<f64> {
        if wind.norm() > 1.0 {
            Some((-wind).angle())
        } else if takeoff {
            Some(
                self.cfg
                    .takeoff_heading_deg
                    .map(heading_deg_to_yaw)
                    .unwrap_or(self.initial_yaw),
            )
        } else {
            None
        }
    }

    /// Evaluate the exit conditions of the current phase.
    fn next_phase(&mut self, m: &Measurement, wind: Vec2) -> Option<(FlightPhase, String)> {
        let t = m.time_s;
        let in_phase = t - self.phase_since;
        let p = m.position.horizontal();
        let h = m.altitude_agl_m;
        let vz = m.velocity.z;
        let gs = m.velocity.horizontal().norm();
        let w = wind.norm();
        let dist = (p - self.landing_site).norm();
        let cfg = &self.cfg;
        let land = &cfg.landing;

        if m.ground_contact
            && matches!(
                self.phase,
                FlightPhase::Climb
                    | FlightPhase::Cruise
                    | FlightPhase::Approach
                    | FlightPhase::GoAround
            )
        {
            return Some((
                FlightPhase::Touchdown,
                format!(
                    "unplanned ground contact during {} at {:.1} m from the landing point",
                    self.phase.label(),
                    dist
                ),
            ));
        }

        match self.phase {
            FlightPhase::Preflight => {
                self.preflight =
                    run_preflight(cfg, &self.vehicle, &self.derived, &self.path_report, m, w);
                let failed: Vec<String> = self
                    .preflight
                    .iter()
                    .filter(|c| !c.passed)
                    .map(|c| format!("{} ({})", c.name, c.detail))
                    .collect();
                if !failed.is_empty() {
                    return Some((
                        FlightPhase::Aborted,
                        format!("preflight failed: {}", failed.join("; ")),
                    ));
                }
                let n = self.preflight.len();
                if m.moored {
                    Some((
                        FlightPhase::TakeoffPreparation,
                        format!("all {n} preflight checks passed; vehicle moored on the ground"),
                    ))
                } else if !self.follower.is_complete() {
                    Some((
                        FlightPhase::Cruise,
                        format!(
                            "all {n} preflight checks passed; airborne start at {h:.1} m, joining the flight path"
                        ),
                    ))
                } else {
                    Some((
                        FlightPhase::Approach,
                        format!(
                            "all {n} preflight checks passed; airborne start at {h:.1} m with no waypoints, starting the approach"
                        ),
                    ))
                }
            }
            FlightPhase::TakeoffPreparation => {
                let tk = &cfg.takeoff;
                // With wind, the weathervaning hull must be aligned with it before release.
                let misalignment = if w > 2.0 {
                    wrap_pi(m.yaw - (-wind).angle()).to_degrees().abs()
                } else {
                    0.0
                };
                if m.mooring_load_n >= tk.release_load_threshold_n
                    && w <= tk.max_wind_m_s
                    && misalignment <= 20.0
                {
                    Some((
                        FlightPhase::Takeoff,
                        format!(
                            "mooring load {:.1} N >= release threshold {:.1} N with heave thrust {:.1} N; wind {:.1} m/s <= {:.1} m/s, nose {:.0} deg off the wind; mooring released",
                            m.mooring_load_n,
                            tk.release_load_threshold_n,
                            self.prep_heave_n,
                            w,
                            tk.max_wind_m_s,
                            misalignment
                        ),
                    ))
                } else if in_phase > tk.preparation_timeout_s {
                    let mut why = Vec::new();
                    if w > tk.max_wind_m_s {
                        why.push(format!(
                            "wind {w:.1} m/s above the take-off limit {:.1} m/s",
                            tk.max_wind_m_s
                        ));
                    }
                    if misalignment > 20.0 {
                        why.push(format!(
                            "hull {misalignment:.0} deg off the wind (limit 20 deg)"
                        ));
                    }
                    if m.mooring_load_n < tk.release_load_threshold_n {
                        why.push(format!(
                            "insufficient lift: mooring load {:.1} N below {:.1} N at heave thrust {:.1} N",
                            m.mooring_load_n, tk.release_load_threshold_n, self.prep_heave_n
                        ));
                    }
                    Some((
                        FlightPhase::Aborted,
                        format!(
                            "take-off preparation timed out after {in_phase:.0} s: {}",
                            why.join("; ")
                        ),
                    ))
                } else {
                    None
                }
            }
            FlightPhase::Takeoff => {
                let tk = &cfg.takeoff;
                if h >= tk.clearance_altitude_m && vz > 0.1 {
                    Some((
                        FlightPhase::InitialClimb,
                        format!(
                            "gear height {h:.1} m >= clearance {:.1} m, climbing at {vz:.2} m/s",
                            tk.clearance_altitude_m
                        ),
                    ))
                } else if in_phase > tk.takeoff_timeout_s {
                    Some((
                        FlightPhase::Aborted,
                        format!(
                            "take-off not achieved: gear height {h:.1} m below {:.1} m after {in_phase:.0} s",
                            tk.clearance_altitude_m
                        ),
                    ))
                } else {
                    None
                }
            }
            FlightPhase::InitialClimb => (h >= cfg.takeoff.initial_climb_altitude_m).then(|| {
                (
                    FlightPhase::Climb,
                    format!(
                        "gear height {h:.1} m >= initial-climb height {:.1} m",
                        cfg.takeoff.initial_climb_altitude_m
                    ),
                )
            }),
            FlightPhase::Climb => {
                let tol = &cfg.transitions;
                let err = h - cfg.target_altitude_m;
                if err.abs() <= tol.altitude_m && vz.abs() <= tol.vertical_speed_m_s {
                    let next = if self.follower.is_complete() {
                        FlightPhase::Approach
                    } else {
                        FlightPhase::Cruise
                    };
                    Some((
                        next,
                        format!(
                            "target altitude {:.1} m captured (error {err:+.1} m <= {:.1} m, vz {vz:+.2} m/s); {} waypoint(s) remaining",
                            cfg.target_altitude_m,
                            tol.altitude_m,
                            self.follower.remaining()
                        ),
                    ))
                } else {
                    None
                }
            }
            FlightPhase::Cruise => self.follower.is_complete().then(|| {
                (
                    FlightPhase::Approach,
                    format!("all {} waypoints captured", self.path.waypoints.len()),
                )
            }),
            FlightPhase::Approach => {
                if !self.on_final_line {
                    return None;
                }
                let a = &cfg.approach;
                let g = &self.approach;
                let d = g.distance_to_go(p);
                let cross = g.cross_track(p);
                let course_err = wrap_pi(m.yaw - g.course_yaw).to_degrees();
                let track_err = if gs > 0.5 {
                    wrap_pi(m.velocity.horizontal().angle() - g.course_yaw).to_degrees()
                } else {
                    course_err
                };
                let checks = [
                    (
                        cross.abs() <= a.corridor_half_width_m,
                        format!(
                            "cross-track {cross:+.1} m (limit {:.1})",
                            a.corridor_half_width_m
                        ),
                    ),
                    (
                        (h - a.altitude_m).abs() <= a.corridor_altitude_tolerance_m,
                        format!(
                            "altitude error {:+.1} m (limit {:.1})",
                            h - a.altitude_m,
                            a.corridor_altitude_tolerance_m
                        ),
                    ),
                    (
                        track_err.abs() <= a.max_course_error_deg,
                        format!(
                            "track error {track_err:+.0} deg (limit {:.0})",
                            a.max_course_error_deg
                        ),
                    ),
                    (
                        gs <= a.speed_m_s * 1.6 + 0.5,
                        format!(
                            "ground speed {gs:.1} m/s (limit {:.1})",
                            a.speed_m_s * 1.6 + 0.5
                        ),
                    ),
                    (
                        w <= land.max_wind_m_s,
                        format!("wind {w:.1} m/s (limit {:.1})", land.max_wind_m_s),
                    ),
                ];
                let established = checks.iter().all(|c| c.0);
                if d <= g.gate_distance + 5.0 && established {
                    let detail: Vec<_> = checks.iter().map(|c| c.1.clone()).collect();
                    Some((
                        FlightPhase::Descent,
                        format!(
                            "established at the gate ({:.0} m to go): {}",
                            d,
                            detail.join(", ")
                        ),
                    ))
                } else if d < g.gate_distance - 20.0 {
                    let failed: Vec<_> = checks
                        .iter()
                        .filter(|c| !c.0)
                        .map(|c| c.1.clone())
                        .collect();
                    Some((
                        FlightPhase::GoAround,
                        format!(
                            "not established on final at the gate: {}",
                            failed.join(", ")
                        ),
                    ))
                } else {
                    None
                }
            }
            FlightPhase::Descent => {
                let g = &self.approach;
                let d = g.distance_to_go(p);
                let cross = g.cross_track(p);
                let glide = g.glide_altitude(d);
                if m.ground_contact {
                    return Some((
                        FlightPhase::Touchdown,
                        format!(
                            "ground contact during descent at {dist:.1} m from the landing point"
                        ),
                    ));
                }
                if h > land.decision_height_m {
                    if cross.abs() > land.max_cross_track_error_m {
                        return Some((
                            FlightPhase::GoAround,
                            format!(
                                "cross-track error {cross:+.1} m exceeds {:.1} m",
                                land.max_cross_track_error_m
                            ),
                        ));
                    }
                    if (h - glide).abs() > land.max_altitude_error_m {
                        return Some((
                            FlightPhase::GoAround,
                            format!(
                                "glide-path deviation {:+.1} m exceeds {:.1} m",
                                h - glide,
                                land.max_altitude_error_m
                            ),
                        ));
                    }
                    if w > land.max_wind_m_s {
                        return Some((
                            FlightPhase::GoAround,
                            format!(
                                "wind {w:.1} m/s exceeds the landing limit {:.1} m/s: approach infeasible",
                                land.max_wind_m_s
                            ),
                        ));
                    }
                    if d < -2.0 * land.flare_entry_radius_m {
                        return Some((
                            FlightPhase::GoAround,
                            format!(
                                "overshoot: {:.1} m past the landing point without flare conditions",
                                -d
                            ),
                        ));
                    }
                }
                if h <= land.flare_height_m + 0.75
                    && dist <= land.flare_entry_radius_m
                    && gs <= land.flare_entry_max_speed_m_s
                {
                    Some((
                        FlightPhase::Flare,
                        format!(
                            "flare conditions met: gear height {h:.1} m, {dist:.1} m from the point (<= {:.1}), ground speed {gs:.2} m/s (<= {:.1})",
                            land.flare_entry_radius_m, land.flare_entry_max_speed_m_s
                        ),
                    ))
                } else {
                    None
                }
            }
            FlightPhase::Flare => {
                if m.ground_contact {
                    return Some((
                        FlightPhase::Touchdown,
                        format!(
                            "weight on gear: contact at {dist:.1} m from the landing point, vz {vz:+.2} m/s, ground speed {gs:.2} m/s"
                        ),
                    ));
                }
                if h > land.decision_height_m {
                    if dist > 2.0 * land.landing_point_tolerance_m {
                        return Some((
                            FlightPhase::GoAround,
                            format!(
                                "drifted {dist:.1} m from the landing point (limit {:.1} m) above the decision height",
                                2.0 * land.landing_point_tolerance_m
                            ),
                        ));
                    }
                    if w > land.max_wind_m_s {
                        return Some((
                            FlightPhase::GoAround,
                            format!(
                                "wind {w:.1} m/s exceeds the landing limit {:.1} m/s above the decision height",
                                land.max_wind_m_s
                            ),
                        ));
                    }
                }
                None
            }
            FlightPhase::Touchdown => {
                if !m.ground_contact && h > 0.5 {
                    self.bounces += 1;
                    return Some((
                        FlightPhase::Flare,
                        format!("bounce: contact lost, gear height {h:.2} m"),
                    ));
                }
                (m.ground_contact && gs <= 2.0 * land.settle_speed_m_s && vz.abs() <= 0.2).then(
                    || {
                        (
                            FlightPhase::PostLanding,
                            format!("on the ground, ground speed {gs:.2} m/s, vz {vz:+.2} m/s"),
                        )
                    },
                )
            }
            FlightPhase::PostLanding => {
                if !m.ground_contact && h > 0.5 {
                    self.bounces += 1;
                    return Some((
                        FlightPhase::Flare,
                        format!("lift-off after touchdown: gear height {h:.2} m"),
                    ));
                }
                if m.ground_contact && gs <= land.settle_speed_m_s && vz.abs() <= 0.1 {
                    let since = *self.settle_since.get_or_insert(t);
                    if t - since >= land.settle_time_s {
                        return Some((
                            FlightPhase::Landed,
                            format!(
                                "settled for {:.1} s (ground speed {gs:.2} m/s <= {:.2}); mooring engaged",
                                t - since,
                                land.settle_speed_m_s
                            ),
                        ));
                    }
                } else {
                    self.settle_since = None;
                }
                None
            }
            FlightPhase::GoAround => {
                let target = cfg.approach.altitude_m;
                if h >= target - cfg.transitions.altitude_m {
                    if self.attempts < land.max_attempts {
                        Some((
                            FlightPhase::Approach,
                            format!(
                                "go-around altitude {h:.1} m reached; starting approach {} of {}",
                                self.attempts + 1,
                                land.max_attempts
                            ),
                        ))
                    } else {
                        Some((
                            FlightPhase::Aborted,
                            format!(
                                "maximum number of landing attempts ({}) reached",
                                land.max_attempts
                            ),
                        ))
                    }
                } else {
                    None
                }
            }
            FlightPhase::Landed | FlightPhase::Aborted => None,
        }
    }

    fn plan(&mut self, m: &Measurement, wind: Vec2, trim: f64) -> Plan {
        let cfg = &self.cfg;
        let lim = &self.vehicle.limits;
        let p = m.position.horizontal();
        let h = m.altitude_agl_m;
        let dist = (p - self.landing_site).norm();
        let g = self.approach;
        let mut guidance = GuidanceStatus {
            distance_to_landing_m: dist,
            distance_to_go_m: g.distance_to_go(p),
            landing_attempt: self.attempts,
            heave_trim_n: trim,
            ..Default::default()
        };
        let vertical = |alt: Option<f64>, ff: f64| VerticalReference {
            altitude_agl: alt,
            vertical_speed_ff: ff,
            vertical_speed_min: -REFERENCE_RATE_MARGIN * lim.max_descent_rate_m_s,
            vertical_speed_max: REFERENCE_RATE_MARGIN * lim.max_climb_rate_m_s,
        };
        let hold = |pos: Vec2, max_speed: f64, yaw: Option<f64>| HorizontalReference {
            ground_velocity_ff: Vec2::ZERO,
            hold_position: Some(pos),
            max_ground_speed: max_speed,
            preferred_yaw: yaw,
            yaw_override: None,
        };
        let track = |v: Vec2, max_speed: f64| HorizontalReference {
            ground_velocity_ff: v,
            hold_position: None,
            max_ground_speed: max_speed,
            preferred_yaw: None,
            yaw_override: None,
        };
        let pref = self.preferred_yaw(wind, false);
        let pref_takeoff = self.preferred_yaw(wind, true);
        let hold_speed = cfg.guidance.hold_max_speed_m_s;
        let r_max = lim.max_turn_rate_deg_s.to_radians();

        let mut heave_override = None;
        let mut horizontal_active = true;
        let mut surge_enabled = true;
        let (vref, href) = match self.phase {
            FlightPhase::Preflight | FlightPhase::Landed => {
                guidance.mode = if self.phase == FlightPhase::Landed {
                    "moored".into()
                } else {
                    "preflight".into()
                };
                heave_override = Some(0.0);
                horizontal_active = false;
                (vertical(None, 0.0), hold(p, 0.0, None))
            }
            FlightPhase::TakeoffPreparation => {
                heave_override = Some(self.prep_heave_n);
                if wind.norm() > 2.0 {
                    // Yaw the mast-moored hull into the wind with differential thrust; a hull
                    // exactly downwind sits at the unstable weathervane equilibrium.
                    guidance.mode = "moored: heave thrust ramp, aligning with the wind".into();
                    surge_enabled = false;
                    let yaw = (-wind).angle();
                    (
                        vertical(None, 0.0),
                        HorizontalReference {
                            yaw_override: Some(yaw),
                            ..hold(p, 0.0, None)
                        },
                    )
                } else {
                    guidance.mode = "moored: heave thrust ramp".into();
                    horizontal_active = false;
                    (vertical(None, 0.0), hold(p, 0.0, None))
                }
            }
            FlightPhase::Takeoff | FlightPhase::InitialClimb => {
                guidance.mode = "vertical take-off: position hold over the site".into();
                horizontal_active = !m.ground_contact;
                guidance.cross_track_error_m = Some((p - self.takeoff_site).norm());
                (
                    vertical(None, cfg.takeoff_climb_rate_m_s),
                    hold(self.takeoff_site, hold_speed, pref_takeoff),
                )
            }
            FlightPhase::Climb => {
                let alt = cfg.target_altitude_m;
                guidance.altitude_ref_m = Some(alt);
                let href = match self.follower.update(p, h, m.yaw, r_max, cfg) {
                    Some(c) => {
                        guidance.mode = "climb: path following".into();
                        guidance.cross_track_error_m = Some(c.cross_track_error);
                        guidance.active_waypoint = Some(c.waypoint_index);
                        HorizontalReference {
                            ground_velocity_ff: c.ground_velocity_ff,
                            hold_position: c.hold_position,
                            max_ground_speed: c.max_ground_speed,
                            preferred_yaw: pref,
                            yaw_override: c.yaw_override,
                        }
                    }
                    None => {
                        guidance.mode = "climb: position hold over the site".into();
                        hold(self.takeoff_site, hold_speed, pref_takeoff)
                    }
                };
                (vertical(Some(alt), 0.0), href)
            }
            FlightPhase::Cruise => match self.follower.update(p, h, m.yaw, r_max, cfg) {
                Some(c) => {
                    guidance.mode = "en-route: LOS path following".into();
                    guidance.altitude_ref_m = Some(c.altitude);
                    guidance.cross_track_error_m = Some(c.cross_track_error);
                    guidance.active_waypoint = Some(c.waypoint_index);
                    let href = HorizontalReference {
                        ground_velocity_ff: c.ground_velocity_ff,
                        hold_position: c.hold_position,
                        max_ground_speed: c.max_ground_speed,
                        preferred_yaw: pref,
                        yaw_override: c.yaw_override,
                    };
                    (vertical(Some(c.altitude), c.vertical_speed_ff), href)
                }
                None => {
                    guidance.mode = "en-route: path complete".into();
                    guidance.altitude_ref_m = Some(h.max(cfg.safety.min_enroute_altitude_m));
                    (
                        vertical(guidance.altitude_ref_m, 0.0),
                        hold(p, hold_speed, pref),
                    )
                }
            },
            FlightPhase::Approach => {
                let a = &cfg.approach;
                let join = g.join_point();
                if !self.on_final_line {
                    // Capture the centre line near the join point, or anywhere outside the
                    // gate when already roughly aligned and inside twice the corridor.
                    let s = (p - join).dot(g.course_dir);
                    let cross = g.cross_track(p);
                    let course_err = wrap_pi(m.yaw - g.course_yaw).abs();
                    if (p - join).norm() <= 30.0
                        || (s >= -10.0
                            && g.distance_to_go(p) >= g.gate_distance + 20.0
                            && cross.abs() <= 2.0 * a.corridor_half_width_m
                            && course_err <= 75f64.to_radians())
                    {
                        self.on_final_line = true;
                    }
                }
                guidance.altitude_ref_m = Some(a.altitude_m);
                let lookahead = adaptive_lookahead(cfg.guidance.lookahead_m, a.speed_m_s, r_max);
                let href = if self.on_final_line {
                    guidance.mode = "approach: final-approach line".into();
                    guidance.cross_track_error_m = Some(g.cross_track(p));
                    track(g.line_velocity(p, lookahead, a.speed_m_s), a.speed_m_s)
                } else {
                    guidance.mode = "approach: direct to the join point".into();
                    let speed = a.speed_m_s.max(0.8 * cfg.cruise_speed_m_s);
                    track((join - p).normalized() * speed, speed)
                };
                (vertical(Some(a.altitude_m), 0.0), href)
            }
            FlightPhase::Descent => {
                let a = &cfg.approach;
                let d = g.distance_to_go(p);
                let speed = (a.speed_m_s * d / a.deceleration_distance_m).clamp(0.5, a.speed_m_s);
                let alt = g.glide_altitude(d);
                let vz_ff = if d > 0.0 && alt < g.approach_altitude {
                    -speed * g.glide_tan
                } else {
                    0.0
                };
                guidance.altitude_ref_m = Some(alt);
                guidance.cross_track_error_m = Some(g.cross_track(p));
                let href = if dist < 10.0 {
                    guidance.mode = "descent: converge on the landing point".into();
                    hold(self.landing_site, speed, pref)
                } else {
                    guidance.mode = "descent: glide path".into();
                    // Look-ahead shrinks with the distance to go so that the lateral error
                    // converges before the vehicle reaches the point at low speed.
                    track(
                        g.line_velocity(p, cfg.guidance.lookahead_m.min((0.3 * d).max(5.0)), speed),
                        speed,
                    )
                };
                (vertical(Some(alt), vz_ff), href)
            }
            FlightPhase::Flare => {
                let land = &cfg.landing;
                guidance.cross_track_error_m = Some(dist);
                let href = hold(self.landing_site, 0.6, pref);
                // Start the vertical descent only when centred over the point (with
                // hysteresis); below the decision height the landing is committed.
                let start_radius = 0.4 * land.landing_point_tolerance_m;
                let keep_radius = 0.8 * land.landing_point_tolerance_m;
                self.flare_descending = dist <= start_radius
                    || (self.flare_descending && dist <= keep_radius)
                    || h <= land.decision_height_m;
                if self.flare_descending {
                    // Touchdown-speed gate: pause the sink while the ground speed exceeds
                    // 60 % of the touchdown limit (e.g. during a gust).
                    let gs = m.velocity.horizontal().norm();
                    if gs <= 0.6 * land.max_touchdown_horizontal_speed_m_s {
                        guidance.mode = "flare: final vertical descent".into();
                        let sink = land.touchdown_sink_rate_m_s.max((0.2 * h).min(0.6));
                        (vertical(None, -sink), href)
                    } else {
                        guidance.mode = "flare: descent paused, ground speed too high".into();
                        (vertical(None, 0.0), href)
                    }
                } else {
                    guidance.mode = "flare: holding height until over the point".into();
                    guidance.altitude_ref_m = Some(land.flare_height_m);
                    (vertical(Some(land.flare_height_m), 0.0), href)
                }
            }
            FlightPhase::Touchdown | FlightPhase::PostLanding => {
                guidance.mode = "on ground: hold-down thrust".into();
                heave_override = Some(trim - cfg.landing.hold_down_force_n);
                horizontal_active = false;
                (vertical(None, 0.0), hold(p, 0.0, None))
            }
            FlightPhase::GoAround => {
                let a = &cfg.approach;
                guidance.mode = "go-around: climb straight ahead".into();
                guidance.altitude_ref_m = Some(a.altitude_m);
                (
                    vertical(Some(a.altitude_m), 0.0),
                    track(g.course_dir * a.speed_m_s, a.speed_m_s),
                )
            }
            FlightPhase::Aborted => {
                if m.moored || m.ground_contact {
                    guidance.mode = "aborted: on the ground".into();
                    heave_override = Some(if m.moored {
                        0.0
                    } else {
                        trim - cfg.landing.hold_down_force_n
                    });
                    horizontal_active = false;
                    (vertical(None, 0.0), hold(p, 0.0, None))
                } else {
                    let (hp, ha) = *self
                        .hold_point
                        .get_or_insert((p, h.max(cfg.safety.min_enroute_altitude_m)));
                    guidance.mode = "aborted: holding position".into();
                    guidance.altitude_ref_m = Some(ha);
                    (vertical(Some(ha), 0.0), hold(hp, hold_speed, pref))
                }
            }
        };
        Plan {
            refs: ControlReferences {
                vertical: vref,
                horizontal: href,
                heave_trim_n: trim,
                heave_override_n: heave_override,
                max_airspeed: 0.9 * lim.max_airspeed_m_s,
                max_yaw_rate: lim.max_turn_rate_deg_s.to_radians(),
                horizontal_active,
                surge_enabled,
            },
            guidance,
        }
    }

    /// Rate-limit the altitude reference to `REFERENCE_RATE_MARGIN` times the vehicle's
    /// climb and descent limits so that step changes of the target altitude become a
    /// feasible reference trajectory. While the limiter is active its rate becomes the
    /// vertical-speed feed-forward; otherwise the guidance feed-forward is passed through.
    fn shape_altitude_reference(&mut self, plan: &mut Plan, h: f64, dt: f64) {
        let v = &mut plan.refs.vertical;
        let Some(raw) = v.altitude_agl else {
            self.altitude_command = None;
            return;
        };
        if !(dt > 0.0) {
            return;
        }
        let lim = &self.vehicle.limits;
        let cmd = self.altitude_command.get_or_insert(h);
        let delta = raw - *cmd;
        let step = delta.clamp(
            -REFERENCE_RATE_MARGIN * lim.max_descent_rate_m_s * dt,
            REFERENCE_RATE_MARGIN * lim.max_climb_rate_m_s * dt,
        );
        *cmd += step;
        if (delta - step).abs() > 1e-9 {
            v.vertical_speed_ff = step / dt;
        }
        v.altitude_agl = Some(*cmd);
        plan.guidance.altitude_ref_m = Some(*cmd);
    }

    /// Run one control step on measurement `m` with control period `dt`.
    pub fn step(&mut self, m: &Measurement, dt: f64) -> AutopilotOutput {
        let wind = Vec2::new(
            self.wind_x.update(m.wind_estimate.x, dt),
            self.wind_y.update(m.wind_estimate.y, dt),
        );
        let p = m.position.horizontal();
        let dist = (p - self.landing_site).norm();
        let trim = self.heave_trim(m);

        // 1. Safety monitoring and responses.
        let new_events = self.safety.evaluate(&SafetyInputs {
            meas: m,
            phase: self.phase,
            saturated: self.last_saturated,
            distance_to_landing_m: dist,
            decision_height_m: self.cfg.landing.decision_height_m,
            limits: &self.vehicle.limits,
            cfg: &self.cfg.safety,
        });
        let mut new_transition = None;
        match SafetyMonitor::strongest_response(&new_events) {
            SafetyResponse::GoAround if self.phase.is_landing_sequence() => {
                let e = new_events
                    .iter()
                    .find(|e| e.response == SafetyResponse::GoAround)
                    .expect("event exists");
                new_transition = Some(self.transition(
                    m,
                    FlightPhase::GoAround,
                    format!("safety response to critical event: {}", e.message),
                ));
            }
            SafetyResponse::ProceedToLanding
                if matches!(self.phase, FlightPhase::Climb | FlightPhase::Cruise) =>
            {
                let e = new_events
                    .iter()
                    .find(|e| e.response == SafetyResponse::ProceedToLanding)
                    .expect("event exists");
                new_transition = Some(self.transition(
                    m,
                    FlightPhase::Approach,
                    format!("safety response, proceeding to landing: {}", e.message),
                ));
            }
            _ => {}
        }

        // 2. Phase logic (one transition per step).
        if new_transition.is_none()
            && let Some((to, reason)) = self.next_phase(m, wind)
        {
            if to == FlightPhase::GoAround && self.phase.is_landing_sequence() {
                self.safety.push(SafetyEvent {
                    time_s: m.time_s,
                    severity: SafetySeverity::Warning,
                    constraint: SafetyConstraint::ApproachFeasibility,
                    message: format!("go-around initiated: {reason}"),
                    phase: self.phase,
                    value: 0.0,
                    limit: 0.0,
                    response: SafetyResponse::GoAround,
                    state: StateSnapshot::from_measurement(m, dist),
                    cleared_at_s: Some(m.time_s),
                });
            }
            if to == FlightPhase::Touchdown && reason.starts_with("unplanned") {
                self.safety.push(SafetyEvent {
                    time_s: m.time_s,
                    severity: SafetySeverity::Critical,
                    constraint: SafetyConstraint::UnplannedGroundContact,
                    message: reason.clone(),
                    phase: self.phase,
                    value: m.altitude_agl_m,
                    limit: 0.0,
                    response: SafetyResponse::LogOnly,
                    state: StateSnapshot::from_measurement(m, dist),
                    cleared_at_s: None,
                });
            }
            new_transition = Some(self.transition(m, to, reason));
        }

        // Open-loop heave ramp while moored.
        if self.phase == FlightPhase::TakeoffPreparation {
            let target = (trim + self.cfg.takeoff.lift_margin_n).min(self.limits.heave_max_n);
            let max = self.limits.heave_max_n;
            let ramp = self.cfg.takeoff.thrust_ramp_n_s * dt;
            // Ramp to the estimated requirement, then keep increasing slowly up to the
            // limit until the measured mooring load confirms positive lift; hold the
            // thrust once it does (e.g. while waiting for wind alignment).
            let rate = if self.prep_heave_n < target {
                ramp
            } else if m.mooring_load_n < self.cfg.takeoff.release_load_threshold_n {
                0.25 * ramp
            } else {
                0.0
            };
            self.prep_heave_n = (self.prep_heave_n + rate).min(max).max(trim.min(0.0));
        }

        // 3. References and control.
        let mut plan = self.plan(m, wind, trim);
        self.shape_altitude_reference(&mut plan, m.altitude_agl_m, dt);
        let input = ControlInput {
            phase: self.phase.class(),
            meas: ControlMeasurement {
                position: p,
                altitude_agl: m.altitude_agl_m,
                vertical_speed: m.velocity.z,
                ground_velocity: m.velocity.horizontal(),
                airspeed_forward: m.air_velocity_body.x,
                airspeed: m.air_velocity_body.norm(),
                yaw: m.yaw,
                yaw_rate: m.yaw_rate,
                wind,
                air_density: m.air_density,
            },
            refs: plan.refs,
            distance_to_landing_m: dist,
        };
        let control = self.controller.compute(&input, dt);
        let a = &control.allocation;
        self.last_saturated = control.saturation.any();
        let mut command = a.command;
        if self.phase == FlightPhase::Landed || (self.phase == FlightPhase::Aborted && m.moored) {
            command = ActuatorCommand::default();
        }
        AutopilotOutput {
            command,
            mooring: self.pending_mooring.take(),
            phase: self.phase,
            guidance: plan.guidance,
            control,
            new_events,
            new_transition,
        }
    }

    /// Current heading reference helper for reports (compass deg).
    pub fn approach_course_deg(&self) -> f64 {
        yaw_to_heading_deg(self.approach.course_yaw)
    }
}
