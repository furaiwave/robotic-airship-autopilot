//! Continuous vehicle state.

use airship_model::{Vec2, Vec3};
use serde::{Deserialize, Serialize};

/// Number of continuous state variables integrated by RK4.
pub const STATE_DIM: usize = 12;

/// Rigid-body state of the airship reference point (centre of volume).
///
/// Translational states are expressed in the world ENU frame. Attitude uses
/// yaw/pitch/roll angles with the conventions of `airship_model` (pitch and roll
/// remain small by construction of the pendulum-stable configuration).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct VehicleState {
    /// Position (m): East, North, altitude MSL.
    pub position: Vec3,
    /// Velocity relative to the ground (m/s).
    pub velocity: Vec3,
    /// Yaw angle `psi` (rad, CCW from East).
    pub yaw: f64,
    /// Yaw rate `r` (rad/s).
    pub yaw_rate: f64,
    /// Pitch angle `theta` (rad, nose-up positive).
    pub pitch: f64,
    /// Pitch rate `q` (rad/s, nose-up positive).
    pub pitch_rate: f64,
    /// Roll angle `phi` (rad, right side down positive).
    pub roll: f64,
    /// Roll rate `p` (rad/s).
    pub roll_rate: f64,
}

impl VehicleState {
    pub fn to_array(&self) -> [f64; STATE_DIM] {
        [
            self.position.x,
            self.position.y,
            self.position.z,
            self.velocity.x,
            self.velocity.y,
            self.velocity.z,
            self.yaw,
            self.yaw_rate,
            self.pitch,
            self.pitch_rate,
            self.roll,
            self.roll_rate,
        ]
    }

    pub fn from_array(a: &[f64; STATE_DIM]) -> Self {
        Self {
            position: Vec3::new(a[0], a[1], a[2]),
            velocity: Vec3::new(a[3], a[4], a[5]),
            yaw: a[6],
            yaw_rate: a[7],
            pitch: a[8],
            pitch_rate: a[9],
            roll: a[10],
            roll_rate: a[11],
        }
    }

    pub fn is_finite(&self) -> bool {
        self.to_array().iter().all(|v| v.is_finite())
    }

    pub fn horizontal_position(&self) -> Vec2 {
        self.position.horizontal()
    }

    pub fn ground_speed(&self) -> f64 {
        self.velocity.horizontal().norm()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn array_roundtrip() {
        let s = VehicleState {
            position: Vec3::new(1.0, 2.0, 3.0),
            velocity: Vec3::new(4.0, 5.0, 6.0),
            yaw: 7.0,
            yaw_rate: 8.0,
            pitch: 9.0,
            pitch_rate: 10.0,
            roll: 11.0,
            roll_rate: 12.0,
        };
        assert_eq!(VehicleState::from_array(&s.to_array()), s);
    }
}
