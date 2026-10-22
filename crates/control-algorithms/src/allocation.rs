//! Control allocation: virtual commands (surge force, heave force, yaw moment) to
//! actuator set-points.
//!
//! 1. **Heave**: `T_h = sat(F_z)`.
//! 2. **Rudder first**: the rudder moment per radian at the measured axial airspeed is
//!    `N_delta = 1/2 rho u|u| S CL_delta l_f`. If it is significant the rudder takes
//!    `delta = sat(N / N_delta)`; at hover it stays centred because it has no authority.
//! 3. **Differential thrust** supplies the remaining moment `N_rem = N - N_delta delta`:
//!    `T_R - T_L = N_rem / b`. Yaw has priority over surge in one direction only: to make
//!    room for the thrust difference the common-mode (surge) part may be *reduced*
//!    without limit, but *increased* by at most a small allowance per motor. Because
//!    reverse thrust is weaker than forward thrust, unrestricted yaw priority would
//!    otherwise add net forward thrust during braking turns and the vehicle would
//!    accelerate while the speed loop demands deceleration.
//!
//! The allocation uses the *measured* airspeed but no other knowledge of the plant.

use airship_model::AirshipParams;
use flight_dynamics::{ActuatorCommand, ActuatorLimits};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct VirtualCommand {
    /// Body-x force (N).
    pub surge_n: f64,
    /// Body-z force (N).
    pub heave_n: f64,
    /// Yaw moment, positive nose-left (N m).
    pub yaw_moment_nm: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct AllocationResult {
    pub command: ActuatorCommand,
    /// Virtual command the actuator set-points will produce (static model).
    pub achieved: VirtualCommand,
    pub surge_saturated: bool,
    pub heave_saturated: bool,
    pub yaw_saturated: bool,
    /// Fraction of the achieved yaw moment produced by the rudder (-).
    pub rudder_share: f64,
}

#[derive(Debug, Clone)]
pub struct ControlAllocator {
    pub limits: ActuatorLimits,
    arm_m: f64,
    /// `S CL_delta l_f` (m^3): rudder moment per radian per pascal of dynamic pressure.
    rudder_factor: f64,
    /// Extra forward thrust per motor that the yaw allocation may add beyond the
    /// surge command (N). Bounding it keeps the speed loop able to brake in turns.
    forward_allowance_n: f64,
}

impl ControlAllocator {
    pub fn new(p: &AirshipParams) -> Self {
        Self {
            limits: ActuatorLimits::from_params(p),
            arm_m: p.propulsion.main_lateral_arm_m,
            rudder_factor: p.aero.fin_area_m2 * p.rudder.effectiveness_per_rad * p.aero.fin_arm_m,
            forward_allowance_n: 0.1 * p.propulsion.main_max_thrust_n,
        }
    }

    /// Maximum yaw moment from differential thrust alone (N m).
    pub fn max_differential_moment(&self) -> f64 {
        self.arm_m * (self.limits.main_max_n - self.limits.main_min_n)
    }

    /// Maximum rudder moment at axial airspeed `u` (N m).
    pub fn max_rudder_moment(&self, u: f64, rho: f64) -> f64 {
        0.5 * rho * u * u * self.rudder_factor * self.limits.rudder_max_rad
    }

    pub fn allocate(&self, v: &VirtualCommand, airspeed_u: f64, rho: f64) -> AllocationResult {
        let lim = &self.limits;
        let heave = v.heave_n.clamp(lim.heave_min_n, lim.heave_max_n);

        let n_per_rad = 0.5 * rho * airspeed_u * airspeed_u.abs() * self.rudder_factor;
        let (rudder, n_rudder) = if (n_per_rad * lim.rudder_max_rad).abs() > 1.0 {
            let d = (v.yaw_moment_nm / n_per_rad).clamp(-lim.rudder_max_rad, lim.rudder_max_rad);
            (d, n_per_rad * d)
        } else {
            (0.0, 0.0)
        };

        let span = lim.main_max_n - lim.main_min_n;
        let diff_req = ((v.yaw_moment_nm - n_rudder) / self.arm_m).clamp(-span, span);
        let half_req = diff_req.abs() / 2.0;
        let c_surge = (v.surge_n / 2.0).clamp(lim.main_min_n, lim.main_max_n);
        // Yaw-priority common mode, but yaw may only *reduce* the forward thrust freely;
        // it may add at most `forward_allowance` per motor beyond the surge command.
        let c_yaw = c_surge.clamp(lim.main_min_n + half_req, lim.main_max_n - half_req);
        let common = c_yaw.min(c_surge + self.forward_allowance_n);
        let half_avail = (lim.main_max_n - common)
            .min(common - lim.main_min_n)
            .max(0.0);
        let diff = diff_req.clamp(-2.0 * half_avail, 2.0 * half_avail);
        let right = common + diff / 2.0;
        let left = common - diff / 2.0;

        let achieved = VirtualCommand {
            surge_n: left + right,
            heave_n: heave,
            yaw_moment_nm: n_rudder + self.arm_m * (right - left),
        };
        let tol = |a: f64, b: f64| (a - b).abs() > 1e-6 * (1.0 + b.abs());
        AllocationResult {
            command: ActuatorCommand {
                thrust_left_n: left,
                thrust_right_n: right,
                thrust_heave_n: heave,
                rudder_rad: rudder,
            },
            achieved,
            surge_saturated: tol(achieved.surge_n, v.surge_n),
            heave_saturated: tol(achieved.heave_n, v.heave_n),
            yaw_saturated: tol(achieved.yaw_moment_nm, v.yaw_moment_nm),
            rudder_share: if achieved.yaw_moment_nm.abs() > 1e-9 {
                n_rudder / achieved.yaw_moment_nm
            } else {
                0.0
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alloc() -> ControlAllocator {
        ControlAllocator::new(&AirshipParams::default())
    }

    #[test]
    fn feasible_commands_are_reproduced_exactly() {
        let a = alloc();
        let v = VirtualCommand {
            surge_n: 40.0,
            heave_n: 20.0,
            yaw_moment_nm: 15.0,
        };
        let r = a.allocate(&v, 0.0, 1.2);
        assert!((r.achieved.surge_n - 40.0).abs() < 1e-9);
        assert!((r.achieved.yaw_moment_nm - 15.0).abs() < 1e-9);
        assert_eq!(r.command.rudder_rad, 0.0, "no rudder at hover");
        assert!(!r.surge_saturated && !r.yaw_saturated && !r.heave_saturated);
    }

    #[test]
    fn rudder_takes_the_moment_at_speed() {
        let a = alloc();
        let v = VirtualCommand {
            surge_n: 30.0,
            heave_n: 0.0,
            yaw_moment_nm: 40.0,
        };
        let r = a.allocate(&v, 6.0, 1.2);
        assert!(r.command.rudder_rad > 0.0);
        assert!((r.command.thrust_left_n - r.command.thrust_right_n).abs() < 1e-9);
        assert!((r.rudder_share - 1.0).abs() < 1e-9);
    }

    #[test]
    fn yaw_has_priority_over_surge() {
        let a = alloc();
        let v = VirtualCommand {
            surge_n: 120.0,
            heave_n: 0.0,
            yaw_moment_nm: 60.0,
        };
        let r = a.allocate(&v, 0.0, 1.2);
        assert!(
            (r.achieved.yaw_moment_nm - 60.0).abs() < 1e-9,
            "yaw moment preserved"
        );
        assert!(r.surge_saturated && !r.yaw_saturated);
        let l = &a.limits;
        for t in [r.command.thrust_left_n, r.command.thrust_right_n] {
            assert!(t <= l.main_max_n + 1e-9 && t >= l.main_min_n - 1e-9);
        }
    }

    #[test]
    fn yaw_cannot_add_large_forward_thrust_while_braking() {
        let a = alloc();
        // Hover/braking with a large yaw demand.
        let r = a.allocate(
            &VirtualCommand {
                surge_n: 0.0,
                heave_n: 0.0,
                yaw_moment_nm: -500.0,
            },
            0.0,
            1.2,
        );
        assert!(
            r.achieved.surge_n <= 2.0 * 0.1 * 60.0 + 1e-9,
            "net forward thrust {}",
            r.achieved.surge_n
        );
        assert!(
            r.achieved.yaw_moment_nm < -50.0,
            "still substantial yaw authority"
        );
        // Continuity across zero surge command.
        let r1 = a.allocate(
            &VirtualCommand {
                surge_n: -0.01,
                heave_n: 0.0,
                yaw_moment_nm: 80.0,
            },
            0.0,
            1.2,
        );
        let r2 = a.allocate(
            &VirtualCommand {
                surge_n: 0.01,
                heave_n: 0.0,
                yaw_moment_nm: 80.0,
            },
            0.0,
            1.2,
        );
        assert!((r1.achieved.yaw_moment_nm - r2.achieved.yaw_moment_nm).abs() < 0.1);
    }

    #[test]
    fn heave_is_saturated() {
        let a = alloc();
        let r = a.allocate(
            &VirtualCommand {
                heave_n: 1e4,
                ..Default::default()
            },
            0.0,
            1.2,
        );
        assert!(r.heave_saturated);
        assert_eq!(r.command.thrust_heave_n, a.limits.heave_max_n);
    }
}
