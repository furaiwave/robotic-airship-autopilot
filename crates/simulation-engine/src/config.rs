//! Complete, serialisable description of one simulation run.

use airship_model::{AirshipParams, DerivedProperties, ValidationReport, Vec2};
use autopilot::{AutopilotConfig, FlightPath, Waypoint, validate_flight_path};
use control_algorithms::ControllerConfig;
use flight_dynamics::EnvironmentConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulationSettings {
    /// Simulated duration limit (s).
    pub duration_s: f64,
    /// Fixed physics integration step (s).
    pub dt_s: f64,
    /// Autopilot / controller update rate (Hz). The control period must be a multiple of `dt_s`.
    pub control_rate_hz: f64,
    /// Telemetry recording rate (Hz). The recording period must be a multiple of `dt_s`.
    pub telemetry_rate_hz: f64,
}

impl Default for SimulationSettings {
    fn default() -> Self {
        Self {
            duration_s: 900.0,
            dt_s: 0.01,
            control_rate_hz: 20.0,
            telemetry_rate_hz: 10.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulationConfig {
    pub name: String,
    pub description: String,
    pub airship: AirshipParams,
    pub environment: EnvironmentConfig,
    pub flight_path: FlightPath,
    pub autopilot: AutopilotConfig,
    pub controller: ControllerConfig,
    pub simulation: SimulationSettings,
}

/// Default en-route path of the reference mission.
pub fn default_flight_path() -> FlightPath {
    let wp = |x: f64, y: f64, alt: f64, speed: f64| Waypoint {
        x_m: x,
        y_m: y,
        altitude_m: alt,
        heading_deg: None,
        speed_m_s: speed,
        tolerance_m: 15.0,
    };
    FlightPath {
        name: "Reference mission".into(),
        waypoints: vec![
            wp(150.0, 120.0, 40.0, 5.0),
            wp(450.0, 180.0, 40.0, 5.0),
            wp(720.0, 40.0, 30.0, 4.5),
        ],
    }
}

impl Default for SimulationConfig {
    fn default() -> Self {
        let mut environment = EnvironmentConfig::default();
        environment.landing_site = Vec2::new(400.0, -100.0);
        environment.landing_area.heading_deg = 270.0;
        Self {
            name: "Reference mission".into(),
            description: "Vertical take-off from a mooring mast, en-route waypoints, approach from the East and vertical landing.".into(),
            airship: AirshipParams::default(),
            environment,
            flight_path: default_flight_path(),
            autopilot: AutopilotConfig::default(),
            controller: ControllerConfig::default(),
            simulation: SimulationSettings::default(),
        }
    }
}

fn is_multiple(period: f64, dt: f64) -> bool {
    let n = period / dt;
    (n - n.round()).abs() < 1e-6 && n.round() >= 1.0
}

impl SimulationConfig {
    pub fn derived(&self) -> DerivedProperties {
        DerivedProperties::compute(&self.airship, self.environment.ground_density())
    }

    /// Full validation: ranges of every section plus physical and cross-section consistency.
    pub fn validate(&self) -> ValidationReport {
        let mut r = ValidationReport::new();
        if self.name.trim().is_empty() {
            r.error("name", "name must not be empty");
        }
        let airship = self.airship.validate();
        let airship_ok = !airship.has_errors();
        r.merge("airship", airship);
        r.merge("environment", self.environment.validate());
        if airship_ok {
            r.merge("airship", self.derived().validate_physics(&self.airship));
            r.merge("autopilot", self.autopilot.validate(&self.airship));
            r.merge(
                "flight_path",
                validate_flight_path(
                    &self.flight_path,
                    self.environment.takeoff_site,
                    &self.airship,
                    &self.autopilot,
                ),
            );
        }
        r.merge("controller", self.controller.validate());

        let s = &self.simulation;
        let dt_ok = r.range("simulation.dt_s", s.dt_s, 0.001, 0.05, "s");
        r.range("simulation.duration_s", s.duration_s, 1.0, 7200.0, "s");
        let c_ok = r.range(
            "simulation.control_rate_hz",
            s.control_rate_hz,
            1.0,
            200.0,
            "Hz",
        );
        let t_ok = r.range(
            "simulation.telemetry_rate_hz",
            s.telemetry_rate_hz,
            0.5,
            100.0,
            "Hz",
        );
        if dt_ok && c_ok && !is_multiple(1.0 / s.control_rate_hz, s.dt_s) {
            r.error(
                "simulation.control_rate_hz",
                "the control period must be an integer multiple of dt_s",
            );
        }
        if dt_ok && t_ok && !is_multiple(1.0 / s.telemetry_rate_hz, s.dt_s) {
            r.error(
                "simulation.telemetry_rate_hz",
                "the telemetry period must be an integer multiple of dt_s",
            );
        }
        if dt_ok && s.dt_s > 0.02 {
            r.warning("simulation.dt_s", "time steps above 0.02 s reduce the accuracy of the ground-contact and actuator dynamics");
        }
        if dt_ok && s.duration_s / s.dt_s > 2.0e6 {
            r.error(
                "simulation.duration_s",
                "more than 2 million integration steps requested",
            );
        }
        let e = &self.environment;
        if !e.landing_area.contains(e.landing_site, e.landing_site) {
            r.error(
                "environment.landing_area",
                "the landing point must lie inside the landing area",
            );
        }
        if (e.landing_site - e.takeoff_site).norm() < 1.0 && !e.initial_state.moored {
            r.warning(
                "environment.landing_site",
                "landing site coincides with the take-off site",
            );
        }
        r
    }
}

/// FNV-1a 64-bit hash of the canonical JSON serialisation (for reproducibility records).
pub fn config_hash(cfg: &SimulationConfig) -> String {
    let json = serde_json::to_string(cfg).unwrap_or_default();
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in json.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{h:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        let r = SimulationConfig::default().validate();
        assert!(!r.has_errors(), "{:#?}", r.issues);
    }

    #[test]
    fn rejects_invalid_time_steps() {
        let mut c = SimulationConfig::default();
        c.simulation.dt_s = 0.0;
        assert!(c.validate().has_errors());
        c.simulation.dt_s = f64::NAN;
        assert!(c.validate().has_errors());
        let mut c = SimulationConfig::default();
        c.simulation.dt_s = 0.03; // 20 Hz control period is not a multiple of 0.03 s
        assert!(
            c.validate()
                .errors()
                .any(|i| i.field == "simulation.control_rate_hz")
        );
    }

    #[test]
    fn json_roundtrip_and_hash_stability() {
        let c = SimulationConfig::default();
        let json = serde_json::to_string(&c).unwrap();
        let back: SimulationConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back, c);
        assert_eq!(config_hash(&back), config_hash(&c));
        let mut d = c.clone();
        d.airship.total_mass_kg += 0.1;
        assert_ne!(config_hash(&d), config_hash(&c));
    }
}
