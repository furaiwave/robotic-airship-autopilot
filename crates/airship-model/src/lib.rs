//! # airship-model
//!
//! Vehicle parameters, derived physical properties, the atmosphere model and the
//! small linear-algebra toolkit shared by every other crate of the simulator.
//!
//! ## Coordinate frames and sign conventions
//!
//! * **World frame (ENU):** `x` points East, `y` points North, `z` points **up**.
//!   `z` is the altitude above mean sea level (MSL) of the vehicle reference point,
//!   the centre of volume (CV) of the hull.
//! * **Body frame (FLU):** `x` forward along the hull axis, `y` to the left (port),
//!   `z` up through the envelope.
//! * **Yaw** `psi` (rad) is measured counter-clockwise from world `+x` (East).
//!   The user-facing *compass heading* is clockwise from North: `heading = 90 deg - psi`.
//! * **Pitch** `theta` is positive nose-up; **roll** `phi` is positive right side down.
//! * The body-to-world rotation is `R = Rz(psi) * Ry(-theta) * Rx(phi)`.
//!
//! All internal quantities are SI (m, s, kg, N, rad). Configuration structures that
//! are exchanged with the user carry the unit in the field name (`_deg`, `_m_s`, ...).

pub mod atmosphere;
pub mod derived;
pub mod math;
pub mod params;
pub mod validation;

pub use atmosphere::{AtmosphereConfig, AtmosphereModel};
pub use derived::{DerivedProperties, lamb_coefficients};
pub use math::{Mat3, Vec2, Vec3};
pub use params::{
    AeroParams, AirshipParams, LiftingGas, OperatingLimits, PropulsionParams, RudderParams,
};
pub use validation::{Severity, ValidationIssue, ValidationReport};

/// Standard gravitational acceleration (m/s^2).
pub const G0: f64 = 9.806_65;
