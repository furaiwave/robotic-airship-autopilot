//! # flight-dynamics
//!
//! Physical model of the robotic airship: forces and moments, the equations of
//! motion, the fixed-step RK4 integrator, actuator dynamics, ground contact,
//! the simulated wind environment and the sensor (measurement) model.
//!
//! This crate contains **no guidance, control or flight-phase logic**.

pub mod actuators;
pub mod dynamics;
pub mod environment;
pub mod ground;
pub mod integrator;
pub mod rng;
pub mod sensors;
pub mod state;

pub use actuators::{ActuatorCommand, ActuatorLimits, ActuatorState, thruster_power};
pub use dynamics::{AirshipDynamics, ForceBreakdown};
pub use environment::{
    DiscreteGust, DisturbanceTrigger, EnvironmentConfig, InitialStateConfig, LandingArea,
    SensorNoiseConfig, TurbulenceConfig, WindChange, WindConfig, WindField,
};
pub use ground::GroundContactParams;
pub use sensors::{Measurement, SensorModel, TruthSnapshot};
pub use state::VehicleState;
