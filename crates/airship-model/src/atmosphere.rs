//! Atmosphere model: constant density or the ISA troposphere (0-11 km).
//!
//! ISA troposphere (temperature lapse rate `L = 0.0065 K/m`):
//!
//! ```text
//! T(h)   = T0 + dT - L h
//! p(h)   = p0 (1 - L h / T0)^(g M / (R L))
//! rho(h) = p(h) / (R_specific T(h))
//! ```
//!
//! `dT` is an optional temperature offset from the standard day; it changes the
//! density through the ideal gas law but not the standard pressure profile.
//! The model is an *assumption*, not a measurement of a real environment.

use serde::{Deserialize, Serialize};

use crate::validation::ValidationReport;

/// Sea-level standard temperature (K).
pub const ISA_T0: f64 = 288.15;
/// Sea-level standard pressure (Pa).
pub const ISA_P0: f64 = 101_325.0;
/// Sea-level standard density (kg/m^3).
pub const ISA_RHO0: f64 = 1.225;
/// Tropospheric temperature lapse rate (K/m).
pub const ISA_LAPSE: f64 = 0.0065;
/// Specific gas constant of dry air (J/(kg K)).
pub const R_AIR: f64 = 287.052_87;
/// Exponent `g M / (R L)` of the ISA pressure law.
const ISA_EXP: f64 = 5.255_88;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AtmosphereModel {
    /// Uniform density `constant_density_kg_m3` at every altitude.
    Constant,
    /// International Standard Atmosphere troposphere with optional temperature offset.
    Isa,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtmosphereConfig {
    pub model: AtmosphereModel,
    /// Density used by the `constant` model (kg/m^3).
    pub constant_density_kg_m3: f64,
    /// Temperature offset from ISA used by the `isa` model (K).
    pub temperature_offset_k: f64,
}

impl Default for AtmosphereConfig {
    fn default() -> Self {
        Self {
            model: AtmosphereModel::Isa,
            constant_density_kg_m3: ISA_RHO0,
            temperature_offset_k: 0.0,
        }
    }
}

impl AtmosphereConfig {
    /// Air density (kg/m^3) at geometric altitude `alt_msl_m` above mean sea level.
    pub fn density(&self, alt_msl_m: f64) -> f64 {
        match self.model {
            AtmosphereModel::Constant => self.constant_density_kg_m3,
            AtmosphereModel::Isa => {
                let h = alt_msl_m.clamp(-500.0, 11_000.0);
                let p = ISA_P0 * (1.0 - ISA_LAPSE * h / ISA_T0).powf(ISA_EXP);
                let t = ISA_T0 + self.temperature_offset_k - ISA_LAPSE * h;
                p / (R_AIR * t)
            }
        }
    }

    /// Air temperature (K), informational.
    pub fn temperature(&self, alt_msl_m: f64) -> f64 {
        match self.model {
            AtmosphereModel::Constant => ISA_T0,
            AtmosphereModel::Isa => {
                ISA_T0 + self.temperature_offset_k - ISA_LAPSE * alt_msl_m.clamp(-500.0, 11_000.0)
            }
        }
    }

    pub fn validate(&self) -> ValidationReport {
        let mut r = ValidationReport::new();
        r.range(
            "constant_density_kg_m3",
            self.constant_density_kg_m3,
            0.3,
            1.5,
            "kg/m^3",
        );
        r.range(
            "temperature_offset_k",
            self.temperature_offset_k,
            -40.0,
            40.0,
            "K",
        );
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isa_sea_level_and_1000m() {
        let a = AtmosphereConfig::default();
        assert!((a.density(0.0) - 1.225).abs() < 1e-3);
        // Tabulated ISA density at 1000 m is 1.1117 kg/m^3.
        assert!((a.density(1000.0) - 1.1117).abs() < 1e-3);
        assert!(a.density(500.0) < a.density(0.0));
    }

    #[test]
    fn hot_day_is_less_dense() {
        let hot = AtmosphereConfig {
            temperature_offset_k: 20.0,
            ..Default::default()
        };
        assert!(hot.density(0.0) < AtmosphereConfig::default().density(0.0));
    }

    #[test]
    fn constant_model() {
        let a = AtmosphereConfig {
            model: AtmosphereModel::Constant,
            constant_density_kg_m3: 1.1,
            ..Default::default()
        };
        assert_eq!(a.density(0.0), 1.1);
        assert_eq!(a.density(3000.0), 1.1);
    }
}
