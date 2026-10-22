//! PID controller with anti-windup, filtered derivative, saturation and rate limiting.
//!
//! ```text
//! u = sat( Kp e + I + D + u_ff )
//! I_k = clamp(I_{k-1} + Ki e dt, I_min, I_max)       (only if it does not deepen saturation)
//! D   = -Kd * LPF(dy/dt)                              (derivative on measurement, no set-point kick)
//! u_k = u_{k-1} + clamp(u - u_{k-1}, -R dt, R dt)    (optional output rate limit)
//! ```
//!
//! The integrator stores the integral *term* (already multiplied by `Ki`), so changing
//! the gains between steps (gain scheduling) does not cause an output jump
//! (bumpless gain changes).
//!
//! Invalid inputs (non-positive or non-finite `dt`, non-finite error) do not update the
//! internal state; the previous output is held.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PidGains {
    pub kp: f64,
    pub ki: f64,
    pub kd: f64,
}

impl PidGains {
    pub const fn new(kp: f64, ki: f64, kd: f64) -> Self {
        Self { kp, ki, kd }
    }

    pub fn is_valid(&self) -> bool {
        [self.kp, self.ki, self.kd]
            .iter()
            .all(|g| g.is_finite() && *g >= 0.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PidLimits {
    pub output_min: f64,
    pub output_max: f64,
    pub integrator_min: f64,
    pub integrator_max: f64,
    /// Maximum output slew rate (units/s); `f64::INFINITY` disables rate limiting.
    pub rate_limit: f64,
    /// Time constant of the derivative low-pass filter (s).
    pub derivative_tau_s: f64,
}

impl PidLimits {
    /// Symmetric limits with the integrator bounded by the output range.
    pub fn symmetric(max: f64) -> Self {
        Self::range(-max, max)
    }

    pub fn range(min: f64, max: f64) -> Self {
        Self {
            output_min: min,
            output_max: max,
            integrator_min: min,
            integrator_max: max,
            rate_limit: f64::INFINITY,
            derivative_tau_s: 0.2,
        }
    }

    pub fn with_rate_limit(mut self, r: f64) -> Self {
        self.rate_limit = r;
        self
    }
}

/// Individual terms of the last update (for telemetry and tests).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct PidTerms {
    pub p: f64,
    pub i: f64,
    pub d: f64,
    pub ff: f64,
    pub output: f64,
    pub saturated: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Pid {
    integral: f64,
    d_filtered: f64,
    prev_measurement: Option<f64>,
    prev_output: Option<f64>,
    pub last: PidTerms,
}

impl Pid {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn integral(&self) -> f64 {
        self.integral
    }

    /// Initialise the integral term (bumpless engagement).
    pub fn preset_integral(&mut self, value: f64) {
        if value.is_finite() {
            self.integral = value;
        }
    }

    /// One controller update.
    ///
    /// * `error` - set-point minus measurement (already wrapped for angles).
    /// * `measurement` - used for the derivative when `measurement_rate` is `None`.
    /// * `measurement_rate` - directly measured derivative of the controlled variable.
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        gains: &PidGains,
        lim: &PidLimits,
        error: f64,
        measurement: f64,
        measurement_rate: Option<f64>,
        feedforward: f64,
        dt: f64,
    ) -> f64 {
        let hold = self
            .prev_output
            .unwrap_or(0.0)
            .clamp(lim.output_min, lim.output_max);
        if !(dt > 0.0 && dt.is_finite())
            || !error.is_finite()
            || !measurement.is_finite()
            || !feedforward.is_finite()
        {
            return hold;
        }
        let raw_rate = match measurement_rate {
            Some(r) if r.is_finite() => r,
            _ => self
                .prev_measurement
                .map(|p| (measurement - p) / dt)
                .unwrap_or(0.0),
        };
        self.prev_measurement = Some(measurement);
        let a = if lim.derivative_tau_s > 0.0 {
            dt / (lim.derivative_tau_s + dt)
        } else {
            1.0
        };
        self.d_filtered += a * (raw_rate - self.d_filtered);

        let p = gains.kp * error;
        let d = -gains.kd * self.d_filtered;
        let candidate =
            (self.integral + gains.ki * error * dt).clamp(lim.integrator_min, lim.integrator_max);
        let u_candidate = p + candidate + d + feedforward;
        let deepens = (u_candidate > lim.output_max && error > 0.0)
            || (u_candidate < lim.output_min && error < 0.0);
        if !deepens {
            self.integral = candidate;
        }
        // Keep the stored integral inside its bounds when limits change between calls.
        self.integral = self.integral.clamp(lim.integrator_min, lim.integrator_max);

        let unsat = p + self.integral + d + feedforward;
        let mut u = unsat.clamp(lim.output_min, lim.output_max);
        if let Some(prev) = self.prev_output
            && lim.rate_limit.is_finite()
        {
            let max_step = lim.rate_limit * dt;
            u = prev + (u - prev).clamp(-max_step, max_step);
        }
        self.prev_output = Some(u);
        self.last = PidTerms {
            p,
            i: self.integral,
            d,
            ff: feedforward,
            output: u,
            saturated: (unsat - u).abs() > 1e-9,
        };
        u
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// First-order plant m dx/dt = u - c x under PI control.
    fn simulate(gains: PidGains, lim: PidLimits, setpoint: f64, steps: usize) -> (f64, Pid) {
        let (m, c, dt) = (10.0, 1.0, 0.01);
        let mut x = 0.0;
        let mut pid = Pid::new();
        for _ in 0..steps {
            let u = pid.update(&gains, &lim, setpoint - x, x, None, 0.0, dt);
            x += dt * (u - c * x) / m;
        }
        (x, pid)
    }

    #[test]
    fn proportional_only_has_steady_state_error() {
        let (x, _) = simulate(
            PidGains::new(5.0, 0.0, 0.0),
            PidLimits::symmetric(100.0),
            1.0,
            20_000,
        );
        // Steady state x = kp/(kp + c) = 5/6
        assert!((x - 5.0 / 6.0).abs() < 1e-3);
    }

    #[test]
    fn integral_removes_steady_state_error() {
        let (x, _) = simulate(
            PidGains::new(5.0, 2.0, 0.0),
            PidLimits::symmetric(100.0),
            1.0,
            20_000,
        );
        assert!((x - 1.0).abs() < 1e-3);
    }

    #[test]
    fn output_is_saturated() {
        let mut pid = Pid::new();
        let u = pid.update(
            &PidGains::new(100.0, 0.0, 0.0),
            &PidLimits::symmetric(3.0),
            10.0,
            0.0,
            None,
            0.0,
            0.01,
        );
        assert_eq!(u, 3.0);
        assert!(pid.last.saturated);
    }

    #[test]
    fn anti_windup_limits_integrator_during_saturation() {
        let gains = PidGains::new(1.0, 5.0, 0.0);
        let lim = PidLimits::symmetric(1.0);
        let mut pid = Pid::new();
        // Large persistent error saturates the output; the integrator must not wind up.
        for _ in 0..10_000 {
            pid.update(&gains, &lim, 50.0, 0.0, None, 0.0, 0.01);
        }
        assert!(pid.integral() <= 1.0 + 1e-12);
        // When the error reverses, the output leaves saturation immediately (no windup delay).
        let u = pid.update(&gains, &lim, -2.0, 0.0, None, 0.0, 0.01);
        assert!(u < 1.0, "output {u} should drop below the limit at once");
    }

    #[test]
    fn without_anti_windup_comparison_overshoot_is_smaller() {
        // Saturated plant: compare overshoot to a naive integrator implementation.
        let (m, c, dt) = (10.0, 1.0, 0.01);
        let gains = PidGains::new(4.0, 3.0, 0.0);
        let umax = 3.0;
        let run = |naive: bool| {
            let mut x: f64 = 0.0;
            let mut naive_i = 0.0;
            let mut pid = Pid::new();
            let mut peak: f64 = 0.0;
            for _ in 0..30_000 {
                let e = 2.0 - x;
                let u = if naive {
                    naive_i += gains.ki * e * dt;
                    (gains.kp * e + naive_i).clamp(-umax, umax)
                } else {
                    pid.update(&gains, &PidLimits::symmetric(umax), e, x, None, 0.0, dt)
                };
                x += dt * (u - c * x) / m;
                peak = peak.max(x);
            }
            peak
        };
        assert!(run(false) < run(true));
    }

    #[test]
    fn derivative_acts_on_measurement_without_setpoint_kick() {
        let gains = PidGains::new(0.0, 0.0, 1.0);
        let mut pid = Pid::new();
        let lim = PidLimits::symmetric(1e6);
        pid.update(&gains, &lim, 0.0, 0.0, None, 0.0, 0.01);
        // Set-point step with constant measurement: no derivative kick.
        let u = pid.update(&gains, &lim, 10.0, 0.0, None, 0.0, 0.01);
        assert_eq!(u, 0.0);
        // A measured rate produces a (filtered) opposing derivative term.
        let u = pid.update(&gains, &lim, 0.0, 0.0, Some(2.0), 0.0, 0.01);
        assert!(u < 0.0);
    }

    #[test]
    fn rate_limit_is_enforced() {
        let mut pid = Pid::new();
        let lim = PidLimits::symmetric(100.0).with_rate_limit(10.0);
        let g = PidGains::new(1.0, 0.0, 0.0);
        pid.update(&g, &lim, 0.0, 0.0, None, 0.0, 0.1);
        let u = pid.update(&g, &lim, 50.0, 0.0, None, 0.0, 0.1);
        assert!((u - 1.0).abs() < 1e-12);
    }

    #[test]
    fn invalid_dt_holds_output_and_state() {
        let mut pid = Pid::new();
        let g = PidGains::new(2.0, 1.0, 0.0);
        let lim = PidLimits::symmetric(100.0);
        let u0 = pid.update(&g, &lim, 1.0, 0.0, None, 0.0, 0.1);
        let i0 = pid.integral();
        for dt in [0.0, -0.1, f64::NAN, f64::INFINITY] {
            assert_eq!(pid.update(&g, &lim, 5.0, 0.0, None, 0.0, dt), u0);
            assert_eq!(pid.integral(), i0);
        }
        assert_eq!(pid.update(&g, &lim, f64::NAN, 0.0, None, 0.0, 0.1), u0);
    }

    #[test]
    fn gain_change_is_bumpless() {
        let mut pid = Pid::new();
        let lim = PidLimits::symmetric(100.0);
        for _ in 0..100 {
            pid.update(
                &PidGains::new(1.0, 2.0, 0.0),
                &lim,
                1.0,
                0.0,
                None,
                0.0,
                0.01,
            );
        }
        let before = pid.last.output;
        // Changing Ki does not rescale the accumulated integral term.
        let after = pid.update(
            &PidGains::new(1.0, 20.0, 0.0),
            &lim,
            1.0,
            0.0,
            None,
            0.0,
            1e-9,
        );
        assert!((after - before).abs() < 1e-6);
    }
}
