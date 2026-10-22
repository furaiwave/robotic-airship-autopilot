//! Measurement model: what the onboard autopilot can observe.
//!
//! Each measured quantity equals the simulated truth plus optional zero-mean white
//! Gaussian noise drawn from a seeded generator that is independent of the
//! turbulence generator. The wind estimate is formed the way an onboard system would
//! form it, as measured ground velocity minus measured air-relative velocity, so it
//! inherits the noise of both sensors.

use airship_model::math::wrap_pi;
use airship_model::{Mat3, Vec3};
use serde::{Deserialize, Serialize};

use crate::environment::SensorNoiseConfig;
use crate::rng::SplitMix64;
use crate::state::VehicleState;

/// Truth quantities passed to the sensor model in addition to the vehicle state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TruthSnapshot {
    pub time_s: f64,
    pub state: VehicleState,
    pub wind: Vec3,
    pub ground_elevation_m: f64,
    pub gear_height_m: f64,
    pub air_density: f64,
    pub ground_normal_n: f64,
    pub moored: bool,
    /// Vertical force on the mooring mast, positive when the vehicle pulls upward (N).
    pub mooring_load_n: f64,
    pub battery_soc: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Measurement {
    pub time_s: f64,
    /// Position (m, ENU; z is MSL altitude of the CV).
    pub position: Vec3,
    /// Height of the gear contact point above ground (m).
    pub altitude_agl_m: f64,
    /// Ground velocity (m/s, ENU).
    pub velocity: Vec3,
    pub yaw: f64,
    pub yaw_rate: f64,
    pub pitch: f64,
    pub roll: f64,
    /// Air-relative velocity in the body frame from the air-data sensor (m/s).
    pub air_velocity_body: Vec3,
    /// Wind estimate (m/s, ENU).
    pub wind_estimate: Vec3,
    pub air_density: f64,
    /// Weight-on-gear switch (normal force above 2 N).
    pub ground_contact: bool,
    pub moored: bool,
    pub mooring_load_n: f64,
    pub battery_soc: f64,
}

#[derive(Debug, Clone)]
pub struct SensorModel {
    cfg: SensorNoiseConfig,
    rng: SplitMix64,
}

impl SensorModel {
    pub fn new(cfg: &SensorNoiseConfig) -> Self {
        Self {
            cfg: cfg.clone(),
            rng: SplitMix64::new(cfg.seed ^ 0x5345_4E53_4F52_5321),
        }
    }

    fn noise(&mut self, sigma: f64) -> f64 {
        if self.cfg.enabled && sigma > 0.0 {
            sigma * self.rng.normal()
        } else {
            0.0
        }
    }

    pub fn measure(&mut self, t: &TruthSnapshot) -> Measurement {
        let s = &t.state;
        let c = self.cfg.clone();
        let position = s.position
            + Vec3::new(
                self.noise(c.position_m),
                self.noise(c.position_m),
                self.noise(c.altitude_m),
            );
        let velocity = s.velocity
            + Vec3::new(
                self.noise(c.velocity_m_s),
                self.noise(c.velocity_m_s),
                self.noise(c.velocity_m_s),
            );
        let yaw = wrap_pi(s.yaw + self.noise(c.heading_deg.to_radians()));
        let r_bw = Mat3::body_to_world(s.yaw, s.pitch, s.roll);
        let air_true = r_bw.transpose().mul_vec(s.velocity - t.wind);
        let air = air_true
            + Vec3::new(
                self.noise(c.airspeed_m_s),
                self.noise(c.airspeed_m_s),
                self.noise(c.airspeed_m_s),
            );
        // Wind estimate: measured ground velocity minus measured air velocity rotated to world.
        let r_meas = Mat3::body_to_world(yaw, s.pitch, s.roll);
        let wind_estimate = velocity - r_meas.mul_vec(air);
        Measurement {
            time_s: t.time_s,
            position,
            altitude_agl_m: position.z - t.gear_height_m - t.ground_elevation_m,
            velocity,
            yaw,
            yaw_rate: s.yaw_rate,
            pitch: s.pitch,
            roll: s.roll,
            air_velocity_body: air,
            wind_estimate,
            air_density: t.air_density,
            ground_contact: t.ground_normal_n > 2.0,
            moored: t.moored,
            mooring_load_n: t.mooring_load_n,
            battery_soc: t.battery_soc,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn truth() -> TruthSnapshot {
        TruthSnapshot {
            time_s: 1.0,
            state: VehicleState {
                position: Vec3::new(10.0, 20.0, 150.0),
                velocity: Vec3::new(3.0, 0.0, 0.5),
                yaw: 0.3,
                ..Default::default()
            },
            wind: Vec3::new(-2.0, 1.0, 0.0),
            ground_elevation_m: 100.0,
            gear_height_m: 2.0,
            air_density: 1.2,
            ground_normal_n: 0.0,
            moored: false,
            mooring_load_n: 0.0,
            battery_soc: 0.9,
        }
    }

    #[test]
    fn noiseless_measurement_equals_truth() {
        let mut s = SensorModel::new(&SensorNoiseConfig::default());
        let m = s.measure(&truth());
        assert_eq!(m.position, truth().state.position);
        assert!((m.altitude_agl_m - 48.0).abs() < 1e-12);
        assert!(
            (m.wind_estimate - truth().wind).norm() < 1e-12,
            "wind estimate recovers the true wind"
        );
        assert!(!m.ground_contact);
    }

    #[test]
    fn noise_is_seeded() {
        let cfg = SensorNoiseConfig {
            enabled: true,
            ..Default::default()
        };
        let mut a = SensorModel::new(&cfg);
        let mut b = SensorModel::new(&cfg);
        let ma = a.measure(&truth());
        assert_eq!(ma, b.measure(&truth()));
        assert_ne!(ma.position, truth().state.position);
    }
}
