//! Quantities derived from [`AirshipParams`] at a reference air density.
//!
//! Every value here is either an exact geometric consequence of the inputs or an
//! *estimate* based on a documented simplification (marked "estimate").

use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

use crate::G0;
use crate::params::AirshipParams;
use crate::validation::ValidationReport;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuoyancyState {
    /// Buoyancy exceeds weight by more than 0.5 %: the vehicle must be held down.
    Light,
    /// Buoyancy and weight agree within +/- 0.5 %.
    Neutral,
    /// Weight exceeds buoyancy by more than 0.5 %: thrust is required to lift off.
    Heavy,
}

/// Lamb's inertia coefficients `(k1, k2, k')` of a prolate spheroid with fineness
/// ratio `L/D` (axial, transverse and rotational added-mass factors).
///
/// With eccentricity `e = sqrt(1 - (D/L)^2)`:
///
/// ```text
/// alpha0 = 2 (1 - e^2) / e^3 * (0.5 ln((1+e)/(1-e)) - e)
/// beta0  = 1/e^2 - (1 - e^2) / (2 e^3) * ln((1+e)/(1-e))
/// k1 = alpha0 / (2 - alpha0),   k2 = beta0 / (2 - beta0)
/// k' = e^4 (beta0 - alpha0) / ((2 - e^2) (2 e^2 - (2 - e^2)(beta0 - alpha0)))
/// ```
pub fn lamb_coefficients(fineness: f64) -> (f64, f64, f64) {
    let f = fineness.max(1.0);
    let e2 = 1.0 - 1.0 / (f * f);
    if e2 < 1e-6 {
        // Sphere limit.
        return (0.5, 0.5, 0.0);
    }
    let e = e2.sqrt();
    let ln = ((1.0 + e) / (1.0 - e)).ln();
    let e3 = e2 * e;
    let alpha0 = 2.0 * (1.0 - e2) / e3 * (0.5 * ln - e);
    let beta0 = 1.0 / e2 - (1.0 - e2) / (2.0 * e3) * ln;
    let k1 = alpha0 / (2.0 - alpha0);
    let k2 = beta0 / (2.0 - beta0);
    let db = beta0 - alpha0;
    let kr = e2 * e2 * db / ((2.0 - e2) * (2.0 * e2 - (2.0 - e2) * db));
    (k1, k2, kr)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DerivedProperties {
    /// Density at which buoyancy-related values are evaluated (kg/m^3).
    pub reference_density_kg_m3: f64,
    pub hull_volume_m3: f64,
    /// True if the hull volume is the prolate-ellipsoid estimate rather than a user value.
    pub hull_volume_is_estimate: bool,
    pub displaced_volume_m3: f64,
    pub fineness_ratio: f64,
    /// Frontal reference area `pi D^2 / 4` (m^2).
    pub frontal_area_m2: f64,
    /// Side (lateral) projected area of the ellipse `pi L D / 4` (m^2).
    pub lateral_area_m2: f64,
    /// Planform (vertical) projected area `pi L D / 4` (m^2).
    pub planform_area_m2: f64,

    /// Archimedes buoyant force `rho V g` (N).
    pub buoyancy_n: f64,
    /// Weight `m g` (N).
    pub weight_n: f64,
    /// Net static lift `B - W` (N); negative means the vehicle is heavy.
    pub static_lift_n: f64,
    /// `B / W` (-).
    pub buoyancy_ratio: f64,
    pub buoyancy_state: BuoyancyState,
    /// Estimate of the lifting-gas mass at ambient conditions (kg).
    pub gas_mass_estimate_kg: f64,

    pub lamb_k1: f64,
    pub lamb_k2: f64,
    pub lamb_k_rot: f64,
    /// Added (virtual) mass along the hull axis `k1 rho V` (kg).
    pub added_mass_axial_kg: f64,
    /// Added mass in the transverse directions `k2 rho V` (kg).
    pub added_mass_transverse_kg: f64,
    pub effective_mass_axial_kg: f64,
    pub effective_mass_lateral_kg: f64,
    pub effective_mass_vertical_kg: f64,

    /// Rigid-body roll inertia, estimate `m D^2 / 10` (kg m^2).
    pub inertia_roll_kg_m2: f64,
    /// Rigid-body pitch and yaw inertia, estimate `m (L^2 + D^2) / 20` (kg m^2).
    pub inertia_pitch_yaw_kg_m2: f64,
    /// Added rotational inertia `k' rho V (L^2 + D^2) / 20` (kg m^2).
    pub added_inertia_pitch_yaw_kg_m2: f64,
    pub effective_inertia_roll_kg_m2: f64,
    pub effective_inertia_pitch_yaw_kg_m2: f64,

    /// Pendulum restoring stiffness `B * BG` (N m / rad).
    pub pendulum_stiffness_nm_rad: f64,
    pub pitch_natural_frequency_rad_s: f64,
    pub roll_natural_frequency_rad_s: f64,
    /// Linear pitch damping `2 zeta sqrt(K I)` (N m s / rad).
    pub pitch_damping_nms_rad: f64,
    pub roll_damping_nms_rad: f64,
    /// Quadratic hull yaw-damping coefficient `0.5 rho C_Dy D L^4 / 32` (N m s^2).
    pub hull_yaw_damping_coeff: f64,
    /// Fin yaw-damping coefficient `0.5 rho S C_La l^2` (N m s / (rad m/s)), multiplied by airspeed.
    pub fin_yaw_damping_coeff: f64,

    pub max_forward_thrust_n: f64,
    pub max_reverse_thrust_n: f64,
    pub max_heave_up_n: f64,
    pub max_heave_down_n: f64,
    /// Maximum yaw moment from differential main thrust (N m).
    pub max_differential_yaw_moment_nm: f64,
    /// Rudder moment at full deflection per unit dynamic pressure (N m / Pa).
    pub rudder_moment_per_pa: f64,
    /// Airspeed at which full rudder equals the differential-thrust yaw moment (m/s), estimate.
    pub rudder_crossover_airspeed_m_s: f64,
    pub propeller_disk_area_m2: f64,

    /// Level-flight speed at which axial drag equals maximum thrust (m/s), estimate.
    pub estimated_max_level_speed_m_s: f64,
    /// Vertical speed at which vertical drag balances heave thrust minus heaviness (m/s), estimate.
    pub estimated_max_climb_rate_m_s: f64,
    /// Vertical speed at which vertical drag balances down-thrust plus heaviness (m/s), estimate.
    pub estimated_max_descent_rate_m_s: f64,
    /// Heave thrust needed to cancel the static heaviness (N, negative if light).
    pub hover_heave_thrust_n: f64,
    /// Ideal momentum-theory electrical power to hold that thrust (W), estimate.
    pub hover_power_w: f64,
}

impl DerivedProperties {
    /// Evaluate derived properties of `p` at air density `rho` (kg/m^3).
    pub fn compute(p: &AirshipParams, rho: f64) -> Self {
        let l = p.length_m;
        let d = p.max_diameter_m;
        let m = p.total_mass_kg;
        let hull_volume = p.hull_volume();
        let displaced = p.displaced_volume();
        let fineness = l / d;
        let frontal = PI * d * d / 4.0;
        let lateral = PI * l * d / 4.0;

        let buoyancy = rho * displaced * G0;
        let weight = m * G0;
        let static_lift = buoyancy - weight;
        let ratio = buoyancy / weight;
        let state = if ratio > 1.005 {
            BuoyancyState::Light
        } else if ratio < 0.995 {
            BuoyancyState::Heavy
        } else {
            BuoyancyState::Neutral
        };

        let (k1, k2, kr) = lamb_coefficients(fineness);
        let rho_v = rho * hull_volume;
        let ma = k1 * rho_v;
        let mt = k2 * rho_v;

        let i_roll = m * d * d / 10.0;
        let i_py = m * (l * l + d * d) / 20.0;
        let i_py_added = kr * rho_v * (l * l + d * d) / 20.0;
        let i_roll_eff = i_roll;
        let i_py_eff = i_py + i_py_added;

        let k_pend = buoyancy * p.cg_below_cv_m;
        let zeta = p.aero.pitch_roll_damping_ratio;

        let prop = &p.propulsion;
        let max_fwd = 2.0 * prop.main_max_thrust_n;
        let max_rev = 2.0 * prop.main_max_thrust_n * prop.main_reverse_ratio;
        let max_up = prop.heave_max_thrust_n;
        let max_down = prop.heave_max_thrust_n * prop.heave_reverse_ratio;
        let diff_yaw =
            prop.main_lateral_arm_m * prop.main_max_thrust_n * (1.0 + prop.main_reverse_ratio);
        let rudder_per_pa = p.aero.fin_area_m2
            * p.rudder.effectiveness_per_rad
            * p.rudder.max_deflection_deg.to_radians()
            * p.aero.fin_arm_m;
        let crossover = if rudder_per_pa > 0.0 {
            (2.0 * diff_yaw / (rho * rudder_per_pa)).sqrt()
        } else {
            f64::INFINITY
        };
        let disk = PI * prop.propeller_diameter_m.powi(2) / 4.0;

        let heaviness = -static_lift;
        let k_vert = 0.5 * rho * p.aero.cd_vertical * lateral;
        let climb = if max_up > heaviness {
            ((max_up - heaviness) / k_vert).sqrt()
        } else {
            0.0
        };
        let descent = if max_down + heaviness > 0.0 {
            ((max_down + heaviness) / k_vert).sqrt()
        } else {
            0.0
        };
        let level = (max_fwd / (0.5 * rho * p.aero.cd_axial * frontal)).sqrt();
        let hover_power = heaviness.abs().powf(1.5) / (2.0 * rho * disk).sqrt() / prop.efficiency;

        Self {
            reference_density_kg_m3: rho,
            hull_volume_m3: hull_volume,
            hull_volume_is_estimate: p.hull_volume_m3.is_none(),
            displaced_volume_m3: displaced,
            fineness_ratio: fineness,
            frontal_area_m2: frontal,
            lateral_area_m2: lateral,
            planform_area_m2: lateral,
            buoyancy_n: buoyancy,
            weight_n: weight,
            static_lift_n: static_lift,
            buoyancy_ratio: ratio,
            buoyancy_state: state,
            gas_mass_estimate_kg: p.lifting_gas.density_ratio() * rho * hull_volume,
            lamb_k1: k1,
            lamb_k2: k2,
            lamb_k_rot: kr,
            added_mass_axial_kg: ma,
            added_mass_transverse_kg: mt,
            effective_mass_axial_kg: m + ma,
            effective_mass_lateral_kg: m + mt,
            effective_mass_vertical_kg: m + mt,
            inertia_roll_kg_m2: i_roll,
            inertia_pitch_yaw_kg_m2: i_py,
            added_inertia_pitch_yaw_kg_m2: i_py_added,
            effective_inertia_roll_kg_m2: i_roll_eff,
            effective_inertia_pitch_yaw_kg_m2: i_py_eff,
            pendulum_stiffness_nm_rad: k_pend,
            pitch_natural_frequency_rad_s: (k_pend / i_py_eff).sqrt(),
            roll_natural_frequency_rad_s: (k_pend / i_roll_eff).sqrt(),
            pitch_damping_nms_rad: 2.0 * zeta * (k_pend * i_py_eff).sqrt(),
            roll_damping_nms_rad: 2.0 * zeta * (k_pend * i_roll_eff).sqrt(),
            hull_yaw_damping_coeff: 0.5 * rho * p.aero.cd_lateral * d * l.powi(4) / 32.0,
            fin_yaw_damping_coeff: 0.5
                * rho
                * p.aero.fin_area_m2
                * p.aero.fin_lift_slope_per_rad
                * p.aero.fin_arm_m.powi(2),
            max_forward_thrust_n: max_fwd,
            max_reverse_thrust_n: max_rev,
            max_heave_up_n: max_up,
            max_heave_down_n: max_down,
            max_differential_yaw_moment_nm: diff_yaw,
            rudder_moment_per_pa: rudder_per_pa,
            rudder_crossover_airspeed_m_s: crossover,
            propeller_disk_area_m2: disk,
            estimated_max_level_speed_m_s: level,
            estimated_max_climb_rate_m_s: climb,
            estimated_max_descent_rate_m_s: descent,
            hover_heave_thrust_n: heaviness,
            hover_power_w: hover_power,
        }
    }

    /// Physical-consistency checks of the vehicle at this density. These catch
    /// configurations that pass range checks but are physically meaningless or
    /// cannot perform the vertical take-off and landing that the autopilot assumes.
    pub fn validate_physics(&self, p: &AirshipParams) -> ValidationReport {
        let mut r = ValidationReport::new();
        if p.total_mass_kg <= self.gas_mass_estimate_kg {
            r.error(
                "total_mass_kg",
                format!(
                    "total mass must exceed the lifting-gas mass contained in the envelope (~{:.1} kg)",
                    self.gas_mass_estimate_kg
                ),
            );
        }
        if !(0.6..=1.4).contains(&self.buoyancy_ratio) {
            r.error(
                "total_mass_kg",
                format!(
                    "buoyancy/weight ratio {:.2} is outside 0.6-1.4: the vehicle is not a meaningful lighter-than-air configuration",
                    self.buoyancy_ratio
                ),
            );
        } else if (self.buoyancy_ratio - 1.0).abs() > 0.1 {
            r.warning(
                "total_mass_kg",
                format!(
                    "static imbalance of {:.0} N exceeds 10% of the weight",
                    self.static_lift_n
                ),
            );
        }
        if self.max_heave_up_n <= -self.static_lift_n {
            r.error(
                "propulsion.heave_max_thrust_n",
                format!(
                    "heave thrust ({:.0} N) cannot overcome the static heaviness ({:.0} N): vertical take-off is impossible",
                    self.max_heave_up_n, -self.static_lift_n
                ),
            );
        }
        if self.max_heave_down_n <= self.static_lift_n {
            r.error(
                "propulsion.heave_reverse_ratio",
                format!(
                    "downward heave thrust ({:.0} N) cannot overcome the static lift ({:.0} N): controlled descent and landing are impossible",
                    self.max_heave_down_n, self.static_lift_n
                ),
            );
        }
        if p.limits.max_climb_rate_m_s > self.estimated_max_climb_rate_m_s {
            r.warning(
                "limits.max_climb_rate_m_s",
                format!(
                    "configured climb-rate limit exceeds the estimated capability of {:.2} m/s",
                    self.estimated_max_climb_rate_m_s
                ),
            );
        }
        if p.limits.max_descent_rate_m_s > self.estimated_max_descent_rate_m_s {
            r.warning(
                "limits.max_descent_rate_m_s",
                format!(
                    "configured descent-rate limit exceeds the estimated capability of {:.2} m/s",
                    self.estimated_max_descent_rate_m_s
                ),
            );
        }
        if p.limits.max_airspeed_m_s > self.estimated_max_level_speed_m_s {
            r.warning(
                "limits.max_airspeed_m_s",
                format!(
                    "configured airspeed limit exceeds the estimated maximum level speed of {:.1} m/s",
                    self.estimated_max_level_speed_m_s
                ),
            );
        }
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lamb_coefficients_match_tabulated_values() {
        // Munk / Lamb table for fineness ratio 4: k1 = 0.082, k2 = 0.860, k' = 0.608
        let (k1, k2, kr) = lamb_coefficients(4.0);
        assert!((k1 - 0.082).abs() < 0.002, "k1 {k1}");
        assert!((k2 - 0.860).abs() < 0.002, "k2 {k2}");
        assert!((kr - 0.608).abs() < 0.003, "k' {kr}");
        // Sphere limit
        let (k1, k2, kr) = lamb_coefficients(1.0);
        assert_eq!((k1, k2, kr), (0.5, 0.5, 0.0));
        // Close to a sphere the coefficients approach 0.5
        let (k1, k2, _) = lamb_coefficients(1.01);
        assert!((k1 - 0.5).abs() < 0.02 && (k2 - 0.5).abs() < 0.02);
    }

    #[test]
    fn archimedes_buoyancy_and_weight() {
        let p = AirshipParams::default();
        let d = DerivedProperties::compute(&p, 1.2);
        let expected_b = 1.2 * p.displaced_volume() * G0;
        assert!((d.buoyancy_n - expected_b).abs() < 1e-9);
        assert!((d.weight_n - p.total_mass_kg * G0).abs() < 1e-9);
        assert!((d.static_lift_n - (expected_b - p.total_mass_kg * G0)).abs() < 1e-9);
    }

    #[test]
    fn buoyancy_state_classification() {
        let mut p = AirshipParams::default();
        let rho = 1.225;
        let neutral_mass = rho * p.displaced_volume();
        p.total_mass_kg = neutral_mass;
        assert_eq!(
            DerivedProperties::compute(&p, rho).buoyancy_state,
            BuoyancyState::Neutral
        );
        p.total_mass_kg = neutral_mass * 1.05;
        assert_eq!(
            DerivedProperties::compute(&p, rho).buoyancy_state,
            BuoyancyState::Heavy
        );
        p.total_mass_kg = neutral_mass * 0.95;
        assert_eq!(
            DerivedProperties::compute(&p, rho).buoyancy_state,
            BuoyancyState::Light
        );
    }

    #[test]
    fn physics_validation_rejects_infeasible_vtol() {
        let mut p = AirshipParams::default();
        let rho = 1.225;
        assert!(
            !DerivedProperties::compute(&p, rho)
                .validate_physics(&p)
                .has_errors()
        );
        // Too heavy for the heave thruster.
        p.total_mass_kg = rho * p.displaced_volume() + 15.0;
        let r = DerivedProperties::compute(&p, rho).validate_physics(&p);
        assert!(
            r.errors()
                .any(|i| i.field == "propulsion.heave_max_thrust_n")
        );
        // Too light to be pulled down.
        let mut p = AirshipParams::default();
        p.propulsion.heave_reverse_ratio = 0.05;
        p.total_mass_kg = rho * p.displaced_volume() - 3.0;
        let r = DerivedProperties::compute(&p, rho).validate_physics(&p);
        assert!(
            r.errors()
                .any(|i| i.field == "propulsion.heave_reverse_ratio")
        );
        // Mass below the gas mass is unphysical.
        let mut p = AirshipParams::default();
        p.total_mass_kg = 5.0;
        assert!(
            DerivedProperties::compute(&p, rho)
                .validate_physics(&p)
                .has_errors()
        );
    }

    #[test]
    fn effective_mass_is_anisotropic() {
        let d = DerivedProperties::compute(&AirshipParams::default(), 1.225);
        assert!(d.effective_mass_lateral_kg > d.effective_mass_axial_kg);
        assert!(d.effective_inertia_pitch_yaw_kg_m2 > d.inertia_pitch_yaw_kg_m2);
    }
}
