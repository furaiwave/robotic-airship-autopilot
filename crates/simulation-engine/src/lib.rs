//! # simulation-engine
//!
//! Owns the authoritative simulation: the fixed-step loop that couples the physical
//! model, wind field, sensors and autopilot; telemetry recording; landing assessment;
//! performance metrics; predefined scenarios and experiments; and controller
//! comparison under identical conditions.
//!
//! All results produced by this crate are **simulated data** from a simplified model.

pub mod compare;
pub mod config;
pub mod engine;
pub mod landing;
pub mod metrics;
pub mod presets;
pub mod telemetry;

pub use compare::{ComparisonReport, ComparisonRun, compare_runs, config_differences};
pub use config::{SimulationConfig, SimulationSettings, config_hash};
pub use engine::{FinishReason, Simulation, SimulationError, SimulationStatus};
pub use landing::{LandingAssessment, LandingClass, TouchdownRecord};
pub use metrics::{Metric, ResultsReport};
pub use presets::{Experiment, ExperimentRun, Scenario, experiments, scenarios};
pub use telemetry::{TelemetrySample, telemetry_csv};

/// Disclaimer attached to every result and export.
pub const DISCLAIMER: &str = "Simulated data from a simplified, unvalidated airship model for academic research. \
It does not demonstrate real-world flight performance, airworthiness or flight readiness.";
