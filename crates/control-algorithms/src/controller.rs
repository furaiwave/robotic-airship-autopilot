//! Controller configuration, the [`FlightController`] trait and the two implementations.

use airship_model::{AirshipParams, DerivedProperties, ValidationReport};
use serde::{Deserialize, Serialize};

use crate::allocation::ControlAllocator;
use crate::cascade::{
    CascadeCore, CascadeSettings, ControlInput, ControlModel, ControlOutput, ForceLimits, LoopGains,
};
use crate::pid::PidGains;
use crate::schedule::{GainScheduleConfig, OperatingPoint, scheduled_gains};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControllerKind {
    /// Controller A: cascaded PID with fixed gains.
    CascadedPid,
    /// Controller B: cascaded PID with gains from an explicit scheduling law.
    GainScheduled,
}

impl ControllerKind {
    pub fn label(self) -> &'static str {
        match self {
            ControllerKind::CascadedPid => "Cascaded PID (A)",
            ControllerKind::GainScheduled => "Gain-scheduled PID (B)",
        }
    }
}

/// Fixed gains of Controller A.
///
/// The defaults were obtained from the same pole-placement formulas as Controller B,
/// evaluated once at a nominal cruise operating point (airspeed 4 m/s, |w| = 0.5 m/s,
/// cruise bandwidths), and then kept constant.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CascadedPidConfig {
    pub altitude: PidGains,
    pub vertical_speed: PidGains,
    pub heading: PidGains,
    pub yaw_rate: PidGains,
    pub airspeed: PidGains,
    pub position_gain_1_s: f64,
}

impl Default for CascadedPidConfig {
    fn default() -> Self {
        Self {
            altitude: PidGains::new(0.25, 0.006, 0.0),
            vertical_speed: PidGains::new(125.0, 46.0, 0.0),
            heading: PidGains::new(0.4, 0.0, 0.0),
            yaw_rate: PidGains::new(1540.0, 1230.0, 0.0),
            airspeed: PidGains::new(79.0, 27.0, 0.0),
            position_gain_1_s: 0.2,
        }
    }
}

impl CascadedPidConfig {
    pub fn loop_gains(&self) -> LoopGains {
        LoopGains {
            altitude: self.altitude,
            vertical_speed: self.vertical_speed,
            heading: self.heading,
            yaw_rate: self.yaw_rate,
            airspeed: self.airspeed,
            position: self.position_gain_1_s,
        }
    }

    pub fn validate(&self) -> ValidationReport {
        let mut r = ValidationReport::new();
        for (name, g) in [
            ("altitude", self.altitude),
            ("vertical_speed", self.vertical_speed),
            ("heading", self.heading),
            ("yaw_rate", self.yaw_rate),
            ("airspeed", self.airspeed),
        ] {
            if !g.is_valid() {
                r.error(name, "gains must be finite and non-negative");
            }
            if g.kp == 0.0 && g.ki == 0.0 {
                r.warning(name, "loop has neither proportional nor integral action");
            }
        }
        r.range("position_gain_1_s", self.position_gain_1_s, 0.0, 5.0, "1/s");
        r
    }
}

/// Settings shared by both controllers.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CommonControlSettings {
    pub derivative_filter_tau_s: f64,
    pub heave_rate_limit_n_s: f64,
    pub surge_rate_limit_n_s: f64,
    pub yaw_moment_rate_limit_nm_s: f64,
    /// Use the nominal drag model as a feed-forward term.
    pub drag_feedforward: bool,
    pub max_reverse_airspeed_m_s: f64,
}

impl Default for CommonControlSettings {
    fn default() -> Self {
        Self {
            derivative_filter_tau_s: 0.2,
            heave_rate_limit_n_s: 400.0,
            surge_rate_limit_n_s: 400.0,
            yaw_moment_rate_limit_nm_s: 1000.0,
            drag_feedforward: true,
            max_reverse_airspeed_m_s: 1.5,
        }
    }
}

impl CommonControlSettings {
    fn cascade(&self) -> CascadeSettings {
        CascadeSettings {
            derivative_filter_tau_s: self.derivative_filter_tau_s,
            heave_rate_limit_n_s: self.heave_rate_limit_n_s,
            surge_rate_limit_n_s: self.surge_rate_limit_n_s,
            yaw_moment_rate_limit_nm_s: self.yaw_moment_rate_limit_nm_s,
            drag_feedforward: self.drag_feedforward,
            max_reverse_airspeed_m_s: self.max_reverse_airspeed_m_s,
        }
    }

    pub fn validate(&self) -> ValidationReport {
        let mut r = ValidationReport::new();
        r.range(
            "derivative_filter_tau_s",
            self.derivative_filter_tau_s,
            0.0,
            5.0,
            "s",
        );
        r.range(
            "heave_rate_limit_n_s",
            self.heave_rate_limit_n_s,
            1.0,
            1.0e7,
            "N/s",
        );
        r.range(
            "surge_rate_limit_n_s",
            self.surge_rate_limit_n_s,
            1.0,
            1.0e7,
            "N/s",
        );
        r.range(
            "yaw_moment_rate_limit_nm_s",
            self.yaw_moment_rate_limit_nm_s,
            1.0,
            1.0e8,
            "N m/s",
        );
        r.range(
            "max_reverse_airspeed_m_s",
            self.max_reverse_airspeed_m_s,
            0.0,
            5.0,
            "m/s",
        );
        r
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControllerConfig {
    pub kind: ControllerKind,
    pub common: CommonControlSettings,
    pub cascaded_pid: CascadedPidConfig,
    pub gain_scheduled: GainScheduleConfig,
}

impl Default for ControllerConfig {
    fn default() -> Self {
        Self {
            kind: ControllerKind::CascadedPid,
            common: CommonControlSettings::default(),
            cascaded_pid: CascadedPidConfig::default(),
            gain_scheduled: GainScheduleConfig::default(),
        }
    }
}

impl ControllerConfig {
    pub fn validate(&self) -> ValidationReport {
        let mut r = ValidationReport::new();
        r.merge("common", self.common.validate());
        r.merge("cascaded_pid", self.cascaded_pid.validate());
        r.merge("gain_scheduled", self.gain_scheduled.validate());
        r
    }
}

/// A flight controller turns references and measurements into actuator commands.
pub trait FlightController: Send {
    fn kind(&self) -> ControllerKind;
    fn compute(&mut self, input: &ControlInput, dt: f64) -> ControlOutput;
    fn reset(&mut self);
}

/// Shared plumbing of both controllers.
#[derive(Debug, Clone)]
struct Plant {
    model: ControlModel,
    allocator: ControlAllocator,
    limits: ForceLimits,
    settings: CascadeSettings,
}

impl Plant {
    fn new(
        p: &AirshipParams,
        d: &DerivedProperties,
        limits: ForceLimits,
        common: &CommonControlSettings,
    ) -> Self {
        Self {
            model: ControlModel::from_vehicle(p, d),
            allocator: ControlAllocator::new(p),
            limits,
            settings: common.cascade(),
        }
    }
}

/// Controller A.
#[derive(Debug, Clone)]
pub struct CascadedPidController {
    plant: Plant,
    gains: LoopGains,
    core: CascadeCore,
}

impl CascadedPidController {
    pub fn new(
        p: &AirshipParams,
        d: &DerivedProperties,
        limits: ForceLimits,
        cfg: &ControllerConfig,
    ) -> Self {
        Self {
            plant: Plant::new(p, d, limits, &cfg.common),
            gains: cfg.cascaded_pid.loop_gains(),
            core: CascadeCore::new(),
        }
    }
}

impl FlightController for CascadedPidController {
    fn kind(&self) -> ControllerKind {
        ControllerKind::CascadedPid
    }

    fn compute(&mut self, input: &ControlInput, dt: f64) -> ControlOutput {
        let pl = &self.plant;
        self.core.compute(
            input,
            &self.gains,
            &pl.limits,
            &pl.settings,
            &pl.model,
            &pl.allocator,
            dt,
        )
    }

    fn reset(&mut self) {
        self.core.reset();
    }
}

/// Controller B.
#[derive(Debug, Clone)]
pub struct GainScheduledController {
    plant: Plant,
    schedule: GainScheduleConfig,
    core: CascadeCore,
}

impl GainScheduledController {
    pub fn new(
        p: &AirshipParams,
        d: &DerivedProperties,
        limits: ForceLimits,
        cfg: &ControllerConfig,
    ) -> Self {
        Self {
            plant: Plant::new(p, d, limits, &cfg.common),
            schedule: cfg.gain_scheduled,
            core: CascadeCore::new(),
        }
    }
}

impl FlightController for GainScheduledController {
    fn kind(&self) -> ControllerKind {
        ControllerKind::GainScheduled
    }

    fn compute(&mut self, input: &ControlInput, dt: f64) -> ControlOutput {
        let m = &input.meas;
        let op = OperatingPoint {
            airspeed: m.airspeed,
            airspeed_forward: m.airspeed_forward,
            vertical_speed: m.vertical_speed,
            yaw_rate: m.yaw_rate,
            air_density: m.air_density,
            distance_to_landing_m: input.distance_to_landing_m,
        };
        let pl = &self.plant;
        let gains = scheduled_gains(&self.schedule, &pl.model, input.phase, &op);
        self.core.compute(
            input,
            &gains,
            &pl.limits,
            &pl.settings,
            &pl.model,
            &pl.allocator,
            dt,
        )
    }

    fn reset(&mut self) {
        self.core.reset();
    }
}

pub fn build_controller(
    cfg: &ControllerConfig,
    p: &AirshipParams,
    d: &DerivedProperties,
    limits: ForceLimits,
) -> Box<dyn FlightController> {
    match cfg.kind {
        ControllerKind::CascadedPid => Box::new(CascadedPidController::new(p, d, limits, cfg)),
        ControllerKind::GainScheduled => Box::new(GainScheduledController::new(p, d, limits, cfg)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cascade::{
        ControlMeasurement, ControlReferences, HorizontalReference, PhaseClass, VerticalReference,
    };
    use crate::schedule::OperatingPoint;
    use airship_model::Vec2;

    fn setup() -> (AirshipParams, DerivedProperties, ForceLimits) {
        let p = AirshipParams::default();
        let d = DerivedProperties::compute(&p, 1.21);
        let l = ForceLimits {
            surge_min_n: -d.max_reverse_thrust_n,
            surge_max_n: d.max_forward_thrust_n,
            heave_min_n: -d.max_heave_down_n,
            heave_max_n: d.max_heave_up_n,
            yaw_max_nm: d.max_differential_yaw_moment_nm,
        };
        (p, d, l)
    }

    fn input(alt_ref: f64, alt: f64) -> ControlInput {
        ControlInput {
            phase: PhaseClass::Cruise,
            meas: ControlMeasurement {
                altitude_agl: alt,
                air_density: 1.21,
                ..Default::default()
            },
            refs: ControlReferences {
                vertical: VerticalReference {
                    altitude_agl: Some(alt_ref),
                    vertical_speed_ff: 0.0,
                    vertical_speed_min: -1.2,
                    vertical_speed_max: 1.5,
                },
                horizontal: HorizontalReference {
                    ground_velocity_ff: Vec2::new(3.0, 0.0),
                    hold_position: None,
                    max_ground_speed: 8.0,
                    preferred_yaw: None,
                    yaw_override: None,
                },
                heave_trim_n: 10.0,
                heave_override_n: None,
                max_airspeed: 10.0,
                max_yaw_rate: 0.17,
                horizontal_active: true,
                surge_enabled: true,
            },
            distance_to_landing_m: 300.0,
        }
    }

    #[test]
    fn controllers_command_climb_when_below_target() {
        let (p, d, l) = setup();
        for kind in [ControllerKind::CascadedPid, ControllerKind::GainScheduled] {
            let cfg = ControllerConfig {
                kind,
                ..Default::default()
            };
            let mut c = build_controller(&cfg, &p, &d, l);
            assert_eq!(c.kind(), kind);
            let out = c.compute(&input(40.0, 20.0), 0.05);
            assert!(out.references.vertical_speed > 0.0);
            assert!(out.virtual_command.heave_n > 10.0, "heave above trim");
            assert!(
                out.virtual_command.surge_n > 0.0,
                "forward thrust for forward ground velocity"
            );
            assert!(
                out.references.yaw.abs() < 1e-9,
                "ground velocity East: yaw reference 0"
            );
        }
    }

    #[test]
    fn controller_turns_toward_desired_track() {
        let (p, d, l) = setup();
        let mut c = build_controller(&ControllerConfig::default(), &p, &d, l);
        let mut i = input(30.0, 30.0);
        i.refs.horizontal.ground_velocity_ff = Vec2::new(0.0, 3.0); // fly North
        let out = c.compute(&i, 0.05);
        assert!(
            out.references.yaw_rate > 0.0,
            "turn left (CCW) from East to North"
        );
        assert!(out.virtual_command.yaw_moment_nm > 0.0);
    }

    #[test]
    fn wind_triangle_points_nose_into_crosswind() {
        let (p, d, l) = setup();
        let mut c = build_controller(&ControllerConfig::default(), &p, &d, l);
        let mut i = input(30.0, 30.0);
        i.meas.wind = Vec2::new(0.0, -3.0); // wind toward South
        let out = c.compute(&i, 0.05);
        // Desired ground track East, so the nose must crab toward North (positive yaw).
        assert!(out.references.yaw > 0.0);
    }

    #[test]
    fn horizontal_loops_inactive_on_ground() {
        let (p, d, l) = setup();
        let mut c = build_controller(&ControllerConfig::default(), &p, &d, l);
        let mut i = input(0.0, 0.0);
        i.refs.horizontal_active = false;
        let out = c.compute(&i, 0.05);
        assert_eq!(out.virtual_command.surge_n, 0.0);
        assert_eq!(out.virtual_command.yaw_moment_nm, 0.0);
    }

    #[test]
    fn default_fixed_gains_match_schedule_at_design_point() {
        // Documents how the Controller A defaults were derived.
        let (p, d, _) = setup();
        let cfg = ControllerConfig::default();
        let model = ControlModel::from_vehicle(&p, &d);
        let g = scheduled_gains(
            &cfg.gain_scheduled,
            &model,
            PhaseClass::Cruise,
            &OperatingPoint {
                airspeed: 4.0,
                airspeed_forward: 4.0,
                vertical_speed: 0.5,
                yaw_rate: 0.0,
                air_density: 1.21,
                distance_to_landing_m: 1e4,
            },
        );
        let a = cfg.cascaded_pid;
        let close = |x: f64, y: f64| (x - y).abs() / y < 0.2;
        assert!(
            close(a.vertical_speed.kp, g.vertical_speed.kp),
            "{} vs {}",
            a.vertical_speed.kp,
            g.vertical_speed.kp
        );
        assert!(
            close(a.yaw_rate.kp, g.yaw_rate.kp),
            "{} vs {}",
            a.yaw_rate.kp,
            g.yaw_rate.kp
        );
        assert!(
            close(a.airspeed.kp, g.airspeed.kp),
            "{} vs {}",
            a.airspeed.kp,
            g.airspeed.kp
        );
        assert!(close(a.yaw_rate.ki, g.yaw_rate.ki));
        assert!(
            close(a.vertical_speed.ki, g.vertical_speed.ki),
            "{} vs {}",
            a.vertical_speed.ki,
            g.vertical_speed.ki
        );
    }
}
