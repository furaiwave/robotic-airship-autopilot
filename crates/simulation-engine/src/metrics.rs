//! Performance metrics computed from recorded telemetry.
//!
//! Notation: samples `k = 1..N` at times `t_k` with spacing `dt_k = t_{k+1} - t_k`
//! (the telemetry period). For an error signal `e` over a sample window `W`:
//!
//! ```text
//! RMS(e)  = sqrt( sum_{k in W} e_k^2 dt_k / sum_{k in W} dt_k )
//! MAX(e)  = max_{k in W} |e_k|
//! INT(x)  = sum_k x_k dt_k
//! ```
//!
//! Each metric carries its definition and unit (see [`metric_definitions`]).

use airship_model::{DerivedProperties, ValidationIssue};
use autopilot::{FlightPhase, PhaseTransition, PreflightCheck, SafetyEvent, SafetySeverity};
use control_algorithms::ControllerKind;
use flight_dynamics::environment::DisturbanceActivation;
use serde::{Deserialize, Serialize};

use crate::DISCLAIMER;
use crate::config::{SimulationConfig, config_hash};
use crate::engine::{FinishReason, Simulation};
use crate::landing::{AssessmentInput, LandingAssessment, TouchdownRecord, assess};
use crate::telemetry::TelemetrySample;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Metric {
    pub key: String,
    pub label: String,
    pub group: String,
    pub unit: String,
    pub value: Option<f64>,
    pub definition: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhaseInterval {
    pub phase: FlightPhase,
    pub start_s: f64,
    pub end_s: f64,
    pub duration_s: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NumericalInfo {
    pub integrator: String,
    pub dt_s: f64,
    pub control_rate_hz: f64,
    pub telemetry_rate_hz: f64,
    pub steps: u64,
    pub simulated_time_s: f64,
    pub wall_time_ms: f64,
    pub real_time_factor: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResultsReport {
    pub name: String,
    pub controller: ControllerKind,
    pub controller_label: String,
    pub config_hash: String,
    pub turbulence_seed: Option<u64>,
    pub sensor_noise_seed: Option<u64>,
    pub finish_reason: Option<FinishReason>,
    pub finish_message: String,
    pub final_phase: FlightPhase,
    pub landing: LandingAssessment,
    pub metrics: Vec<Metric>,
    pub phase_timeline: Vec<PhaseInterval>,
    pub transitions: Vec<PhaseTransition>,
    pub safety_events: Vec<SafetyEvent>,
    pub disturbances: Vec<DisturbanceActivation>,
    pub touchdowns: Vec<TouchdownRecord>,
    pub preflight: Vec<PreflightCheck>,
    pub flight_path_warnings: Vec<ValidationIssue>,
    pub numerical: NumericalInfo,
    pub derived: DerivedProperties,
    pub partial: bool,
    pub disclaimer: String,
}

impl ResultsReport {
    pub fn metric(&self, key: &str) -> Option<f64> {
        self.metrics
            .iter()
            .find(|m| m.key == key)
            .and_then(|m| m.value)
    }
}

/// `(key, label, group, unit, definition)` of every reported metric.
pub fn metric_definitions() -> Vec<(
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
)> {
    vec![
        (
            "altitude_rmse_m",
            "Altitude tracking RMS error",
            "Tracking",
            "m",
            "RMS of e_h = h_ref - h over airborne samples with an active altitude reference",
        ),
        (
            "altitude_max_error_m",
            "Altitude tracking max error",
            "Tracking",
            "m",
            "MAX |h_ref - h| over the same window",
        ),
        (
            "cross_track_rmse_m",
            "Position (cross-track) RMS error",
            "Tracking",
            "m",
            "RMS of the signed cross-track error to the active path leg or approach line (climb, cruise, approach line, descent)",
        ),
        (
            "cross_track_max_m",
            "Position (cross-track) max error",
            "Tracking",
            "m",
            "MAX |cross-track error| over the same window",
        ),
        (
            "horizontal_velocity_rmse_m_s",
            "Horizontal velocity RMS error",
            "Tracking",
            "m/s",
            "RMS of |v_ref - v_ground| (2-D ground velocity) over airborne samples",
        ),
        (
            "vertical_velocity_rmse_m_s",
            "Vertical velocity RMS error",
            "Tracking",
            "m/s",
            "RMS of vz_ref - vz over airborne samples",
        ),
        (
            "heading_rmse_deg",
            "Heading RMS error",
            "Tracking",
            "deg",
            "RMS of wrap(psi_ref - psi) over airborne samples",
        ),
        (
            "max_pitch_deg",
            "Maximum pitch deviation",
            "Attitude",
            "deg",
            "MAX |theta| over airborne samples",
        ),
        (
            "max_roll_deg",
            "Maximum roll deviation",
            "Attitude",
            "deg",
            "MAX |phi| over airborne samples",
        ),
        (
            "time_to_target_altitude_s",
            "Time to target altitude",
            "Timing",
            "s",
            "t(first |h - h_target| <= tolerance) - t(mooring release)",
        ),
        (
            "takeoff_duration_s",
            "Take-off duration",
            "Timing",
            "s",
            "t(enter Climb) - t(enter Takeoff): release to initial-climb height",
        ),
        (
            "approach_duration_s",
            "Approach duration",
            "Timing",
            "s",
            "t(touchdown) - t(last Approach entry)",
        ),
        (
            "time_to_touchdown_s",
            "Time to touchdown",
            "Timing",
            "s",
            "t(first contact of the assessed touchdown) - 0",
        ),
        (
            "mission_duration_s",
            "Mission duration",
            "Timing",
            "s",
            "Simulated time at the end of the run",
        ),
        (
            "landing_point_error_m",
            "Landing-point error",
            "Landing",
            "m",
            "Horizontal distance between the touchdown point and the target landing point",
        ),
        (
            "touchdown_horizontal_speed_m_s",
            "Touchdown horizontal speed",
            "Landing",
            "m/s",
            "|v_ground| at contact onset",
        ),
        (
            "touchdown_vertical_speed_m_s",
            "Touchdown vertical speed",
            "Landing",
            "m/s",
            "-vz at contact onset (positive = sinking)",
        ),
        (
            "max_surge_effort",
            "Max surge effort",
            "Control",
            "-",
            "MAX of |F_x| / F_x,max (normalised achieved surge command)",
        ),
        (
            "max_heave_effort",
            "Max heave effort",
            "Control",
            "-",
            "MAX of |F_z| / F_z,max",
        ),
        (
            "max_yaw_effort",
            "Max yaw effort",
            "Control",
            "-",
            "MAX of |N| / N_max",
        ),
        (
            "control_effort_index_s",
            "Control effort index",
            "Control",
            "s",
            "INT(u_surge^2 + u_heave^2 + u_yaw^2) with normalised commands",
        ),
        (
            "saturation_time_s",
            "Control saturation duration",
            "Control",
            "s",
            "INT(1[any channel saturated]) over the run",
        ),
        (
            "saturation_time_heave_s",
            "Heave saturation duration",
            "Control",
            "s",
            "INT(1[heave saturated])",
        ),
        (
            "saturation_time_yaw_s",
            "Yaw saturation duration",
            "Control",
            "s",
            "INT(1[yaw saturated])",
        ),
        (
            "energy_wh",
            "Electrical energy used",
            "Energy",
            "Wh",
            "INT(P) with momentum-theory thruster power plus avionics power",
        ),
        (
            "propulsive_impulse_n_s",
            "Propulsive impulse",
            "Energy",
            "N s",
            "INT(|T_L| + |T_R| + |T_h|)",
        ),
        (
            "final_battery_soc",
            "Final battery state of charge",
            "Energy",
            "-",
            "1 - E_used / E_capacity at the end of the run",
        ),
        (
            "landing_attempts",
            "Landing attempts",
            "Safety",
            "count",
            "Number of Approach phase entries",
        ),
        (
            "go_around_count",
            "Go-arounds",
            "Safety",
            "count",
            "Number of transitions into GoAround",
        ),
        (
            "bounce_count",
            "Bounces",
            "Safety",
            "count",
            "Contact lost after touchdown",
        ),
        (
            "warning_events",
            "Warning events",
            "Safety",
            "count",
            "Safety events with severity warning",
        ),
        (
            "critical_events",
            "Critical events",
            "Safety",
            "count",
            "Safety events with severity critical",
        ),
        (
            "fatal_events",
            "Fatal events",
            "Safety",
            "count",
            "Safety events with severity fatal",
        ),
        (
            "wall_time_ms",
            "Numerical execution time",
            "Numerics",
            "ms",
            "Wall-clock time spent stepping the simulation",
        ),
        (
            "real_time_factor",
            "Real-time factor",
            "Numerics",
            "-",
            "Simulated time / wall-clock time",
        ),
    ]
}

struct Acc {
    sum: f64,
    weight: f64,
    max: f64,
}

impl Acc {
    fn new() -> Self {
        Self {
            sum: 0.0,
            weight: 0.0,
            max: 0.0,
        }
    }
    fn add(&mut self, e: f64, dt: f64) {
        if e.is_finite() {
            self.sum += e * e * dt;
            self.weight += dt;
            self.max = self.max.max(e.abs());
        }
    }
    fn rms(&self) -> Option<f64> {
        (self.weight > 0.0).then(|| (self.sum / self.weight).sqrt())
    }
    fn max(&self) -> Option<f64> {
        (self.weight > 0.0).then_some(self.max)
    }
}

fn entry_time(tr: &[PhaseTransition], phase: FlightPhase) -> Option<f64> {
    tr.iter().find(|t| t.to == phase).map(|t| t.time_s)
}

fn phase_timeline(tr: &[PhaseTransition], end: f64) -> Vec<PhaseInterval> {
    let mut out = Vec::new();
    let mut current = (FlightPhase::Preflight, 0.0);
    for t in tr {
        out.push(PhaseInterval {
            phase: current.0,
            start_s: current.1,
            end_s: t.time_s,
            duration_s: t.time_s - current.1,
        });
        current = (t.to, t.time_s);
    }
    out.push(PhaseInterval {
        phase: current.0,
        start_s: current.1,
        end_s: end,
        duration_s: end - current.1,
    });
    out
}

pub fn compute_report(sim: &Simulation) -> ResultsReport {
    let cfg: &SimulationConfig = &sim.cfg;
    let tel: &[TelemetrySample] = &sim.telemetry;
    let ap = &sim.autopilot;
    let transitions = ap.transitions.clone();
    let events = ap.safety.events.clone();
    let dt_tel = 1.0 / cfg.simulation.telemetry_rate_hz;
    let end_t = sim.time();

    let (mut alt, mut xt, mut vh, mut vz, mut hd) =
        (Acc::new(), Acc::new(), Acc::new(), Acc::new(), Acc::new());
    let (mut pitch, mut roll) = (0.0f64, 0.0f64);
    let mut airborne_samples = 0usize;
    let (mut ms, mut mh, mut my) = (0.0f64, 0.0f64, 0.0f64);
    let (mut effort, mut sat, mut sat_h, mut sat_y, mut impulse) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for s in tel {
        let airborne = s.phase.is_airborne() && !s.contact;
        if airborne {
            airborne_samples += 1;
            if let Some(e) = s.err_altitude_m {
                alt.add(e, dt_tel);
            }
            vh.add(s.err_horizontal_velocity_m_s, dt_tel);
            vz.add(s.err_vz_m_s, dt_tel);
            hd.add(s.err_heading_deg, dt_tel);
            pitch = pitch.max(s.pitch_deg.abs());
            roll = roll.max(s.roll_deg.abs());
        }
        if matches!(
            s.phase,
            FlightPhase::Climb | FlightPhase::Cruise | FlightPhase::Approach | FlightPhase::Descent
        ) && let Some(e) = s.err_cross_track_m
        {
            xt.add(e, dt_tel);
        }
        ms = ms.max(s.u_surge);
        mh = mh.max(s.u_heave);
        my = my.max(s.u_yaw);
        effort += (s.u_surge.powi(2) + s.u_heave.powi(2) + s.u_yaw.powi(2)) * dt_tel;
        if s.sat_surge || s.sat_heave || s.sat_yaw {
            sat += dt_tel;
        }
        if s.sat_heave {
            sat_h += dt_tel;
        }
        if s.sat_yaw {
            sat_y += dt_tel;
        }
        impulse +=
            (s.thrust_left_n.abs() + s.thrust_right_n.abs() + s.thrust_heave_n.abs()) * dt_tel;
    }

    let release = entry_time(&transitions, FlightPhase::Takeoff);
    let tol = cfg.autopilot.transitions.altitude_m;
    let target = cfg.autopilot.target_altitude_m;
    let time_to_alt = release.and_then(|r| {
        tel.iter()
            .find(|s| s.t_s >= r && (s.altitude_agl_m - target).abs() <= tol)
            .map(|s| s.t_s - r)
    });
    let takeoff_duration = match (release, entry_time(&transitions, FlightPhase::Climb)) {
        (Some(a), Some(b)) => Some(b - a),
        _ => None,
    };

    let final_phase = ap.phase();
    let landing = assess(&AssessmentInput {
        landing: &cfg.autopilot.landing,
        area: &cfg.environment.landing_area,
        target: cfg.environment.landing_site,
        touchdowns: &sim.touchdowns,
        transitions: &transitions,
        events: &events,
        final_phase,
        reached_target_area: sim.reached_target_area,
    });
    let td = landing.touchdown.clone();
    let last_approach = transitions
        .iter()
        .rev()
        .find(|t| t.to == FlightPhase::Approach)
        .map(|t| t.time_s);
    let approach_duration = match (&td, last_approach) {
        (Some(t), Some(a)) => Some(t.time_s - a),
        _ => None,
    };
    let go_arounds = transitions
        .iter()
        .filter(|t| t.to == FlightPhase::GoAround)
        .count();
    let wall_ms = sim.wall_time().as_secs_f64() * 1000.0;
    let rtf = if wall_ms > 0.0 {
        end_t / (wall_ms / 1000.0)
    } else {
        0.0
    };
    let count = |s: SafetySeverity| events.iter().filter(|e| e.severity == s).count() as f64;
    let final_soc = tel.last().map(|s| s.battery_soc);
    let energy = sim.energy_wh();

    let values: Vec<(&str, Option<f64>)> = vec![
        ("altitude_rmse_m", alt.rms()),
        ("altitude_max_error_m", alt.max()),
        ("cross_track_rmse_m", xt.rms()),
        ("cross_track_max_m", xt.max()),
        ("horizontal_velocity_rmse_m_s", vh.rms()),
        ("vertical_velocity_rmse_m_s", vz.rms()),
        ("heading_rmse_deg", hd.rms()),
        ("max_pitch_deg", (airborne_samples > 0).then_some(pitch)),
        ("max_roll_deg", (airborne_samples > 0).then_some(roll)),
        ("time_to_target_altitude_s", time_to_alt),
        ("takeoff_duration_s", takeoff_duration),
        ("approach_duration_s", approach_duration),
        ("time_to_touchdown_s", td.as_ref().map(|t| t.time_s)),
        ("mission_duration_s", Some(end_t)),
        (
            "landing_point_error_m",
            td.as_ref().map(|t| t.distance_to_target_m),
        ),
        (
            "touchdown_horizontal_speed_m_s",
            td.as_ref().map(|t| t.horizontal_speed_m_s),
        ),
        (
            "touchdown_vertical_speed_m_s",
            td.as_ref().map(|t| -t.vertical_speed_m_s),
        ),
        ("max_surge_effort", Some(ms)),
        ("max_heave_effort", Some(mh)),
        ("max_yaw_effort", Some(my)),
        ("control_effort_index_s", Some(effort)),
        ("saturation_time_s", Some(sat)),
        ("saturation_time_heave_s", Some(sat_h)),
        ("saturation_time_yaw_s", Some(sat_y)),
        ("energy_wh", Some(energy)),
        ("propulsive_impulse_n_s", Some(impulse)),
        ("final_battery_soc", final_soc),
        ("landing_attempts", Some(ap.landing_attempts() as f64)),
        ("go_around_count", Some(go_arounds as f64)),
        ("bounce_count", Some(ap.bounces() as f64)),
        ("warning_events", Some(count(SafetySeverity::Warning))),
        ("critical_events", Some(count(SafetySeverity::Critical))),
        ("fatal_events", Some(count(SafetySeverity::Fatal))),
        ("wall_time_ms", Some(wall_ms)),
        ("real_time_factor", Some(rtf)),
    ];
    let defs = metric_definitions();
    let metrics = values
        .into_iter()
        .map(|(key, value)| {
            let d = defs
                .iter()
                .find(|d| d.0 == key)
                .expect("metric definition exists");
            Metric {
                key: key.into(),
                label: d.1.into(),
                group: d.2.into(),
                unit: d.3.into(),
                value,
                definition: d.4.into(),
            }
        })
        .collect();

    let env = &cfg.environment;
    ResultsReport {
        name: cfg.name.clone(),
        controller: cfg.controller.kind,
        controller_label: cfg.controller.kind.label().into(),
        config_hash: config_hash(cfg),
        turbulence_seed: env
            .wind
            .turbulence
            .enabled
            .then_some(env.wind.turbulence.seed),
        sensor_noise_seed: env.sensor_noise.enabled.then_some(env.sensor_noise.seed),
        finish_reason: sim.finished(),
        finish_message: sim.finish_message().unwrap_or("running").into(),
        final_phase,
        landing,
        metrics,
        phase_timeline: phase_timeline(&transitions, end_t),
        transitions,
        safety_events: events,
        disturbances: sim.disturbances.clone(),
        touchdowns: sim.touchdowns.clone(),
        preflight: ap.preflight.clone(),
        flight_path_warnings: ap.path_report().warnings().cloned().collect(),
        numerical: NumericalInfo {
            integrator: "classical 4th-order Runge-Kutta, fixed step".into(),
            dt_s: cfg.simulation.dt_s,
            control_rate_hz: cfg.simulation.control_rate_hz,
            telemetry_rate_hz: cfg.simulation.telemetry_rate_hz,
            steps: sim.status().steps,
            simulated_time_s: end_t,
            wall_time_ms: wall_ms,
            real_time_factor: rtf,
        },
        derived: cfg.derived(),
        partial: sim.finished().is_none(),
        disclaimer: DISCLAIMER.into(),
    }
}

impl Simulation {
    /// Compute the results report from the telemetry recorded so far.
    pub fn report(&self) -> ResultsReport {
        compute_report(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rms_of_constant_error() {
        let mut a = Acc::new();
        for _ in 0..10 {
            a.add(-2.0, 0.1);
        }
        assert!((a.rms().unwrap() - 2.0).abs() < 1e-12);
        assert_eq!(a.max(), Some(2.0));
        assert_eq!(Acc::new().rms(), None);
    }

    #[test]
    fn every_metric_has_a_unique_definition() {
        let d = metric_definitions();
        let mut keys: Vec<_> = d.iter().map(|x| x.0).collect();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), d.len());
    }
}
