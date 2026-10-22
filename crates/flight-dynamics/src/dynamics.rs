//! Forces, moments and equations of motion.
//!
//! ## Translational dynamics (world ENU frame)
//!
//! ```text
//! F = F_b + F_g + R (F_aero,b + F_thrust,b) + F_ground
//! F_b = (0, 0, rho(z) V g)              buoyancy (Archimedes, local density)
//! F_g = (0, 0, -m g)                     gravity
//! a_b = (R^T F) ./ (m + m_a,x, m + m_a,y, m + m_a,z)
//! dv/dt = R a_b,   dp/dt = v
//! ```
//!
//! The added (virtual) masses `m_a` are the Lamb coefficients times `rho V` and are
//! applied as a quasi-static anisotropic inertia. Added-mass Coriolis coupling and
//! the Munk moment are neglected (see `docs/limitations.md`).
//!
//! ## Aerodynamics (body frame, relative to the local air)
//!
//! With `v_r = R^T (v - w) = (u, v, w)` and `V = |v_r|`:
//!
//! ```text
//! hull drag   D = -1/2 rho (Cdx Ax |u| u, Cdy Ay |v| v, Cdz Az |w| w)
//! fin force   Y_f = -1/2 rho S (CLa |u| v_f + CD_plate |v_f| v_f),  v_f = v - r l_f
//! rudder      Y_r = -1/2 rho u|u| S CLd delta
//! ```
//!
//! ## Rotational dynamics (decoupled small-angle axes)
//!
//! ```text
//! I_z' dr/dt = N_fin + N_rudder + b (T_R - T_L) - c_h r|r| + N_ground
//! I_y' dq/dt = z_T (T_L + T_R) - BG X_aero - B BG sin(theta) - c_theta q + M_ground
//! I_x  dp/dt = -BG Y_aero - B BG sin(phi) - c_phi p + L_ground
//! d(psi)/dt = r,  d(theta)/dt = q,  d(phi)/dt = p
//! ```

use airship_model::{AirshipParams, AtmosphereConfig, DerivedProperties, G0, Mat3, Vec3};
use serde::{Deserialize, Serialize};

use crate::actuators::ActuatorState;
use crate::ground::{self, GroundContactParams};
use crate::integrator::rk4_step;
use crate::state::{STATE_DIM, VehicleState};

/// Crossflow drag coefficient of the fins treated as flat plates at large incidence.
const FIN_PLATE_CD: f64 = 1.2;

/// Detailed force and moment breakdown at one state (for telemetry and tests).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct ForceBreakdown {
    pub air_density: f64,
    pub buoyancy: Vec3,
    pub gravity: Vec3,
    /// Hull drag + fin + rudder force, world frame (N).
    pub aero: Vec3,
    /// Thruster force, world frame (N).
    pub thrust: Vec3,
    /// Gear contact force, world frame (N).
    pub ground: Vec3,
    /// Sum of all forces except the mooring reaction, world frame (N).
    pub total: Vec3,
    /// Velocity relative to the air in the body frame (m/s).
    pub air_velocity_body: Vec3,
    pub airspeed: f64,
    pub yaw_moment_rudder: f64,
    pub yaw_moment_thrust: f64,
    pub yaw_moment_aero: f64,
    pub pitch_moment: f64,
    pub gear_compression_m: f64,
    pub ground_normal_n: f64,
}

#[derive(Debug, Clone)]
pub struct AirshipDynamics {
    pub params: AirshipParams,
    /// Inertial properties evaluated at ground-level density.
    pub derived: DerivedProperties,
    pub atmosphere: AtmosphereConfig,
    pub ground_elevation_m: f64,
    pub ground: GroundContactParams,
}

impl AirshipDynamics {
    pub fn new(
        params: &AirshipParams,
        atmosphere: &AtmosphereConfig,
        ground_elevation_m: f64,
    ) -> Self {
        let derived = DerivedProperties::compute(params, atmosphere.density(ground_elevation_m));
        Self {
            ground: GroundContactParams::for_vehicle(&derived),
            params: params.clone(),
            derived,
            atmosphere: atmosphere.clone(),
            ground_elevation_m,
        }
    }

    /// Height of the landing-gear contact point above ground (m).
    pub fn gear_agl(&self, s: &VehicleState) -> f64 {
        s.position.z - self.params.gear_height_m - self.ground_elevation_m
    }

    /// State derivative and force breakdown. When `moored` the vehicle is held by the
    /// mooring mast: all derivatives are zero and the mast reaction equals `-total`.
    pub fn evaluate(
        &self,
        s: &VehicleState,
        a: &ActuatorState,
        wind: Vec3,
        moored: bool,
    ) -> (VehicleState, ForceBreakdown) {
        let p = &self.params;
        let d = &self.derived;
        let rho = self.atmosphere.density(s.position.z);
        let r_bw = Mat3::body_to_world(s.yaw, s.pitch, s.roll);
        let r_wb = r_bw.transpose();

        // Air-relative velocity in the body frame.
        let vb = r_wb.mul_vec(s.velocity - wind);
        let (u, v, w) = (vb.x, vb.y, vb.z);
        let airspeed = vb.norm();
        let half_rho = 0.5 * rho;

        // Hull drag (direction-dependent areas and coefficients).
        let drag = Vec3::new(
            -half_rho * p.aero.cd_axial * d.frontal_area_m2 * u.abs() * u,
            -half_rho * p.aero.cd_lateral * d.lateral_area_m2 * v.abs() * v,
            -half_rho * p.aero.cd_vertical * d.planform_area_m2 * w.abs() * w,
        );
        // Vertical tail fins: local lateral velocity includes the yaw-rate contribution.
        let lf = p.aero.fin_arm_m;
        let v_fin = v - s.yaw_rate * lf;
        let fin_y = -half_rho
            * p.aero.fin_area_m2
            * (p.aero.fin_lift_slope_per_rad * u.abs() * v_fin
                + FIN_PLATE_CD * v_fin.abs() * v_fin);
        let rudder_y = -half_rho
            * u
            * u.abs()
            * p.aero.fin_area_m2
            * p.rudder.effectiveness_per_rad
            * a.rudder_rad;
        let aero_b = drag + Vec3::new(0.0, fin_y + rudder_y, 0.0);

        // Thrusters.
        let t_main = a.thrust_left_n + a.thrust_right_n;
        let thrust_b = Vec3::new(t_main, 0.0, a.thrust_heave_n);

        let buoyancy_mag = rho * p.displaced_volume() * G0;
        let buoyancy = Vec3::new(0.0, 0.0, buoyancy_mag);
        let gravity = Vec3::new(0.0, 0.0, -p.total_mass_kg * G0);
        let aero_w = r_bw.mul_vec(aero_b);
        let thrust_w = r_bw.mul_vec(thrust_b);

        let gear_z = s.position.z - p.gear_height_m;
        let c = ground::contact(
            &self.ground,
            gear_z,
            self.ground_elevation_m,
            s.velocity,
            s.pitch,
            s.pitch_rate,
            s.roll,
            s.roll_rate,
            s.yaw_rate,
        );
        let total = buoyancy + gravity + aero_w + thrust_w + c.force;

        // Moments.
        let bg = p.cg_below_cv_m;
        let n_rudder = -lf * rudder_y;
        let n_fin = -lf * fin_y;
        let n_hull = -d.hull_yaw_damping_coeff * s.yaw_rate * s.yaw_rate.abs();
        let n_thrust = p.propulsion.main_lateral_arm_m * (a.thrust_right_n - a.thrust_left_n);
        let yaw_moment = n_rudder + n_fin + n_hull + n_thrust + c.yaw_moment;

        let pitch_moment = p.propulsion.main_vertical_offset_m * t_main
            - bg * aero_b.x
            - buoyancy_mag * bg * s.pitch.sin()
            - d.pitch_damping_nms_rad * s.pitch_rate
            + c.pitch_moment;
        let roll_moment = -bg * aero_b.y
            - buoyancy_mag * bg * s.roll.sin()
            - d.roll_damping_nms_rad * s.roll_rate
            + c.roll_moment;

        let breakdown = ForceBreakdown {
            air_density: rho,
            buoyancy,
            gravity,
            aero: aero_w,
            thrust: thrust_w,
            ground: c.force,
            total,
            air_velocity_body: vb,
            airspeed,
            yaw_moment_rudder: n_rudder,
            yaw_moment_thrust: n_thrust,
            yaw_moment_aero: n_fin + n_hull,
            pitch_moment,
            gear_compression_m: c.compression_m,
            ground_normal_n: c.normal_n,
        };

        if moored {
            // Nose-cone mooring mast: translation, pitch and roll are restrained, the
            // hull is free to weathervane in yaw about the mast (no ground friction).
            let n_free = n_rudder + n_fin + n_hull + n_thrust;
            let deriv = VehicleState {
                yaw: s.yaw_rate,
                yaw_rate: n_free / d.effective_inertia_pitch_yaw_kg_m2,
                ..VehicleState::default()
            };
            return (deriv, breakdown);
        }

        let fb = r_wb.mul_vec(total);
        let ab = Vec3::new(
            fb.x / d.effective_mass_axial_kg,
            fb.y / d.effective_mass_lateral_kg,
            fb.z / d.effective_mass_vertical_kg,
        );
        let accel = r_bw.mul_vec(ab);
        let deriv = VehicleState {
            position: s.velocity,
            velocity: accel,
            yaw: s.yaw_rate,
            yaw_rate: yaw_moment / d.effective_inertia_pitch_yaw_kg_m2,
            pitch: s.pitch_rate,
            pitch_rate: pitch_moment / d.effective_inertia_pitch_yaw_kg_m2,
            roll: s.roll_rate,
            roll_rate: roll_moment / d.effective_inertia_roll_kg_m2,
        };
        (deriv, breakdown)
    }

    /// One fixed RK4 step of length `dt` starting at time `t`. Actuators are held
    /// constant over the step; `wind(t, agl)` is evaluated at every stage.
    pub fn step_rk4(
        &self,
        s: &VehicleState,
        a: &ActuatorState,
        moored: bool,
        t: f64,
        dt: f64,
        wind: impl Fn(f64, f64) -> Vec3,
    ) -> VehicleState {
        let x0 = s.to_array();
        let x1 = rk4_step::<STATE_DIM>(&x0, t, dt, |ti, xi| {
            let si = VehicleState::from_array(xi);
            let wi = wind(ti, self.gear_agl(&si) + self.params.gear_height_m);
            self.evaluate(&si, a, wi, moored).0.to_array()
        });
        let mut out = VehicleState::from_array(&x1);
        out.yaw = airship_model::math::wrap_pi(out.yaw);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use airship_model::Vec2;

    fn model() -> AirshipDynamics {
        let atm = AtmosphereConfig {
            model: airship_model::AtmosphereModel::Constant,
            constant_density_kg_m3: 1.225,
            ..Default::default()
        };
        AirshipDynamics::new(&AirshipParams::default(), &atm, 0.0)
    }

    fn airborne(m: &AirshipDynamics) -> VehicleState {
        VehicleState {
            position: Vec3::new(0.0, 0.0, 100.0 + m.params.gear_height_m),
            ..Default::default()
        }
    }

    #[test]
    fn buoyancy_and_gravity_at_rest() {
        let m = model();
        let s = airborne(&m);
        let (dx, f) = m.evaluate(&s, &ActuatorState::default(), Vec3::ZERO, false);
        let b = 1.225 * m.params.displaced_volume() * G0;
        let w = m.params.total_mass_kg * G0;
        assert!((f.buoyancy.z - b).abs() < 1e-9);
        assert!((f.gravity.z + w).abs() < 1e-9);
        assert!(
            (f.total.z - (b - w)).abs() < 1e-9,
            "net vertical force is B - W at rest"
        );
        let expected_az = (b - w) / m.derived.effective_mass_vertical_kg;
        assert!((dx.velocity.z - expected_az).abs() < 1e-12);
        assert_eq!(dx.velocity.x, 0.0);
    }

    #[test]
    fn drag_opposes_relative_wind_and_scales_quadratically() {
        let m = model();
        let mut s = airborne(&m);
        s.velocity = Vec3::new(4.0, 0.0, 0.0);
        let (_, f1) = m.evaluate(&s, &ActuatorState::default(), Vec3::ZERO, false);
        assert!(f1.aero.x < 0.0, "drag opposes motion");
        let expected = 0.5 * 1.225 * m.params.aero.cd_axial * m.derived.frontal_area_m2 * 16.0;
        assert!((f1.aero.x + expected).abs() < 1e-9);
        s.velocity = Vec3::new(8.0, 0.0, 0.0);
        let (_, f2) = m.evaluate(&s, &ActuatorState::default(), Vec3::ZERO, false);
        assert!((f2.aero.x / f1.aero.x - 4.0).abs() < 1e-9);
        // Drag depends on velocity relative to the air: moving with the wind gives zero drag.
        let (_, f3) = m.evaluate(
            &s,
            &ActuatorState::default(),
            Vec3::new(8.0, 0.0, 0.0),
            false,
        );
        assert!(f3.aero.norm() < 1e-12);
        // A stationary vehicle in wind is pushed downwind.
        s.velocity = Vec3::ZERO;
        let (_, f4) = m.evaluate(
            &s,
            &ActuatorState::default(),
            Vec3::new(0.0, 3.0, 0.0),
            false,
        );
        assert!(f4.aero.y > 0.0);
    }

    #[test]
    fn lateral_drag_exceeds_axial_drag() {
        let m = model();
        let mut s = airborne(&m);
        s.velocity = Vec3::new(3.0, 0.0, 0.0);
        let (_, fa) = m.evaluate(&s, &ActuatorState::default(), Vec3::ZERO, false);
        s.velocity = Vec3::new(0.0, 3.0, 0.0);
        let (_, fl) = m.evaluate(&s, &ActuatorState::default(), Vec3::ZERO, false);
        assert!(fl.aero.norm() > 10.0 * fa.aero.norm());
    }

    #[test]
    fn thrust_and_yaw_moment_signs() {
        let m = model();
        let s = airborne(&m);
        let a = ActuatorState {
            thrust_left_n: 10.0,
            thrust_right_n: 30.0,
            thrust_heave_n: 20.0,
            rudder_rad: 0.0,
        };
        let (dx, f) = m.evaluate(&s, &a, Vec3::ZERO, false);
        assert!((f.thrust.x - 40.0).abs() < 1e-9 && (f.thrust.z - 20.0).abs() < 1e-9);
        assert!(
            dx.yaw_rate > 0.0,
            "more starboard thrust turns the nose left (CCW)"
        );
        assert!(
            dx.pitch_rate > 0.0,
            "forward thrust below the CG pitches the nose up"
        );
    }

    #[test]
    fn rudder_authority_scales_with_dynamic_pressure() {
        let m = model();
        let a = ActuatorState {
            rudder_rad: 0.2,
            ..Default::default()
        };
        let mut s = airborne(&m);
        let (_, f0) = m.evaluate(&s, &a, Vec3::ZERO, false);
        assert!(
            f0.yaw_moment_rudder.abs() < 1e-12,
            "no rudder authority at zero airspeed"
        );
        s.velocity = Vec3::new(3.0, 0.0, 0.0);
        let (_, f3) = m.evaluate(&s, &a, Vec3::ZERO, false);
        s.velocity = Vec3::new(6.0, 0.0, 0.0);
        let (_, f6) = m.evaluate(&s, &a, Vec3::ZERO, false);
        assert!(f3.yaw_moment_rudder > 0.0);
        assert!((f6.yaw_moment_rudder / f3.yaw_moment_rudder - 4.0).abs() < 1e-9);
    }

    #[test]
    fn fins_provide_weathercock_stability() {
        let m = model();
        let mut s = airborne(&m);
        // Flying East with the nose yawed 10 deg left: sideslip toward the right side.
        s.yaw = 10f64.to_radians();
        s.velocity = Vec3::new(5.0, 0.0, 0.0);
        let (dx, _) = m.evaluate(&s, &ActuatorState::default(), Vec3::ZERO, false);
        assert!(
            dx.yaw_rate < 0.0,
            "nose is pushed back toward the relative wind"
        );
    }

    #[test]
    fn pendulum_restores_pitch_and_roll() {
        let m = model();
        let mut s = airborne(&m);
        s.pitch = 0.2;
        s.roll = -0.1;
        let (dx, _) = m.evaluate(&s, &ActuatorState::default(), Vec3::ZERO, false);
        assert!(dx.pitch_rate < 0.0 && dx.roll_rate > 0.0);
    }

    #[test]
    fn mooring_holds_vehicle_and_reports_load() {
        let m = model();
        let s = VehicleState {
            position: Vec3::new(0.0, 0.0, m.params.gear_height_m),
            ..Default::default()
        };
        let a = ActuatorState {
            thrust_heave_n: 50.0,
            ..Default::default()
        };
        let (dx, f) = m.evaluate(&s, &a, Vec3::ZERO, true);
        assert_eq!(dx, VehicleState::default());
        let expected =
            1.225 * m.params.displaced_volume() * G0 - m.params.total_mass_kg * G0 + 50.0;
        assert!((f.total.z - expected).abs() < 1e-9);
        assert_eq!(m.step_rk4(&s, &a, true, 0.0, 0.01, |_, _| Vec3::ZERO), s);
    }

    #[test]
    fn moored_vehicle_weathervanes_into_the_wind() {
        let m = model();
        let mut s = VehicleState {
            position: Vec3::new(0.0, 0.0, m.params.gear_height_m),
            ..Default::default()
        };
        // Nose East, wind blowing from the North (toward -y).
        let wind = Vec3::new(0.0, -4.0, 0.0);
        let mut t = 0.0;
        for _ in 0..6000 {
            s = m.step_rk4(&s, &ActuatorState::default(), true, t, 0.01, |_, _| wind);
            t += 0.01;
        }
        assert_eq!(s.position, Vec3::new(0.0, 0.0, m.params.gear_height_m));
        let heading = airship_model::math::yaw_to_heading_deg(s.yaw);
        assert!(
            heading < 5.0 || heading > 355.0,
            "nose into the northerly wind, heading {heading}"
        );
    }

    #[test]
    fn heavy_vehicle_settles_on_gear_without_penetrating() {
        let mut p = AirshipParams::default();
        p.total_mass_kg = 1.225 * p.displaced_volume() + 3.0; // ~29 N heavy
        let atm = AtmosphereConfig {
            model: airship_model::AtmosphereModel::Constant,
            ..Default::default()
        };
        let m = AirshipDynamics::new(&p, &atm, 0.0);
        let mut s = VehicleState {
            position: Vec3::new(0.0, 0.0, p.gear_height_m + 0.5),
            ..Default::default()
        };
        let mut t = 0.0;
        for _ in 0..6000 {
            s = m.step_rk4(&s, &ActuatorState::default(), false, t, 0.01, |_, _| {
                Vec3::ZERO
            });
            t += 0.01;
        }
        let compression = -(s.position.z - p.gear_height_m);
        let expected = 3.0 * G0 / m.ground.stiffness_n_m;
        assert!(
            (compression - expected).abs() < 1e-3,
            "static compression {compression} vs {expected}"
        );
        assert!(s.velocity.norm() < 1e-3);
    }

    #[test]
    fn vertical_terminal_velocity_matches_drag_balance() {
        let m = model();
        let mut s = airborne(&m);
        s.position.z += 500.0;
        let a = ActuatorState {
            thrust_heave_n: 60.0,
            ..Default::default()
        };
        let mut t = 0.0;
        for _ in 0..6000 {
            s = m.step_rk4(&s, &a, false, t, 0.01, |_, _| Vec3::ZERO);
            t += 0.01;
        }
        let net = 60.0 + m.derived.static_lift_n;
        let expected =
            (net / (0.5 * 1.225 * m.params.aero.cd_vertical * m.derived.planform_area_m2)).sqrt();
        assert!(
            (s.velocity.z - expected).abs() < 1e-3,
            "{} vs {}",
            s.velocity.z,
            expected
        );
        assert!(s.horizontal_position().norm() < 1e-9 && Vec2::ZERO == s.velocity.horizontal());
    }

    #[test]
    fn rk4_converges_with_timestep() {
        // Compare a 30 s manoeuvre at dt = 0.04, 0.02 and 0.01 against dt = 0.0025.
        let m = model();
        let a = ActuatorState {
            thrust_left_n: 30.0,
            thrust_right_n: 45.0,
            thrust_heave_n: 25.0,
            rudder_rad: 0.15,
        };
        let wind = |_: f64, _: f64| Vec3::new(1.0, -2.0, 0.0);
        let run = |dt: f64| {
            let mut s = airborne(&m);
            let n = (30.0 / dt).round() as usize;
            for k in 0..n {
                s = m.step_rk4(&s, &a, false, k as f64 * dt, dt, wind);
            }
            s
        };
        let reference = run(0.0025);
        let err = |dt: f64| (run(dt).position - reference.position).norm();
        let (e1, e2, e3) = (err(0.04), err(0.02), err(0.01));
        assert!(e1 > e2 && e2 > e3, "errors must decrease: {e1} {e2} {e3}");
        assert!(e3 < 1e-4, "dt = 0.01 s position error {e3} m");
    }
}
