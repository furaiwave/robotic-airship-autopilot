//! Minimal fixed-size vector and matrix types plus angle helpers.

use serde::{Deserialize, Serialize};
use std::f64::consts::{PI, TAU};
use std::ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign};

/// Three-component vector (world or body frame, documented at the use site).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub const ZERO: Vec3 = Vec3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn dot(self, o: Vec3) -> f64 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }

    pub fn cross(self, o: Vec3) -> Vec3 {
        Vec3::new(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }

    pub fn norm(self) -> f64 {
        self.dot(self).sqrt()
    }

    pub fn horizontal(self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }
}

impl Add for Vec3 {
    type Output = Vec3;
    fn add(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}

impl AddAssign for Vec3 {
    fn add_assign(&mut self, o: Vec3) {
        *self = *self + o;
    }
}

impl Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}

impl SubAssign for Vec3 {
    fn sub_assign(&mut self, o: Vec3) {
        *self = *self - o;
    }
}

impl Mul<f64> for Vec3 {
    type Output = Vec3;
    fn mul(self, s: f64) -> Vec3 {
        Vec3::new(self.x * s, self.y * s, self.z * s)
    }
}

impl Neg for Vec3 {
    type Output = Vec3;
    fn neg(self) -> Vec3 {
        Vec3::new(-self.x, -self.y, -self.z)
    }
}

/// Two-component horizontal vector (East, North).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    pub const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };

    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Unit vector pointing along yaw angle `psi` (rad, CCW from East).
    pub fn from_angle(psi: f64) -> Self {
        Self::new(psi.cos(), psi.sin())
    }

    pub fn dot(self, o: Vec2) -> f64 {
        self.x * o.x + self.y * o.y
    }

    /// z-component of the 3D cross product (positive if `o` is CCW from `self`).
    pub fn cross(self, o: Vec2) -> f64 {
        self.x * o.y - self.y * o.x
    }

    pub fn norm(self) -> f64 {
        self.dot(self).sqrt()
    }

    /// Direction of the vector as a yaw angle (rad, CCW from East).
    pub fn angle(self) -> f64 {
        self.y.atan2(self.x)
    }

    pub fn normalized(self) -> Vec2 {
        let n = self.norm();
        if n > 1e-12 {
            self * (1.0 / n)
        } else {
            Vec2::ZERO
        }
    }

    /// Scale the vector down so that its norm does not exceed `max`.
    pub fn clamp_norm(self, max: f64) -> Vec2 {
        let n = self.norm();
        if n > max && n > 0.0 {
            self * (max / n)
        } else {
            self
        }
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

impl Add for Vec2 {
    type Output = Vec2;
    fn add(self, o: Vec2) -> Vec2 {
        Vec2::new(self.x + o.x, self.y + o.y)
    }
}

impl Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, o: Vec2) -> Vec2 {
        Vec2::new(self.x - o.x, self.y - o.y)
    }
}

impl Mul<f64> for Vec2 {
    type Output = Vec2;
    fn mul(self, s: f64) -> Vec2 {
        Vec2::new(self.x * s, self.y * s)
    }
}

impl Neg for Vec2 {
    type Output = Vec2;
    fn neg(self) -> Vec2 {
        Vec2::new(-self.x, -self.y)
    }
}

/// Row-major 3x3 matrix.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat3(pub [[f64; 3]; 3]);

impl Mat3 {
    pub const IDENTITY: Mat3 = Mat3([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);

    /// Body-to-world rotation `R = Rz(psi) * Ry(-theta) * Rx(phi)` for the
    /// FLU body frame with nose-up-positive pitch (see crate docs).
    pub fn body_to_world(psi: f64, theta: f64, phi: f64) -> Mat3 {
        let (sps, cps) = psi.sin_cos();
        let (sth, cth) = theta.sin_cos();
        let (sph, cph) = phi.sin_cos();
        let rz = Mat3([[cps, -sps, 0.0], [sps, cps, 0.0], [0.0, 0.0, 1.0]]);
        let ry = Mat3([[cth, 0.0, -sth], [0.0, 1.0, 0.0], [sth, 0.0, cth]]);
        let rx = Mat3([[1.0, 0.0, 0.0], [0.0, cph, -sph], [0.0, sph, cph]]);
        rz.mul_mat(&ry).mul_mat(&rx)
    }

    pub fn mul_mat(&self, o: &Mat3) -> Mat3 {
        let mut r = [[0.0; 3]; 3];
        for (i, row) in r.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                *cell = (0..3).map(|k| self.0[i][k] * o.0[k][j]).sum();
            }
        }
        Mat3(r)
    }

    pub fn mul_vec(&self, v: Vec3) -> Vec3 {
        let m = &self.0;
        Vec3::new(
            m[0][0] * v.x + m[0][1] * v.y + m[0][2] * v.z,
            m[1][0] * v.x + m[1][1] * v.y + m[1][2] * v.z,
            m[2][0] * v.x + m[2][1] * v.y + m[2][2] * v.z,
        )
    }

    pub fn transpose(&self) -> Mat3 {
        let m = &self.0;
        Mat3([
            [m[0][0], m[1][0], m[2][0]],
            [m[0][1], m[1][1], m[2][1]],
            [m[0][2], m[1][2], m[2][2]],
        ])
    }
}

/// Wrap an angle to `(-pi, pi]`.
pub fn wrap_pi(a: f64) -> f64 {
    let r = (a + PI).rem_euclid(TAU) - PI;
    if r <= -PI { r + TAU } else { r }
}

/// Wrap an angle in degrees to `[0, 360)`.
pub fn wrap_360(deg: f64) -> f64 {
    let r = deg.rem_euclid(360.0);
    if r >= 360.0 { 0.0 } else { r }
}

/// Compass heading (deg, clockwise from North) to yaw (rad, CCW from East).
pub fn heading_deg_to_yaw(heading_deg: f64) -> f64 {
    wrap_pi((90.0 - heading_deg).to_radians())
}

/// Yaw (rad, CCW from East) to compass heading (deg, clockwise from North, `[0, 360)`).
pub fn yaw_to_heading_deg(yaw: f64) -> f64 {
    wrap_360(90.0 - yaw.to_degrees())
}

/// Horizontal wind velocity (ENU, m/s) from a meteorological "blowing from" direction.
pub fn wind_vector_from(speed_m_s: f64, from_heading_deg: f64) -> Vec2 {
    let a = from_heading_deg.to_radians();
    Vec2::new(-speed_m_s * a.sin(), -speed_m_s * a.cos())
}

/// Meteorological "blowing from" compass direction (deg) of a horizontal wind vector.
pub fn wind_from_direction_deg(w: Vec2) -> f64 {
    if w.norm() < 1e-9 {
        return 0.0;
    }
    // The wind blows toward atan2(x, y) (compass); it comes from the opposite direction.
    wrap_360(w.x.atan2(w.y).to_degrees() + 180.0)
}

/// Linear interpolation.
pub fn lerp(a: f64, b: f64, s: f64) -> f64 {
    a + (b - a) * s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_yaw_roundtrip() {
        for h in [0.0, 45.0, 90.0, 180.0, 270.0, 359.0] {
            let y = heading_deg_to_yaw(h);
            assert!((yaw_to_heading_deg(y) - h).abs() < 1e-9, "heading {h}");
        }
        // North heading is the +y direction in ENU.
        let d = Vec2::from_angle(heading_deg_to_yaw(0.0));
        assert!(d.x.abs() < 1e-12 && (d.y - 1.0).abs() < 1e-12);
        // East heading is the +x direction.
        let d = Vec2::from_angle(heading_deg_to_yaw(90.0));
        assert!((d.x - 1.0).abs() < 1e-12 && d.y.abs() < 1e-12);
    }

    #[test]
    fn wind_direction_convention() {
        let w = wind_vector_from(5.0, 0.0); // northerly wind blows toward South
        assert!(w.x.abs() < 1e-12);
        assert!((w.y + 5.0).abs() < 1e-12);
        let w = wind_vector_from(3.0, 270.0); // westerly wind blows toward East
        assert!((w.x - 3.0).abs() < 1e-12);
        for from in [0.0, 30.0, 135.0, 270.0] {
            let w = wind_vector_from(4.0, from);
            assert!((wind_from_direction_deg(w) - from).abs() < 1e-9);
        }
    }

    #[test]
    fn rotation_conventions() {
        // Nose-up pitch rotates the body x-axis upward.
        let r = Mat3::body_to_world(0.0, 0.3, 0.0);
        let nose = r.mul_vec(Vec3::new(1.0, 0.0, 0.0));
        assert!(nose.z > 0.0);
        // Positive roll lowers the right (-y) side.
        let r = Mat3::body_to_world(0.0, 0.0, 0.2);
        let right = r.mul_vec(Vec3::new(0.0, -1.0, 0.0));
        assert!(right.z < 0.0);
        // Positive yaw turns the nose from East toward North.
        let r = Mat3::body_to_world(0.5, 0.0, 0.0);
        assert!(r.mul_vec(Vec3::new(1.0, 0.0, 0.0)).y > 0.0);
        // Rotation is orthonormal.
        let r = Mat3::body_to_world(1.1, -0.2, 0.3);
        let i = r.mul_mat(&r.transpose());
        for a in 0..3 {
            for b in 0..3 {
                let e = if a == b { 1.0 } else { 0.0 };
                assert!((i.0[a][b] - e).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn wrap_ranges() {
        assert!((wrap_pi(3.0 * PI) - PI).abs() < 1e-12);
        assert!((wrap_pi(-3.0 * PI) - PI).abs() < 1e-12);
        assert!((wrap_pi(0.5) - 0.5).abs() < 1e-12);
        assert!((wrap_360(-10.0) - 350.0).abs() < 1e-12);
        assert_eq!(wrap_360(720.0), 0.0);
    }
}
