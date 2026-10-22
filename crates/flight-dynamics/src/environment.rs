//! Simulated environment: terrain, sites, initial conditions and the wind field.
//!
//! The wind model is a *test-harness disturbance model*, not a validated
//! meteorological model. The total wind at time `t` and height `h` above ground is
//!
//! ```text
//! w(t, h) = s(h) * w_mean,h(t) + w_mean,z(t) + w_gust(t) + w_turb(t)
//! s(h)    = (max(h, 1 m) / h_ref)^alpha            (power-law shear, alpha = 0 disables it)
//! ```
//!
//! * `w_mean(t)` starts at the base wind and is changed by scheduled **wind changes**,
//!   each ramping linearly from the previous mean to its target over `ramp_duration_s`.
//! * `w_gust(t)` is a sum of discrete **1-cosine gusts** `A/2 (1 - cos(2 pi tau / T))`.
//! * `w_turb(t)` is first-order Gauss-Markov (Ornstein-Uhlenbeck) turbulence per axis,
//!   discretised exactly: `x_k+1 = a x_k + sigma sqrt(1 - a^2) n_k`, `a = exp(-dt/T_c)`,
//!   with `n_k` drawn from a seeded generator, so runs are reproducible.
//!
//! Wind changes and gusts are activated by a trigger: either a simulation time or the
//! first moment the vehicle comes within a horizontal distance of the landing point.
//! Distance triggers inject the same disturbance at the same point of the manoeuvre
//! for every controller, which makes controller comparisons fair.

use airship_model::math::{heading_deg_to_yaw, lerp, wind_vector_from};
use airship_model::{AtmosphereConfig, ValidationReport, Vec2, Vec3};
use serde::{Deserialize, Serialize};

use crate::rng::SplitMix64;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DisturbanceTrigger {
    /// Activate at simulation time `time_s`.
    AtTime { time_s: f64 },
    /// Activate when the horizontal distance to the landing point first drops below `distance_m`.
    DistanceToLanding { distance_m: f64 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindChange {
    pub trigger: DisturbanceTrigger,
    pub ramp_duration_s: f64,
    /// New mean horizontal wind speed at the shear reference height (m/s).
    pub speed_m_s: f64,
    /// New "blowing from" compass direction (deg).
    pub from_direction_deg: f64,
    /// New mean vertical wind, positive up (m/s).
    pub vertical_m_s: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiscreteGust {
    pub trigger: DisturbanceTrigger,
    pub duration_s: f64,
    /// Peak horizontal gust speed (m/s).
    pub speed_m_s: f64,
    /// Compass direction the gust blows *from* (deg).
    pub from_direction_deg: f64,
    /// Peak vertical gust speed, positive up (m/s).
    pub vertical_m_s: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TurbulenceConfig {
    pub enabled: bool,
    /// Standard deviation of each horizontal turbulence component (m/s).
    pub intensity_m_s: f64,
    /// Vertical intensity as a fraction of the horizontal intensity (-).
    pub vertical_ratio: f64,
    /// Correlation time `T_c` of the Gauss-Markov process (s).
    pub correlation_time_s: f64,
    pub seed: u64,
}

impl Default for TurbulenceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            intensity_m_s: 0.8,
            vertical_ratio: 0.5,
            correlation_time_s: 4.0,
            seed: 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindConfig {
    /// Mean horizontal wind speed at the shear reference height (m/s).
    pub speed_m_s: f64,
    /// Compass direction the wind blows *from* (deg).
    pub from_direction_deg: f64,
    /// Mean vertical wind, positive up (m/s).
    pub vertical_m_s: f64,
    /// Power-law shear exponent `alpha` (0 = uniform wind).
    pub shear_exponent: f64,
    /// Height above ground at which `speed_m_s` applies (m).
    pub shear_reference_height_m: f64,
    pub changes: Vec<WindChange>,
    pub gusts: Vec<DiscreteGust>,
    pub turbulence: TurbulenceConfig,
}

impl Default for WindConfig {
    fn default() -> Self {
        Self {
            speed_m_s: 0.0,
            from_direction_deg: 270.0,
            vertical_m_s: 0.0,
            shear_exponent: 0.0,
            shear_reference_height_m: 10.0,
            changes: Vec::new(),
            gusts: Vec::new(),
            turbulence: TurbulenceConfig::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SensorNoiseConfig {
    pub enabled: bool,
    pub seed: u64,
    /// Horizontal position noise std (m).
    pub position_m: f64,
    /// Altitude noise std (m).
    pub altitude_m: f64,
    /// Ground-velocity noise std per axis (m/s).
    pub velocity_m_s: f64,
    /// Heading noise std (deg).
    pub heading_deg: f64,
    /// Air-data (relative velocity) noise std per axis (m/s).
    pub airspeed_m_s: f64,
}

impl Default for SensorNoiseConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            seed: 2,
            position_m: 0.5,
            altitude_m: 0.3,
            velocity_m_s: 0.1,
            heading_deg: 1.0,
            airspeed_m_s: 0.2,
        }
    }
}

/// Prepared landing surface: a rectangle aligned with `heading_deg` around the landing point.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LandingArea {
    /// Direction of the landing axis (compass deg); the default final approach flies along it.
    pub heading_deg: f64,
    /// Usable length before the landing point (m).
    pub undershoot_m: f64,
    /// Usable length beyond the landing point (m).
    pub overrun_m: f64,
    /// Half of the usable width (m).
    pub half_width_m: f64,
}

impl Default for LandingArea {
    fn default() -> Self {
        Self {
            heading_deg: 270.0,
            undershoot_m: 30.0,
            overrun_m: 30.0,
            half_width_m: 20.0,
        }
    }
}

impl LandingArea {
    /// Along-axis and cross-axis coordinates of `p` relative to the landing point `c`.
    pub fn local(&self, c: Vec2, p: Vec2) -> (f64, f64) {
        let u = Vec2::from_angle(heading_deg_to_yaw(self.heading_deg));
        let d = p - c;
        (d.dot(u), u.cross(d))
    }

    pub fn contains(&self, c: Vec2, p: Vec2) -> bool {
        let (along, cross) = self.local(c, p);
        along >= -self.undershoot_m && along <= self.overrun_m && cross.abs() <= self.half_width_m
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InitialStateConfig {
    /// Horizontal position (m, East/North).
    pub position: Vec2,
    /// Height of the gear contact point above ground (m); 0 means resting on the ground.
    pub altitude_agl_m: f64,
    /// Initial ground velocity (m/s, ENU).
    pub velocity: Vec3,
    pub heading_deg: f64,
    pub pitch_deg: f64,
    pub roll_deg: f64,
    /// Start attached to the mooring mast (requires `altitude_agl_m == 0` and zero velocity).
    pub moored: bool,
}

impl Default for InitialStateConfig {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            altitude_agl_m: 0.0,
            velocity: Vec3::ZERO,
            heading_deg: 270.0,
            pitch_deg: 0.0,
            roll_deg: 0.0,
            moored: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnvironmentConfig {
    pub atmosphere: AtmosphereConfig,
    /// Elevation of the (flat) terrain above mean sea level (m).
    pub ground_elevation_m: f64,
    pub takeoff_site: Vec2,
    pub landing_site: Vec2,
    pub landing_area: LandingArea,
    pub initial_state: InitialStateConfig,
    pub wind: WindConfig,
    pub sensor_noise: SensorNoiseConfig,
}

impl Default for EnvironmentConfig {
    fn default() -> Self {
        Self {
            atmosphere: AtmosphereConfig::default(),
            ground_elevation_m: 120.0,
            takeoff_site: Vec2::ZERO,
            landing_site: Vec2::new(-200.0, 150.0),
            landing_area: LandingArea::default(),
            initial_state: InitialStateConfig::default(),
            wind: WindConfig::default(),
            sensor_noise: SensorNoiseConfig::default(),
        }
    }
}

impl EnvironmentConfig {
    /// Air density at ground level (kg/m^3).
    pub fn ground_density(&self) -> f64 {
        self.atmosphere.density(self.ground_elevation_m)
    }

    pub fn validate(&self) -> ValidationReport {
        let mut r = ValidationReport::new();
        r.merge("atmosphere", self.atmosphere.validate());
        r.range(
            "ground_elevation_m",
            self.ground_elevation_m,
            -400.0,
            5000.0,
            "m",
        );
        for (name, p) in [
            ("takeoff_site", self.takeoff_site),
            ("landing_site", self.landing_site),
        ] {
            if !p.is_finite() || p.x.abs() > 1.0e5 || p.y.abs() > 1.0e5 {
                r.error(name, "coordinates must be finite and within +/-100 km");
            }
        }
        let la = &self.landing_area;
        r.range(
            "landing_area.heading_deg",
            la.heading_deg,
            -360.0,
            720.0,
            "deg",
        );
        r.range(
            "landing_area.undershoot_m",
            la.undershoot_m,
            1.0,
            5000.0,
            "m",
        );
        r.range("landing_area.overrun_m", la.overrun_m, 1.0, 5000.0, "m");
        r.range(
            "landing_area.half_width_m",
            la.half_width_m,
            1.0,
            5000.0,
            "m",
        );

        let s = &self.initial_state;
        if !s.position.is_finite() {
            r.error("initial_state.position", "must be finite");
        }
        r.range(
            "initial_state.altitude_agl_m",
            s.altitude_agl_m,
            0.0,
            3000.0,
            "m",
        );
        if !s.velocity.is_finite() || s.velocity.norm() > 30.0 {
            r.error(
                "initial_state.velocity",
                "speed must be finite and below 30 m/s",
            );
        }
        r.range(
            "initial_state.heading_deg",
            s.heading_deg,
            -360.0,
            720.0,
            "deg",
        );
        r.range("initial_state.pitch_deg", s.pitch_deg, -30.0, 30.0, "deg");
        r.range("initial_state.roll_deg", s.roll_deg, -30.0, 30.0, "deg");
        if s.moored && (s.altitude_agl_m > 0.0 || s.velocity.norm() > 0.0) {
            r.error(
                "initial_state.moored",
                "a moored start requires altitude_agl_m = 0 and zero velocity",
            );
        }
        if s.moored && (s.position - self.takeoff_site).norm() > 1.0 {
            r.warning(
                "initial_state.position",
                "moored start is not at the take-off site",
            );
        }

        let w = &self.wind;
        r.range("wind.speed_m_s", w.speed_m_s, 0.0, 30.0, "m/s");
        r.range(
            "wind.from_direction_deg",
            w.from_direction_deg,
            -360.0,
            720.0,
            "deg",
        );
        r.range("wind.vertical_m_s", w.vertical_m_s, -5.0, 5.0, "m/s");
        r.range("wind.shear_exponent", w.shear_exponent, 0.0, 0.6, "-");
        r.range(
            "wind.shear_reference_height_m",
            w.shear_reference_height_m,
            1.0,
            500.0,
            "m",
        );
        for (i, c) in w.changes.iter().enumerate() {
            let f = format!("wind.changes[{i}]");
            validate_trigger(&mut r, &format!("{f}.trigger"), &c.trigger);
            r.range(
                &format!("{f}.ramp_duration_s"),
                c.ramp_duration_s,
                0.0,
                3600.0,
                "s",
            );
            r.range(&format!("{f}.speed_m_s"), c.speed_m_s, 0.0, 30.0, "m/s");
            r.range(
                &format!("{f}.from_direction_deg"),
                c.from_direction_deg,
                -360.0,
                720.0,
                "deg",
            );
            r.range(
                &format!("{f}.vertical_m_s"),
                c.vertical_m_s,
                -5.0,
                5.0,
                "m/s",
            );
        }
        for (i, g) in w.gusts.iter().enumerate() {
            let f = format!("wind.gusts[{i}]");
            validate_trigger(&mut r, &format!("{f}.trigger"), &g.trigger);
            r.range(&format!("{f}.duration_s"), g.duration_s, 0.5, 600.0, "s");
            r.range(&format!("{f}.speed_m_s"), g.speed_m_s, 0.0, 25.0, "m/s");
            r.range(
                &format!("{f}.from_direction_deg"),
                g.from_direction_deg,
                -360.0,
                720.0,
                "deg",
            );
            r.range(
                &format!("{f}.vertical_m_s"),
                g.vertical_m_s,
                -5.0,
                5.0,
                "m/s",
            );
        }
        let t = &w.turbulence;
        r.range(
            "wind.turbulence.intensity_m_s",
            t.intensity_m_s,
            0.0,
            10.0,
            "m/s",
        );
        r.range(
            "wind.turbulence.vertical_ratio",
            t.vertical_ratio,
            0.0,
            2.0,
            "-",
        );
        r.range(
            "wind.turbulence.correlation_time_s",
            t.correlation_time_s,
            0.1,
            600.0,
            "s",
        );

        let n = &self.sensor_noise;
        r.range("sensor_noise.position_m", n.position_m, 0.0, 20.0, "m");
        r.range("sensor_noise.altitude_m", n.altitude_m, 0.0, 20.0, "m");
        r.range("sensor_noise.velocity_m_s", n.velocity_m_s, 0.0, 5.0, "m/s");
        r.range("sensor_noise.heading_deg", n.heading_deg, 0.0, 30.0, "deg");
        r.range("sensor_noise.airspeed_m_s", n.airspeed_m_s, 0.0, 5.0, "m/s");
        r
    }
}

fn validate_trigger(r: &mut ValidationReport, field: &str, t: &DisturbanceTrigger) {
    match *t {
        DisturbanceTrigger::AtTime { time_s } => {
            r.range(&format!("{field}.time_s"), time_s, 0.0, 86_400.0, "s");
        }
        DisturbanceTrigger::DistanceToLanding { distance_m } => {
            r.range(&format!("{field}.distance_m"), distance_m, 0.0, 1.0e5, "m");
        }
    }
}

/// Runtime wind field with trigger bookkeeping and turbulence state.
#[derive(Debug, Clone)]
pub struct WindField {
    cfg: WindConfig,
    landing_site: Vec2,
    change_fired_at: Vec<Option<f64>>,
    gust_fired_at: Vec<Option<f64>>,
    turbulence: Vec3,
    rng: SplitMix64,
}

/// One activated disturbance, reported to telemetry/event logs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DisturbanceActivation {
    pub time_s: f64,
    pub description: String,
}

impl WindField {
    pub fn new(cfg: &WindConfig, landing_site: Vec2) -> Self {
        let mut rng = SplitMix64::new(cfg.turbulence.seed ^ 0x5755_5242_554C_454E);
        let t = &cfg.turbulence;
        // Start the Gauss-Markov process in its stationary distribution.
        let turbulence = if t.enabled {
            Vec3::new(
                t.intensity_m_s * rng.normal(),
                t.intensity_m_s * rng.normal(),
                t.intensity_m_s * t.vertical_ratio * rng.normal(),
            )
        } else {
            Vec3::ZERO
        };
        Self {
            change_fired_at: vec![None; cfg.changes.len()],
            gust_fired_at: vec![None; cfg.gusts.len()],
            cfg: cfg.clone(),
            landing_site,
            turbulence,
            rng,
        }
    }

    /// Evaluate triggers at time `t` for the vehicle at `vehicle_xy`, then advance the
    /// turbulence process by `dt`. Returns disturbances activated in this call.
    pub fn advance(&mut self, t: f64, dt: f64, vehicle_xy: Vec2) -> Vec<DisturbanceActivation> {
        let dist = (vehicle_xy - self.landing_site).norm();
        let mut activated = Vec::new();
        let fires = |trig: &DisturbanceTrigger| match *trig {
            DisturbanceTrigger::AtTime { time_s } => t >= time_s,
            DisturbanceTrigger::DistanceToLanding { distance_m } => dist <= distance_m,
        };
        for (i, c) in self.cfg.changes.iter().enumerate() {
            if self.change_fired_at[i].is_none() && fires(&c.trigger) {
                self.change_fired_at[i] = Some(t);
                activated.push(DisturbanceActivation {
                    time_s: t,
                    description: format!(
                        "wind change #{}: ramp to {:.1} m/s from {:.0} deg over {:.0} s",
                        i + 1,
                        c.speed_m_s,
                        c.from_direction_deg,
                        c.ramp_duration_s
                    ),
                });
            }
        }
        for (i, g) in self.cfg.gusts.iter().enumerate() {
            if self.gust_fired_at[i].is_none() && fires(&g.trigger) {
                self.gust_fired_at[i] = Some(t);
                activated.push(DisturbanceActivation {
                    time_s: t,
                    description: format!(
                        "1-cosine gust #{}: peak {:.1} m/s from {:.0} deg, duration {:.0} s",
                        i + 1,
                        g.speed_m_s,
                        g.from_direction_deg,
                        g.duration_s
                    ),
                });
            }
        }
        let tc = &self.cfg.turbulence;
        if tc.enabled && dt > 0.0 {
            let a = (-dt / tc.correlation_time_s).exp();
            let b = (1.0 - a * a).sqrt();
            let sh = tc.intensity_m_s;
            let sv = tc.intensity_m_s * tc.vertical_ratio;
            self.turbulence = Vec3::new(
                a * self.turbulence.x + sh * b * self.rng.normal(),
                a * self.turbulence.y + sh * b * self.rng.normal(),
                a * self.turbulence.z + sv * b * self.rng.normal(),
            );
        }
        activated
    }

    /// Mean wind (horizontal at the reference height, vertical) at time `t`.
    fn mean(&self, t: f64) -> (Vec2, f64) {
        let mut h = wind_vector_from(self.cfg.speed_m_s, self.cfg.from_direction_deg);
        let mut v = self.cfg.vertical_m_s;
        let mut fired: Vec<(f64, usize)> = self
            .change_fired_at
            .iter()
            .enumerate()
            .filter_map(|(i, f)| f.map(|t0| (t0, i)))
            .collect();
        fired.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        for (t0, i) in fired {
            let c = &self.cfg.changes[i];
            let s = if c.ramp_duration_s <= 0.0 {
                1.0
            } else {
                ((t - t0) / c.ramp_duration_s).clamp(0.0, 1.0)
            };
            let target = wind_vector_from(c.speed_m_s, c.from_direction_deg);
            h = Vec2::new(lerp(h.x, target.x, s), lerp(h.y, target.y, s));
            v = lerp(v, c.vertical_m_s, s);
        }
        (h, v)
    }

    fn gusts(&self, t: f64) -> Vec3 {
        let mut sum = Vec3::ZERO;
        for (i, g) in self.cfg.gusts.iter().enumerate() {
            if let Some(t0) = self.gust_fired_at[i] {
                let tau = (t - t0) / g.duration_s;
                if (0.0..=1.0).contains(&tau) {
                    let shape = 0.5 * (1.0 - (std::f64::consts::TAU * tau).cos());
                    let hw = wind_vector_from(g.speed_m_s, g.from_direction_deg);
                    sum += Vec3::new(hw.x, hw.y, g.vertical_m_s) * shape;
                }
            }
        }
        sum
    }

    /// Total wind velocity (m/s, ENU) at time `t` and height `agl_m` above ground.
    pub fn wind_at(&self, t: f64, agl_m: f64) -> Vec3 {
        let (h, v) = self.mean(t);
        let shear = if self.cfg.shear_exponent > 0.0 {
            (agl_m.max(1.0) / self.cfg.shear_reference_height_m).powf(self.cfg.shear_exponent)
        } else {
            1.0
        };
        Vec3::new(h.x * shear, h.y * shear, v) + self.gusts(t) + self.turbulence
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calm_air_is_zero() {
        let mut w = WindField::new(&WindConfig::default(), Vec2::ZERO);
        w.advance(0.0, 0.01, Vec2::new(500.0, 0.0));
        assert_eq!(w.wind_at(0.0, 10.0), Vec3::ZERO);
    }

    #[test]
    fn constant_wind_direction() {
        let cfg = WindConfig {
            speed_m_s: 4.0,
            from_direction_deg: 90.0,
            ..Default::default()
        };
        let w = WindField::new(&cfg, Vec2::ZERO);
        let v = w.wind_at(0.0, 10.0);
        // Easterly wind blows toward the West.
        assert!((v.x + 4.0).abs() < 1e-12 && v.y.abs() < 1e-12);
    }

    #[test]
    fn wind_change_ramps_linearly() {
        let cfg = WindConfig {
            speed_m_s: 0.0,
            changes: vec![WindChange {
                trigger: DisturbanceTrigger::AtTime { time_s: 10.0 },
                ramp_duration_s: 10.0,
                speed_m_s: 6.0,
                from_direction_deg: 0.0,
                vertical_m_s: 0.0,
            }],
            ..Default::default()
        };
        let mut w = WindField::new(&cfg, Vec2::ZERO);
        w.advance(5.0, 0.01, Vec2::new(1000.0, 0.0));
        assert_eq!(w.wind_at(5.0, 10.0).norm(), 0.0);
        let ev = w.advance(10.0, 0.01, Vec2::new(1000.0, 0.0));
        assert_eq!(ev.len(), 1);
        assert!((w.wind_at(15.0, 10.0).norm() - 3.0).abs() < 1e-9);
        assert!((w.wind_at(30.0, 10.0).norm() - 6.0).abs() < 1e-9);
    }

    #[test]
    fn distance_triggered_gust_is_one_minus_cosine() {
        let cfg = WindConfig {
            gusts: vec![DiscreteGust {
                trigger: DisturbanceTrigger::DistanceToLanding { distance_m: 100.0 },
                duration_s: 10.0,
                speed_m_s: 5.0,
                from_direction_deg: 180.0,
                vertical_m_s: 0.0,
            }],
            ..Default::default()
        };
        let mut w = WindField::new(&cfg, Vec2::ZERO);
        assert!(w.advance(0.0, 0.01, Vec2::new(150.0, 0.0)).is_empty());
        assert_eq!(w.advance(3.0, 0.01, Vec2::new(90.0, 0.0)).len(), 1);
        assert!(
            (w.wind_at(8.0, 10.0).y - 5.0).abs() < 1e-9,
            "peak at mid-duration"
        );
        assert!(w.wind_at(13.0, 10.0).norm() < 1e-9, "zero after the gust");
        // A trigger fires only once.
        assert!(w.advance(20.0, 0.01, Vec2::new(10.0, 0.0)).is_empty());
    }

    #[test]
    fn shear_increases_with_height() {
        let cfg = WindConfig {
            speed_m_s: 5.0,
            shear_exponent: 0.15,
            ..Default::default()
        };
        let w = WindField::new(&cfg, Vec2::ZERO);
        assert!(w.wind_at(0.0, 50.0).norm() > w.wind_at(0.0, 10.0).norm());
        assert!((w.wind_at(0.0, 10.0).norm() - 5.0).abs() < 1e-9);
    }

    #[test]
    fn turbulence_is_reproducible_and_has_expected_intensity() {
        let cfg = WindConfig {
            turbulence: TurbulenceConfig {
                enabled: true,
                intensity_m_s: 1.0,
                seed: 99,
                ..Default::default()
            },
            ..Default::default()
        };
        let run = || {
            let mut w = WindField::new(&cfg, Vec2::ZERO);
            (0..50_000)
                .map(|k| {
                    w.advance(k as f64 * 0.05, 0.05, Vec2::ZERO);
                    w.wind_at(k as f64 * 0.05, 10.0).x
                })
                .collect::<Vec<_>>()
        };
        let a = run();
        assert_eq!(a, run());
        let var = a.iter().map(|x| x * x).sum::<f64>() / a.len() as f64;
        assert!((var.sqrt() - 1.0).abs() < 0.1, "std {}", var.sqrt());
    }

    #[test]
    fn landing_area_containment() {
        let la = LandingArea {
            heading_deg: 90.0,
            undershoot_m: 10.0,
            overrun_m: 5.0,
            half_width_m: 3.0,
        };
        let c = Vec2::new(100.0, 0.0);
        assert!(la.contains(c, Vec2::new(95.0, 2.0)));
        assert!(!la.contains(c, Vec2::new(106.0, 0.0)), "beyond overrun");
        assert!(!la.contains(c, Vec2::new(100.0, 4.0)), "outside width");
    }

    #[test]
    fn default_environment_is_valid() {
        let r = EnvironmentConfig::default().validate();
        assert!(!r.has_errors(), "{:?}", r.issues);
    }
}
