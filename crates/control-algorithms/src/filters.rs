//! Discrete first-order low-pass filter.

/// `y_k = y_{k-1} + dt / (tau + dt) * (x_k - y_{k-1})` (backward-Euler discretisation).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LowPass {
    pub tau_s: f64,
    state: Option<f64>,
}

impl LowPass {
    pub fn new(tau_s: f64) -> Self {
        Self { tau_s, state: None }
    }

    pub fn update(&mut self, x: f64, dt: f64) -> f64 {
        let y = match self.state {
            None => x,
            Some(prev) if dt > 0.0 && dt.is_finite() && x.is_finite() => {
                let a = if self.tau_s > 0.0 {
                    dt / (self.tau_s + dt)
                } else {
                    1.0
                };
                prev + a * (x - prev)
            }
            Some(prev) => prev,
        };
        self.state = Some(y);
        y
    }

    pub fn value(&self) -> Option<f64> {
        self.state
    }

    pub fn reset(&mut self) {
        self.state = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converges_to_step_input() {
        let mut f = LowPass::new(1.0);
        f.update(0.0, 0.01);
        let mut y = 0.0;
        for _ in 0..1000 {
            y = f.update(1.0, 0.01);
        }
        assert!((y - 1.0).abs() < 1e-3);
    }

    #[test]
    fn invalid_dt_holds_value() {
        let mut f = LowPass::new(1.0);
        f.update(2.0, 0.1);
        assert_eq!(f.update(10.0, 0.0), 2.0);
        assert_eq!(f.update(10.0, f64::NAN), 2.0);
    }
}
