//! Actuator dynamics and the propulsion power estimate.
//!
//! * Thrusters: saturated first-order lag `dT/dt = (sat(T_cmd) - T) / tau`,
//!   discretised exactly over one physics step (zero-order-hold command).
//! * Rudder: position limit followed by a slew-rate limit.
//!
//! Actuator states are held constant during the RK4 stages of a step.

use airship_model::AirshipParams;
use serde::{Deserialize, Serialize};

/// Commanded actuator set-points produced by the control allocation.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct ActuatorCommand {
    /// Port (left) main thruster, positive forward (N).
    pub thrust_left_n: f64,
    /// Starboard (right) main thruster, positive forward (N).
    pub thrust_right_n: f64,
    /// Heave thruster, positive up along body z (N).
    pub thrust_heave_n: f64,
    /// Rudder deflection, positive produces a positive (nose-left) yaw moment (rad).
    pub rudder_rad: f64,
}

/// Realised actuator outputs (same fields and conventions as [`ActuatorCommand`]).
pub type ActuatorState = ActuatorCommand;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ActuatorLimits {
    pub main_max_n: f64,
    pub main_min_n: f64,
    pub heave_max_n: f64,
    pub heave_min_n: f64,
    pub rudder_max_rad: f64,
    pub rudder_rate_rad_s: f64,
    pub tau_s: f64,
}

impl ActuatorLimits {
    pub fn from_params(p: &AirshipParams) -> Self {
        let pr = &p.propulsion;
        Self {
            main_max_n: pr.main_max_thrust_n,
            main_min_n: -pr.main_max_thrust_n * pr.main_reverse_ratio,
            heave_max_n: pr.heave_max_thrust_n,
            heave_min_n: -pr.heave_max_thrust_n * pr.heave_reverse_ratio,
            rudder_max_rad: p.rudder.max_deflection_deg.to_radians(),
            rudder_rate_rad_s: p.rudder.max_rate_deg_s.to_radians(),
            tau_s: pr.response_time_s,
        }
    }

    /// Clamp a command to the static actuator limits.
    pub fn saturate(&self, c: &ActuatorCommand) -> ActuatorCommand {
        ActuatorCommand {
            thrust_left_n: c.thrust_left_n.clamp(self.main_min_n, self.main_max_n),
            thrust_right_n: c.thrust_right_n.clamp(self.main_min_n, self.main_max_n),
            thrust_heave_n: c.thrust_heave_n.clamp(self.heave_min_n, self.heave_max_n),
            rudder_rad: c
                .rudder_rad
                .clamp(-self.rudder_max_rad, self.rudder_max_rad),
        }
    }
}

impl ActuatorState {
    /// Advance the actuator states by `dt` toward the (saturated) command.
    pub fn advance(&mut self, cmd: &ActuatorCommand, lim: &ActuatorLimits, dt: f64) {
        if !(dt > 0.0) {
            return;
        }
        let c = lim.saturate(cmd);
        let a = 1.0 - (-dt / lim.tau_s).exp();
        self.thrust_left_n += a * (c.thrust_left_n - self.thrust_left_n);
        self.thrust_right_n += a * (c.thrust_right_n - self.thrust_right_n);
        self.thrust_heave_n += a * (c.thrust_heave_n - self.thrust_heave_n);
        let max_step = lim.rudder_rate_rad_s * dt;
        self.rudder_rad += (c.rudder_rad - self.rudder_rad).clamp(-max_step, max_step);
    }
}

/// Electrical power (W) of one thruster from actuator-disk momentum theory:
///
/// ```text
/// v_i = -V/2 + sqrt(V^2/4 + |T| / (2 rho A))
/// P   = |T| (V + v_i) / eta
/// ```
///
/// `axial_inflow_m_s` is the air speed entering the disk along the thrust direction
/// (negative values are treated as zero, a conservative simplification).
pub fn thruster_power(
    thrust_n: f64,
    axial_inflow_m_s: f64,
    rho: f64,
    disk_area_m2: f64,
    efficiency: f64,
) -> f64 {
    let t = thrust_n.abs();
    if t < 1e-9 {
        return 0.0;
    }
    let v = axial_inflow_m_s.max(0.0);
    let vi = -0.5 * v + (0.25 * v * v + t / (2.0 * rho * disk_area_m2)).sqrt();
    t * (v + vi) / efficiency
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> ActuatorLimits {
        ActuatorLimits::from_params(&AirshipParams::default())
    }

    #[test]
    fn thrust_lag_reaches_63_percent_after_tau() {
        let lim = limits();
        let mut s = ActuatorState::default();
        let cmd = ActuatorCommand {
            thrust_left_n: 40.0,
            ..Default::default()
        };
        let dt = 0.001;
        let n = (lim.tau_s / dt).round() as usize;
        for _ in 0..n {
            s.advance(&cmd, &lim, dt);
        }
        assert!((s.thrust_left_n / 40.0 - (1.0 - (-1.0f64).exp())).abs() < 1e-3);
    }

    #[test]
    fn commands_are_saturated() {
        let lim = limits();
        let mut s = ActuatorState::default();
        let cmd = ActuatorCommand {
            thrust_left_n: 1e6,
            thrust_right_n: -1e6,
            thrust_heave_n: 1e6,
            rudder_rad: 10.0,
        };
        for _ in 0..10_000 {
            s.advance(&cmd, &lim, 0.01);
        }
        assert!((s.thrust_left_n - lim.main_max_n).abs() < 1e-6);
        assert!((s.thrust_right_n - lim.main_min_n).abs() < 1e-6);
        assert!((s.thrust_heave_n - lim.heave_max_n).abs() < 1e-6);
        assert!((s.rudder_rad - lim.rudder_max_rad).abs() < 1e-9);
    }

    #[test]
    fn rudder_is_rate_limited() {
        let lim = limits();
        let mut s = ActuatorState::default();
        let cmd = ActuatorCommand {
            rudder_rad: lim.rudder_max_rad,
            ..Default::default()
        };
        s.advance(&cmd, &lim, 0.1);
        assert!((s.rudder_rad - lim.rudder_rate_rad_s * 0.1).abs() < 1e-12);
    }

    #[test]
    fn invalid_dt_is_ignored() {
        let lim = limits();
        let mut s = ActuatorState::default();
        let cmd = ActuatorCommand {
            thrust_left_n: 10.0,
            ..Default::default()
        };
        s.advance(&cmd, &lim, f64::NAN);
        s.advance(&cmd, &lim, -1.0);
        assert_eq!(s, ActuatorState::default());
    }

    #[test]
    fn hover_power_matches_ideal_formula() {
        let rho = 1.225;
        let a = 0.3;
        let p = thruster_power(50.0, 0.0, rho, a, 1.0);
        assert!((p - 50f64.powf(1.5) / (2.0 * rho * a).sqrt()).abs() < 1e-9);
        // Forward inflow increases the power needed for the same thrust.
        assert!(thruster_power(50.0, 5.0, rho, a, 1.0) > p);
        assert_eq!(thruster_power(0.0, 3.0, rho, a, 0.5), 0.0);
    }
}
