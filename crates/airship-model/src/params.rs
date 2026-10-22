//! User-configurable vehicle parameters.
//!
//! ## Selected airship configuration
//!
//! A small non-rigid research airship ("robotic blimp") with:
//!
//! * a prolate-ellipsoid helium envelope of length `L` and maximum diameter `D`;
//! * a gondola hanging below the envelope so that the centre of gravity (CG) lies a
//!   distance `cg_below_cv_m` (BG) below the centre of volume (CV). This produces
//!   passive pendulum stability in pitch and roll; **no elevator is modelled**;
//! * two reversible **main thrusters** on gondola outriggers at lateral arm `b` that
//!   produce surge force (sum) and yaw moment (difference);
//! * one reversible **heave thruster** acting along the body `z` axis through the CG,
//!   providing vertical control authority for vertical take-off and landing;
//! * an aerodynamic **rudder** on the vertical tail fins whose authority scales with
//!   dynamic pressure and therefore vanishes at hover;
//! * a landing gear (skid) whose contact point is `gear_height_m` below the CV.

use serde::{Deserialize, Serialize};

use crate::validation::ValidationReport;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiftingGas {
    Helium,
    Hydrogen,
}

impl LiftingGas {
    /// Gas density relative to air at the same temperature and pressure.
    pub fn density_ratio(self) -> f64 {
        match self {
            // Molar mass ratio 4.0026 / 28.9647
            LiftingGas::Helium => 0.138_19,
            // Molar mass ratio 2.016 / 28.9647
            LiftingGas::Hydrogen => 0.069_60,
        }
    }
}

/// Aerodynamic coefficients. Reference areas are *derived* from the hull geometry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AeroParams {
    /// Axial drag coefficient referenced to the frontal area `pi D^2 / 4` (-).
    pub cd_axial: f64,
    /// Lateral (cross-flow) drag coefficient referenced to the side area `pi L D / 4` (-).
    pub cd_lateral: f64,
    /// Vertical (cross-flow) drag coefficient referenced to the planform area `pi L D / 4` (-).
    pub cd_vertical: f64,
    /// Total effective vertical-fin area acting in yaw (m^2).
    pub fin_area_m2: f64,
    /// Fin lift-curve slope `dC_L/d(alpha)` (1/rad). Low aspect ratio fins: 2-3.
    pub fin_lift_slope_per_rad: f64,
    /// Distance from the CG to the fin centre of pressure along the hull axis (m).
    pub fin_arm_m: f64,
    /// Damping ratio of the passive pitch and roll pendulum modes (-).
    pub pitch_roll_damping_ratio: f64,
}

impl Default for AeroParams {
    fn default() -> Self {
        Self {
            cd_axial: 0.08,
            cd_lateral: 0.9,
            cd_vertical: 0.9,
            fin_area_m2: 2.0,
            fin_lift_slope_per_rad: 2.5,
            fin_arm_m: 5.0,
            pitch_roll_damping_ratio: 0.35,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropulsionParams {
    /// Maximum forward thrust of *each* main thruster (N).
    pub main_max_thrust_n: f64,
    /// Available reverse thrust as a fraction of the forward maximum (-).
    pub main_reverse_ratio: f64,
    /// Lateral distance of each main thruster from the hull centreline (m).
    pub main_lateral_arm_m: f64,
    /// Vertical distance of the main thrust line below the CG (m).
    pub main_vertical_offset_m: f64,
    /// Maximum upward thrust of the heave thruster (N).
    pub heave_max_thrust_n: f64,
    /// Available downward heave thrust as a fraction of the upward maximum (-).
    pub heave_reverse_ratio: f64,
    /// Propeller diameter used by the momentum-theory power estimate (m).
    pub propeller_diameter_m: f64,
    /// First-order time constant of thrust response (s).
    pub response_time_s: f64,
    /// Overall electrical-to-propulsive efficiency (-).
    pub efficiency: f64,
    /// Usable battery energy (Wh).
    pub battery_capacity_wh: f64,
    /// Constant avionics and payload power draw (W).
    pub avionics_power_w: f64,
}

impl Default for PropulsionParams {
    fn default() -> Self {
        Self {
            main_max_thrust_n: 60.0,
            main_reverse_ratio: 0.5,
            main_lateral_arm_m: 1.2,
            main_vertical_offset_m: 0.5,
            heave_max_thrust_n: 100.0,
            heave_reverse_ratio: 1.0,
            propeller_diameter_m: 0.6,
            response_time_s: 0.3,
            efficiency: 0.6,
            battery_capacity_wh: 1000.0,
            avionics_power_w: 25.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RudderParams {
    /// Mechanical deflection limit (deg).
    pub max_deflection_deg: f64,
    /// Servo slew-rate limit (deg/s).
    pub max_rate_deg_s: f64,
    /// Fin side-force coefficient per radian of rudder deflection `dC_L/d(delta)` (1/rad).
    pub effectiveness_per_rad: f64,
}

impl Default for RudderParams {
    fn default() -> Self {
        Self {
            max_deflection_deg: 25.0,
            max_rate_deg_s: 60.0,
            effectiveness_per_rad: 1.5,
        }
    }
}

/// Operating envelope used by the autopilot reference generation and the safety monitor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperatingLimits {
    pub max_climb_rate_m_s: f64,
    pub max_descent_rate_m_s: f64,
    /// Maximum horizontal airspeed (m/s).
    pub max_airspeed_m_s: f64,
    pub max_turn_rate_deg_s: f64,
    pub max_pitch_deg: f64,
    pub max_roll_deg: f64,
}

impl Default for OperatingLimits {
    fn default() -> Self {
        Self {
            max_climb_rate_m_s: 1.5,
            max_descent_rate_m_s: 1.2,
            max_airspeed_m_s: 10.0,
            max_turn_rate_deg_s: 10.0,
            max_pitch_deg: 20.0,
            max_roll_deg: 15.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AirshipParams {
    pub name: String,
    /// Envelope length `L` (m).
    pub length_m: f64,
    /// Maximum envelope diameter `D` (m).
    pub max_diameter_m: f64,
    /// Envelope volume (m^3). `None` derives it from a prolate ellipsoid `pi L D^2 / 6`.
    pub hull_volume_m3: Option<f64>,
    /// Additional displaced volume of gondola, fins and equipment (m^3).
    pub gondola_volume_m3: f64,
    /// Total mass including structure, lifting gas, payload and battery (kg).
    pub total_mass_kg: f64,
    pub lifting_gas: LiftingGas,
    /// Distance of the CG below the CV, `BG` (m). Must be positive (pendulum stability).
    pub cg_below_cv_m: f64,
    /// Distance from the CV down to the landing-gear contact point (m).
    pub gear_height_m: f64,
    pub aero: AeroParams,
    pub propulsion: PropulsionParams,
    pub rudder: RudderParams,
    pub limits: OperatingLimits,
}

impl Default for AirshipParams {
    fn default() -> Self {
        Self {
            name: "RA-12 research blimp".into(),
            length_m: 12.0,
            max_diameter_m: 3.0,
            hull_volume_m3: None,
            gondola_volume_m3: 0.3,
            total_mass_kg: 70.0,
            lifting_gas: LiftingGas::Helium,
            cg_below_cv_m: 0.7,
            gear_height_m: 2.1,
            aero: AeroParams::default(),
            propulsion: PropulsionParams::default(),
            rudder: RudderParams::default(),
            limits: OperatingLimits::default(),
        }
    }
}

impl AirshipParams {
    /// Envelope volume (m^3): user override or prolate-ellipsoid estimate.
    pub fn hull_volume(&self) -> f64 {
        self.hull_volume_m3
            .unwrap_or_else(|| ellipsoid_volume(self.length_m, self.max_diameter_m))
    }

    /// Total displaced air volume (m^3).
    pub fn displaced_volume(&self) -> f64 {
        self.hull_volume() + self.gondola_volume_m3
    }

    /// Range checks that do not depend on the atmosphere. Physical consistency checks
    /// (buoyancy ratio, gas mass, achievable rates) live in [`crate::DerivedProperties`].
    pub fn validate(&self) -> ValidationReport {
        let mut r = ValidationReport::new();
        if self.name.trim().is_empty() {
            r.error("name", "name must not be empty");
        }
        let len_ok = r.range("length_m", self.length_m, 1.0, 250.0, "m");
        let dia_ok = r.range("max_diameter_m", self.max_diameter_m, 0.3, 60.0, "m");
        if len_ok && dia_ok {
            let fineness = self.length_m / self.max_diameter_m;
            if fineness < 1.0 {
                r.error(
                    "length_m",
                    "length must not be smaller than the diameter (prolate hull assumed)",
                );
            } else {
                r.recommend(
                    "length_m",
                    fineness,
                    2.5,
                    6.0,
                    "fineness ratio L/D outside the typical airship range",
                );
            }
            if let Some(v) = self.hull_volume_m3 {
                let cylinder =
                    std::f64::consts::PI * self.length_m * self.max_diameter_m.powi(2) / 4.0;
                if r.range("hull_volume_m3", v, 0.01, 1.0e6, "m^3")
                    && (v < 0.4 * cylinder || v > cylinder)
                {
                    r.error(
                        "hull_volume_m3",
                        format!(
                            "volume must lie between 40% and 100% of the enclosing cylinder volume ({:.1} m^3) for the given L and D",
                            cylinder
                        ),
                    );
                }
            }
            if self.gear_height_m < self.max_diameter_m / 2.0 {
                r.error(
                    "gear_height_m",
                    "landing-gear contact point must be at least D/2 below the centre of volume",
                );
            }
        }
        r.range(
            "gondola_volume_m3",
            self.gondola_volume_m3,
            0.0,
            1.0e4,
            "m^3",
        );
        r.range("total_mass_kg", self.total_mass_kg, 0.1, 1.0e6, "kg");
        if r.range("cg_below_cv_m", self.cg_below_cv_m, 0.0, 50.0, "m") && self.cg_below_cv_m < 0.05
        {
            r.error(
                "cg_below_cv_m",
                "CG must lie at least 0.05 m below the CV: the model relies on pendulum stability in pitch and roll",
            );
        }
        r.range("gear_height_m", self.gear_height_m, 0.1, 60.0, "m");

        let a = &self.aero;
        r.range("aero.cd_axial", a.cd_axial, 0.01, 1.5, "-");
        r.range("aero.cd_lateral", a.cd_lateral, 0.1, 2.0, "-");
        r.range("aero.cd_vertical", a.cd_vertical, 0.1, 2.0, "-");
        r.range("aero.fin_area_m2", a.fin_area_m2, 0.0, 1.0e4, "m^2");
        r.range(
            "aero.fin_lift_slope_per_rad",
            a.fin_lift_slope_per_rad,
            0.0,
            6.3,
            "1/rad",
        );
        if r.range("aero.fin_arm_m", a.fin_arm_m, 0.0, 200.0, "m")
            && a.fin_arm_m > 0.6 * self.length_m
        {
            r.error(
                "aero.fin_arm_m",
                "fin arm must not exceed 60% of the hull length",
            );
        }
        r.range(
            "aero.pitch_roll_damping_ratio",
            a.pitch_roll_damping_ratio,
            0.05,
            2.0,
            "-",
        );

        let p = &self.propulsion;
        r.range(
            "propulsion.main_max_thrust_n",
            p.main_max_thrust_n,
            0.1,
            1.0e5,
            "N",
        );
        r.range(
            "propulsion.main_reverse_ratio",
            p.main_reverse_ratio,
            0.0,
            1.0,
            "-",
        );
        r.range(
            "propulsion.main_lateral_arm_m",
            p.main_lateral_arm_m,
            0.05,
            50.0,
            "m",
        );
        r.range(
            "propulsion.main_vertical_offset_m",
            p.main_vertical_offset_m,
            -20.0,
            20.0,
            "m",
        );
        r.range(
            "propulsion.heave_max_thrust_n",
            p.heave_max_thrust_n,
            0.0,
            1.0e5,
            "N",
        );
        r.range(
            "propulsion.heave_reverse_ratio",
            p.heave_reverse_ratio,
            0.0,
            1.0,
            "-",
        );
        r.range(
            "propulsion.propeller_diameter_m",
            p.propeller_diameter_m,
            0.05,
            10.0,
            "m",
        );
        r.range(
            "propulsion.response_time_s",
            p.response_time_s,
            0.02,
            5.0,
            "s",
        );
        r.range("propulsion.efficiency", p.efficiency, 0.1, 1.0, "-");
        r.range(
            "propulsion.battery_capacity_wh",
            p.battery_capacity_wh,
            1.0,
            1.0e7,
            "Wh",
        );
        r.range(
            "propulsion.avionics_power_w",
            p.avionics_power_w,
            0.0,
            1.0e5,
            "W",
        );

        let ru = &self.rudder;
        r.range(
            "rudder.max_deflection_deg",
            ru.max_deflection_deg,
            0.0,
            45.0,
            "deg",
        );
        r.range(
            "rudder.max_rate_deg_s",
            ru.max_rate_deg_s,
            1.0,
            500.0,
            "deg/s",
        );
        r.range(
            "rudder.effectiveness_per_rad",
            ru.effectiveness_per_rad,
            0.0,
            6.3,
            "1/rad",
        );

        let l = &self.limits;
        r.range(
            "limits.max_climb_rate_m_s",
            l.max_climb_rate_m_s,
            0.05,
            20.0,
            "m/s",
        );
        r.range(
            "limits.max_descent_rate_m_s",
            l.max_descent_rate_m_s,
            0.05,
            20.0,
            "m/s",
        );
        r.range(
            "limits.max_airspeed_m_s",
            l.max_airspeed_m_s,
            0.5,
            60.0,
            "m/s",
        );
        r.range(
            "limits.max_turn_rate_deg_s",
            l.max_turn_rate_deg_s,
            0.5,
            90.0,
            "deg/s",
        );
        r.range("limits.max_pitch_deg", l.max_pitch_deg, 1.0, 60.0, "deg");
        r.range("limits.max_roll_deg", l.max_roll_deg, 1.0, 60.0, "deg");
        r
    }
}

/// Volume of a prolate ellipsoid with length `l` and maximum diameter `d` (m^3).
pub fn ellipsoid_volume(l: f64, d: f64) -> f64 {
    std::f64::consts::PI * l * d * d / 6.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_valid() {
        let r = AirshipParams::default().validate();
        assert!(!r.has_errors(), "{:?}", r.issues);
    }

    #[test]
    fn ellipsoid_volume_matches_formula() {
        // 4/3 pi a b c with a = 6, b = c = 1.5
        let v = ellipsoid_volume(12.0, 3.0);
        assert!((v - 4.0 / 3.0 * std::f64::consts::PI * 6.0 * 1.5 * 1.5).abs() < 1e-9);
    }

    #[test]
    fn rejects_unphysical_geometry() {
        let p = AirshipParams {
            length_m: 2.0,
            max_diameter_m: 3.0,
            ..Default::default()
        };
        assert!(p.validate().has_errors());
        let p = AirshipParams {
            hull_volume_m3: Some(500.0),
            ..Default::default()
        };
        assert!(
            p.validate().has_errors(),
            "volume larger than enclosing cylinder"
        );
        let p = AirshipParams {
            cg_below_cv_m: 0.0,
            ..Default::default()
        };
        assert!(p.validate().has_errors(), "no pendulum stability");
        let p = AirshipParams {
            total_mass_kg: f64::NAN,
            ..Default::default()
        };
        assert!(p.validate().has_errors());
    }
}
