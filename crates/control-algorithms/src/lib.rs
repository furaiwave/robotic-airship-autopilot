//! # control-algorithms
//!
//! Low-level control building blocks and the two flight controllers:
//!
//! * **Controller A - cascaded PID** ([`CascadedPidController`]): fixed, user-configured gains.
//! * **Controller B - gain-scheduled PID** ([`GainScheduledController`]): the same loop
//!   structure, but every gain is recomputed at each control step from an explicit
//!   scheduling law of airspeed, flight-phase class and distance to the landing point.
//!
//! Both controllers share [`cascade::CascadeCore`] (loop structure, feed-forward,
//! wind-triangle conversion) and [`allocation::ControlAllocator`], so any difference in
//! their results is attributable to the gain law alone.

pub mod allocation;
pub mod cascade;
pub mod controller;
pub mod filters;
pub mod pid;
pub mod schedule;

pub use allocation::{AllocationResult, ControlAllocator, VirtualCommand};
pub use cascade::{
    CascadeCore, ControlInput, ControlMeasurement, ControlModel, ControlOutput, ControlReferences,
    ForceLimits, HorizontalReference, InternalReferences, LoopGains, PhaseClass, Saturation,
    VerticalReference,
};
pub use controller::{
    CascadedPidConfig, CascadedPidController, CommonControlSettings, ControllerConfig,
    ControllerKind, FlightController, GainScheduledController, build_controller,
};
pub use filters::LowPass;
pub use pid::{Pid, PidGains, PidLimits, PidTerms};
pub use schedule::{Bandwidths, GainScheduleConfig, PhaseBandwidthTable, scheduled_gains};
