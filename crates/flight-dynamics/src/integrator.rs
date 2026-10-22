//! Classical fourth-order Runge-Kutta step for a fixed-size state vector.
//!
//! The local truncation error is `O(dt^5)` and the global error `O(dt^4)` for
//! smooth right-hand sides. Non-smooth terms (ground contact onset, quadratic
//! drag at zero relative velocity) reduce the observed order locally.

/// Advance `x` from `t` to `t + dt` using `f(t, x) -> dx/dt`.
pub fn rk4_step<const N: usize>(
    x: &[f64; N],
    t: f64,
    dt: f64,
    mut f: impl FnMut(f64, &[f64; N]) -> [f64; N],
) -> [f64; N] {
    let k1 = f(t, x);
    let x2 = axpy(x, &k1, 0.5 * dt);
    let k2 = f(t + 0.5 * dt, &x2);
    let x3 = axpy(x, &k2, 0.5 * dt);
    let k3 = f(t + 0.5 * dt, &x3);
    let x4 = axpy(x, &k3, dt);
    let k4 = f(t + dt, &x4);
    let mut out = *x;
    for i in 0..N {
        out[i] += dt / 6.0 * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]);
    }
    out
}

fn axpy<const N: usize>(x: &[f64; N], k: &[f64; N], h: f64) -> [f64; N] {
    let mut out = *x;
    for i in 0..N {
        out[i] += h * k[i];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Harmonic oscillator x'' = -x; exact solution cos(t).
    fn integrate(dt: f64) -> f64 {
        let mut x = [1.0, 0.0];
        let n = (2.0 / dt).round() as usize;
        let mut t = 0.0;
        for _ in 0..n {
            x = rk4_step(&x, t, dt, |_, s| [s[1], -s[0]]);
            t += dt;
        }
        (x[0] - 2.0f64.cos()).abs()
    }

    #[test]
    fn fourth_order_convergence() {
        let e1 = integrate(0.1);
        let e2 = integrate(0.05);
        let order = (e1 / e2).log2();
        assert!(order > 3.8 && order < 4.2, "observed order {order}");
    }

    #[test]
    fn exponential_decay_accuracy() {
        let mut x = [1.0];
        for i in 0..100 {
            x = rk4_step(&x, i as f64 * 0.01, 0.01, |_, s| [-s[0]]);
        }
        assert!((x[0] - (-1.0f64).exp()).abs() < 1e-9);
    }
}
