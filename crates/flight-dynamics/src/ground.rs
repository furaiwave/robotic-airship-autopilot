//! Ground-contact model for the landing gear.
//!
//! The gear contact point lies `gear_height` below the CV. With the gear
//! compression `delta = (z_ground + gear_height) - z` the unilateral normal force is
//!
//! ```text
//! N = max(0, k delta - c v_z)        for delta > 0
//! ```
//!
//! and the horizontal friction force is a regularised Coulomb law
//! `F_f = -mu N tanh(|v_h| / v_s) v_h / |v_h|`. Pitch and roll are restrained by a
//! stiff rotational spring-damper (the gear footprint) while in contact, and yaw by
//! a regularised friction moment. Flat terrain at a constant elevation is assumed.

use airship_model::{DerivedProperties, Vec3};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GroundContactParams {
    /// Normal stiffness `k` (N/m).
    pub stiffness_n_m: f64,
    /// Normal damping `c` (N s/m).
    pub damping_n_s_m: f64,
    /// Coulomb friction coefficient `mu` (-).
    pub friction_coefficient: f64,
    /// Friction regularisation speed `v_s` (m/s).
    pub friction_reg_speed_m_s: f64,
    /// Rotational stiffness of the gear footprint in pitch and roll (N m/rad).
    pub attitude_stiffness_nm_rad: f64,
    /// Rotational damping in pitch and roll (N m s/rad).
    pub attitude_damping_nms_rad: f64,
    /// Effective radius of the yaw friction moment (m).
    pub yaw_friction_radius_m: f64,
}

impl GroundContactParams {
    /// Contact parameters scaled to the vehicle: natural frequency 4 rad/s and
    /// damping ratio 0.8 for the effective vertical mass.
    pub fn for_vehicle(d: &DerivedProperties) -> Self {
        let m = d.effective_mass_vertical_kg;
        let omega: f64 = 4.0;
        let k = m * omega * omega;
        let i = d.effective_inertia_pitch_yaw_kg_m2;
        let k_att = 10.0 * d.pendulum_stiffness_nm_rad;
        Self {
            stiffness_n_m: k,
            damping_n_s_m: 2.0 * 0.8 * (k * m).sqrt(),
            friction_coefficient: 0.5,
            friction_reg_speed_m_s: 0.05,
            attitude_stiffness_nm_rad: k_att,
            attitude_damping_nms_rad: 2.0 * (k_att * i).sqrt(),
            yaw_friction_radius_m: 0.5,
        }
    }
}

/// Contact force and moments in the world frame / about body axes.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ContactOutput {
    pub compression_m: f64,
    pub normal_n: f64,
    pub force: Vec3,
    pub pitch_moment: f64,
    pub roll_moment: f64,
    pub yaw_moment: f64,
}

impl ContactOutput {
    pub fn in_contact(&self) -> bool {
        self.compression_m > 0.0
    }
}

#[allow(clippy::too_many_arguments)]
pub fn contact(
    g: &GroundContactParams,
    gear_z: f64,
    ground_z: f64,
    velocity: Vec3,
    pitch: f64,
    pitch_rate: f64,
    roll: f64,
    roll_rate: f64,
    yaw_rate: f64,
) -> ContactOutput {
    let compression = ground_z - gear_z;
    if compression <= 0.0 {
        return ContactOutput {
            compression_m: compression,
            ..Default::default()
        };
    }
    let normal = (g.stiffness_n_m * compression - g.damping_n_s_m * velocity.z).max(0.0);
    let vh = velocity.horizontal();
    let speed = vh.norm();
    let friction = if speed > 1e-9 {
        vh * (-g.friction_coefficient * normal * (speed / g.friction_reg_speed_m_s).tanh() / speed)
    } else {
        airship_model::Vec2::ZERO
    };
    ContactOutput {
        compression_m: compression,
        normal_n: normal,
        force: Vec3::new(friction.x, friction.y, normal),
        pitch_moment: -g.attitude_stiffness_nm_rad * pitch
            - g.attitude_damping_nms_rad * pitch_rate,
        roll_moment: -g.attitude_stiffness_nm_rad * roll - g.attitude_damping_nms_rad * roll_rate,
        yaw_moment: -g.friction_coefficient
            * normal
            * g.yaw_friction_radius_m
            * (yaw_rate / 0.05).tanh(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use airship_model::AirshipParams;

    fn params() -> GroundContactParams {
        GroundContactParams::for_vehicle(&DerivedProperties::compute(
            &AirshipParams::default(),
            1.225,
        ))
    }

    #[test]
    fn no_force_above_ground() {
        let c = contact(
            &params(),
            10.0,
            0.0,
            Vec3::new(1.0, 0.0, -1.0),
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
        );
        assert!(!c.in_contact());
        assert_eq!(c.force, Vec3::ZERO);
    }

    #[test]
    fn normal_force_is_unilateral_and_opposes_penetration() {
        let g = params();
        let c = contact(
            &g,
            -0.05,
            0.0,
            Vec3::new(0.0, 0.0, -0.5),
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
        );
        assert!(c.in_contact());
        assert!((c.normal_n - (g.stiffness_n_m * 0.05 + g.damping_n_s_m * 0.5)).abs() < 1e-9);
        // Rapid separation must not produce a pulling force.
        let c = contact(
            &g,
            -0.001,
            0.0,
            Vec3::new(0.0, 0.0, 5.0),
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
        );
        assert_eq!(c.normal_n, 0.0);
    }

    #[test]
    fn friction_opposes_sliding() {
        let c = contact(
            &params(),
            -0.05,
            0.0,
            Vec3::new(2.0, -1.0, 0.0),
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
        );
        assert!(c.force.x < 0.0 && c.force.y > 0.0);
        let mu_n = 0.5 * c.normal_n;
        assert!(c.force.horizontal().norm() <= mu_n + 1e-9);
    }
}
