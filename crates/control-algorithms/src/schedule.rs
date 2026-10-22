//! Gain-scheduling law of Controller B.
//!
//! Every inner loop is a PI controller acting on a first-order plant
//! `M dx/dt = u - d(sigma) x`, where `M` is an effective mass or inertia and
//! `d(sigma)` is the **linearised aerodynamic damping at the current operating point**
//! `sigma`. Requiring the closed-loop characteristic polynomial
//! `s^2 + (d + Kp)/M s + Ki/M` to equal `s^2 + 2 zeta omega s + omega^2` gives
//!
//! ```text
//! Kp(sigma) = max(2 zeta omega M - d(sigma),  f_min 2 zeta omega M)
//! Ki        = omega^2 M
//! ```
//!
//! Scheduling variables and damping terms:
//!
//! | Loop          | M                  | d(sigma)                                   |
//! |---------------|--------------------|--------------------------------------------|
//! | vertical speed| m + k2 rho V       | rho Cdz Az |w|                             |
//! | yaw rate      | Iz + k' rho V ...  | 1/2 rho S CLa l^2 V_air + 2 c_h |r|        |
//! | airspeed      | m + k1 rho V       | rho Cdx Ax |u|                             |
//!
//! The target bandwidth `omega` of each loop is taken from a table indexed by the
//! flight-phase class; outer loops use `omega / n_outer` (altitude, heading).
//! The horizontal position gain is scheduled on the distance `d` to the landing point:
//!
//! ```text
//! K_pos(d) = K_far + (K_near - K_far) clamp(1 - d / d_s, 0, 1)
//! ```
//!
//! The law uses only measured quantities (airspeed, vertical speed, yaw rate,
//! distance) and the controller's nominal model constants.

use serde::{Deserialize, Serialize};

use crate::cascade::{ControlModel, LoopGains, PhaseClass};
use crate::pid::PidGains;
use airship_model::ValidationReport;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Bandwidths {
    pub vertical_rad_s: f64,
    pub yaw_rad_s: f64,
    pub speed_rad_s: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PhaseBandwidthTable {
    pub ground: Bandwidths,
    pub takeoff: Bandwidths,
    pub climb: Bandwidths,
    pub cruise: Bandwidths,
    pub approach: Bandwidths,
    pub landing: Bandwidths,
}

impl PhaseBandwidthTable {
    pub fn get(&self, c: PhaseClass) -> Bandwidths {
        match c {
            PhaseClass::Ground => self.ground,
            PhaseClass::Takeoff => self.takeoff,
            PhaseClass::Climb => self.climb,
            PhaseClass::Cruise => self.cruise,
            PhaseClass::Approach => self.approach,
            PhaseClass::Landing => self.landing,
        }
    }

    fn all(&self) -> [(&'static str, Bandwidths); 6] {
        [
            ("ground", self.ground),
            ("takeoff", self.takeoff),
            ("climb", self.climb),
            ("cruise", self.cruise),
            ("approach", self.approach),
            ("landing", self.landing),
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GainScheduleConfig {
    /// Closed-loop damping ratio `zeta` of the inner loops (-).
    pub damping_ratio: f64,
    /// Ratio between inner and outer loop bandwidth `n_outer` (-).
    pub outer_loop_ratio: f64,
    /// Lower bound of `Kp` as a fraction of `2 zeta omega M` (-).
    pub min_proportional_fraction: f64,
    pub bandwidths: PhaseBandwidthTable,
    /// Altitude-loop integral gain as a fraction of `omega_outer^2` (-).
    pub altitude_integral_fraction: f64,
    pub position_gain_far_1_s: f64,
    pub position_gain_near_1_s: f64,
    pub position_schedule_distance_m: f64,
}

impl Default for GainScheduleConfig {
    fn default() -> Self {
        let bw = |v, y, s| Bandwidths {
            vertical_rad_s: v,
            yaw_rad_s: y,
            speed_rad_s: s,
        };
        Self {
            damping_ratio: 0.9,
            outer_loop_ratio: 3.0,
            min_proportional_fraction: 0.25,
            bandwidths: PhaseBandwidthTable {
                ground: bw(0.8, 1.0, 0.5),
                takeoff: bw(1.0, 1.0, 0.6),
                climb: bw(0.7, 1.2, 0.6),
                cruise: bw(0.6, 1.2, 0.6),
                approach: bw(0.8, 1.2, 0.6),
                landing: bw(1.2, 1.4, 0.8),
            },
            altitude_integral_fraction: 0.1,
            position_gain_far_1_s: 0.12,
            position_gain_near_1_s: 0.35,
            position_schedule_distance_m: 40.0,
        }
    }
}

impl GainScheduleConfig {
    pub fn validate(&self) -> ValidationReport {
        let mut r = ValidationReport::new();
        r.range("damping_ratio", self.damping_ratio, 0.3, 2.0, "-");
        r.range("outer_loop_ratio", self.outer_loop_ratio, 1.5, 10.0, "-");
        r.range(
            "min_proportional_fraction",
            self.min_proportional_fraction,
            0.0,
            1.0,
            "-",
        );
        r.range(
            "altitude_integral_fraction",
            self.altitude_integral_fraction,
            0.0,
            1.0,
            "-",
        );
        r.range(
            "position_gain_far_1_s",
            self.position_gain_far_1_s,
            0.0,
            2.0,
            "1/s",
        );
        r.range(
            "position_gain_near_1_s",
            self.position_gain_near_1_s,
            0.0,
            2.0,
            "1/s",
        );
        r.range(
            "position_schedule_distance_m",
            self.position_schedule_distance_m,
            1.0,
            5000.0,
            "m",
        );
        for (name, b) in self.bandwidths.all() {
            r.range(
                &format!("bandwidths.{name}.vertical_rad_s"),
                b.vertical_rad_s,
                0.05,
                5.0,
                "rad/s",
            );
            r.range(
                &format!("bandwidths.{name}.yaw_rad_s"),
                b.yaw_rad_s,
                0.05,
                5.0,
                "rad/s",
            );
            r.range(
                &format!("bandwidths.{name}.speed_rad_s"),
                b.speed_rad_s,
                0.05,
                5.0,
                "rad/s",
            );
        }
        r
    }
}

/// Operating point used by the schedule.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct OperatingPoint {
    pub airspeed: f64,
    pub airspeed_forward: f64,
    pub vertical_speed: f64,
    pub yaw_rate: f64,
    pub air_density: f64,
    pub distance_to_landing_m: f64,
}

fn pi_gains(zeta: f64, omega: f64, mass: f64, damping: f64, fmin: f64) -> PidGains {
    let nominal = 2.0 * zeta * omega * mass;
    PidGains {
        kp: (nominal - damping).max(fmin * nominal),
        ki: omega * omega * mass,
        kd: 0.0,
    }
}

/// Evaluate the scheduling law.
pub fn scheduled_gains(
    cfg: &GainScheduleConfig,
    model: &ControlModel,
    phase: PhaseClass,
    op: &OperatingPoint,
) -> LoopGains {
    let bw = cfg.bandwidths.get(phase);
    let z = cfg.damping_ratio;
    let fmin = cfg.min_proportional_fraction;
    let rho = if op.air_density > 0.0 {
        op.air_density
    } else {
        model.reference_density
    };
    let rho_ratio = rho / model.reference_density;

    let d_w = rho * model.vertical_drag_area * op.vertical_speed.abs();
    let d_r = model.fin_yaw_damping * rho_ratio * op.airspeed
        + 2.0 * model.hull_yaw_damping * rho_ratio * op.yaw_rate.abs();
    let d_u = rho * model.axial_drag_area * op.airspeed_forward.abs();

    let w_out_v = bw.vertical_rad_s / cfg.outer_loop_ratio;
    let w_out_y = bw.yaw_rad_s / cfg.outer_loop_ratio;
    let s = (1.0 - op.distance_to_landing_m / cfg.position_schedule_distance_m).clamp(0.0, 1.0);

    LoopGains {
        altitude: PidGains {
            kp: w_out_v,
            ki: cfg.altitude_integral_fraction * w_out_v * w_out_v,
            kd: 0.0,
        },
        vertical_speed: pi_gains(z, bw.vertical_rad_s, model.mass_vertical_kg, d_w, fmin),
        heading: PidGains {
            kp: w_out_y,
            ki: 0.0,
            kd: 0.0,
        },
        yaw_rate: pi_gains(z, bw.yaw_rad_s, model.inertia_yaw_kg_m2, d_r, fmin),
        airspeed: pi_gains(z, bw.speed_rad_s, model.mass_axial_kg, d_u, fmin),
        position: cfg.position_gain_far_1_s
            + (cfg.position_gain_near_1_s - cfg.position_gain_far_1_s) * s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use airship_model::{AirshipParams, DerivedProperties};

    fn model() -> ControlModel {
        let p = AirshipParams::default();
        ControlModel::from_vehicle(&p, &DerivedProperties::compute(&p, 1.2))
    }

    fn op(airspeed: f64, distance: f64) -> OperatingPoint {
        OperatingPoint {
            airspeed,
            airspeed_forward: airspeed,
            vertical_speed: 0.0,
            yaw_rate: 0.0,
            air_density: 1.2,
            distance_to_landing_m: distance,
        }
    }

    #[test]
    fn closed_loop_poles_match_design_at_operating_point() {
        let cfg = GainScheduleConfig::default();
        let m = model();
        let g = scheduled_gains(&cfg, &m, PhaseClass::Cruise, &op(4.0, 1000.0));
        // Yaw-rate loop: (d + Kp)/I = 2 zeta omega and Ki/I = omega^2.
        let d = m.fin_yaw_damping * 4.0;
        let w = cfg.bandwidths.cruise.yaw_rad_s;
        assert!(
            ((d + g.yaw_rate.kp) / m.inertia_yaw_kg_m2 - 2.0 * cfg.damping_ratio * w).abs() < 1e-9
        );
        assert!((g.yaw_rate.ki / m.inertia_yaw_kg_m2 - w * w).abs() < 1e-9);
    }

    #[test]
    fn proportional_gain_decreases_with_aerodynamic_damping() {
        let cfg = GainScheduleConfig::default();
        let m = model();
        let slow = scheduled_gains(&cfg, &m, PhaseClass::Cruise, &op(1.0, 1000.0));
        let fast = scheduled_gains(&cfg, &m, PhaseClass::Cruise, &op(8.0, 1000.0));
        assert!(fast.yaw_rate.kp < slow.yaw_rate.kp);
        assert!(fast.airspeed.kp < slow.airspeed.kp);
        assert_eq!(fast.yaw_rate.ki, slow.yaw_rate.ki);
        // Lower bound holds at extreme damping.
        let extreme = scheduled_gains(&cfg, &m, PhaseClass::Cruise, &op(200.0, 1000.0));
        let nominal =
            2.0 * cfg.damping_ratio * cfg.bandwidths.cruise.yaw_rad_s * m.inertia_yaw_kg_m2;
        assert!((extreme.yaw_rate.kp - cfg.min_proportional_fraction * nominal).abs() < 1e-9);
    }

    #[test]
    fn landing_phase_has_higher_vertical_bandwidth() {
        let cfg = GainScheduleConfig::default();
        let m = model();
        let cruise = scheduled_gains(&cfg, &m, PhaseClass::Cruise, &op(0.0, 1000.0));
        let landing = scheduled_gains(&cfg, &m, PhaseClass::Landing, &op(0.0, 1000.0));
        assert!(landing.vertical_speed.kp > cruise.vertical_speed.kp);
        assert!(landing.vertical_speed.ki > cruise.vertical_speed.ki);
    }

    #[test]
    fn position_gain_scheduled_on_distance() {
        let cfg = GainScheduleConfig::default();
        let m = model();
        let far = scheduled_gains(&cfg, &m, PhaseClass::Landing, &op(0.0, 500.0));
        let mid = scheduled_gains(&cfg, &m, PhaseClass::Landing, &op(0.0, 20.0));
        let near = scheduled_gains(&cfg, &m, PhaseClass::Landing, &op(0.0, 0.0));
        assert_eq!(far.position, cfg.position_gain_far_1_s);
        assert_eq!(near.position, cfg.position_gain_near_1_s);
        assert!(mid.position > far.position && mid.position < near.position);
    }

    #[test]
    fn default_config_is_valid() {
        assert!(!GainScheduleConfig::default().validate().has_errors());
    }
}
