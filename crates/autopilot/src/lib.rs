//! # autopilot
//!
//! Guidance (waypoint path following, final-approach geometry), the explicit
//! take-off and landing state machine, preflight checks and the onboard safety
//! monitor. The autopilot only sees [`flight_dynamics::Measurement`]s; it never
//! reads the true simulated state.

pub mod config;
pub mod flight_path;
pub mod guidance;
pub mod phase;
pub mod preflight;
pub mod safety;
pub mod state_machine;

pub use config::{
    ApproachConfig, AutopilotConfig, ControlLimitFractions, GuidanceConfig, LandingConfig,
    SafetyConfig, TakeoffConfig, TransitionTolerances,
};
pub use flight_path::{FlightPath, PathTemplate, Waypoint, path_template, validate_flight_path};
pub use guidance::{ApproachGeometry, PathFollower};
pub use phase::{FlightPhase, PhaseTransition, StateSnapshot};
pub use preflight::PreflightCheck;
pub use safety::{SafetyConstraint, SafetyEvent, SafetyMonitor, SafetyResponse, SafetySeverity};
pub use state_machine::{Autopilot, AutopilotOutput, GuidanceStatus, MooringCommand};
