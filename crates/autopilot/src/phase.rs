//! Flight phases and the transition log.

use airship_model::math::yaw_to_heading_deg;
use control_algorithms::PhaseClass;
use flight_dynamics::Measurement;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlightPhase {
    Preflight,
    TakeoffPreparation,
    Takeoff,
    InitialClimb,
    Climb,
    Cruise,
    Approach,
    Descent,
    Flare,
    Touchdown,
    PostLanding,
    Landed,
    GoAround,
    Aborted,
}

impl FlightPhase {
    pub const ALL: [FlightPhase; 14] = [
        FlightPhase::Preflight,
        FlightPhase::TakeoffPreparation,
        FlightPhase::Takeoff,
        FlightPhase::InitialClimb,
        FlightPhase::Climb,
        FlightPhase::Cruise,
        FlightPhase::Approach,
        FlightPhase::Descent,
        FlightPhase::Flare,
        FlightPhase::Touchdown,
        FlightPhase::PostLanding,
        FlightPhase::Landed,
        FlightPhase::GoAround,
        FlightPhase::Aborted,
    ];

    pub fn label(self) -> &'static str {
        match self {
            FlightPhase::Preflight => "Preflight checks",
            FlightPhase::TakeoffPreparation => "Take-off preparation",
            FlightPhase::Takeoff => "Take-off",
            FlightPhase::InitialClimb => "Initial climb",
            FlightPhase::Climb => "Climb to target altitude",
            FlightPhase::Cruise => "En-route path following",
            FlightPhase::Approach => "Approach",
            FlightPhase::Descent => "Descent on glide path",
            FlightPhase::Flare => "Flare / final descent",
            FlightPhase::Touchdown => "Touchdown",
            FlightPhase::PostLanding => "Post-landing stabilisation",
            FlightPhase::Landed => "Landed and moored",
            FlightPhase::GoAround => "Go-around",
            FlightPhase::Aborted => "Aborted",
        }
    }

    /// Phase class used by gain scheduling.
    pub fn class(self) -> PhaseClass {
        match self {
            FlightPhase::Preflight
            | FlightPhase::TakeoffPreparation
            | FlightPhase::Touchdown
            | FlightPhase::PostLanding
            | FlightPhase::Landed => PhaseClass::Ground,
            FlightPhase::Takeoff | FlightPhase::InitialClimb => PhaseClass::Takeoff,
            FlightPhase::Climb | FlightPhase::GoAround | FlightPhase::Aborted => PhaseClass::Climb,
            FlightPhase::Cruise => PhaseClass::Cruise,
            FlightPhase::Approach => PhaseClass::Approach,
            FlightPhase::Descent | FlightPhase::Flare => PhaseClass::Landing,
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, FlightPhase::Landed | FlightPhase::Aborted)
    }

    /// Phases in which the vehicle is intended to be airborne.
    pub fn is_airborne(self) -> bool {
        matches!(
            self,
            FlightPhase::InitialClimb
                | FlightPhase::Climb
                | FlightPhase::Cruise
                | FlightPhase::Approach
                | FlightPhase::Descent
                | FlightPhase::Flare
                | FlightPhase::GoAround
        )
    }

    /// Phases of the landing sequence in which a go-around is possible.
    pub fn is_landing_sequence(self) -> bool {
        matches!(
            self,
            FlightPhase::Approach | FlightPhase::Descent | FlightPhase::Flare
        )
    }
}

/// Compact snapshot of the measured state attached to transitions and events.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub x_m: f64,
    pub y_m: f64,
    pub altitude_agl_m: f64,
    pub ground_speed_m_s: f64,
    pub vertical_speed_m_s: f64,
    pub airspeed_m_s: f64,
    pub heading_deg: f64,
    pub pitch_deg: f64,
    pub roll_deg: f64,
    pub distance_to_landing_m: f64,
}

impl StateSnapshot {
    pub fn from_measurement(m: &Measurement, distance_to_landing_m: f64) -> Self {
        Self {
            x_m: m.position.x,
            y_m: m.position.y,
            altitude_agl_m: m.altitude_agl_m,
            ground_speed_m_s: m.velocity.horizontal().norm(),
            vertical_speed_m_s: m.velocity.z,
            airspeed_m_s: m.air_velocity_body.norm(),
            heading_deg: yaw_to_heading_deg(m.yaw),
            pitch_deg: m.pitch.to_degrees(),
            roll_deg: m.roll.to_degrees(),
            distance_to_landing_m,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhaseTransition {
    pub time_s: f64,
    pub from: FlightPhase,
    pub to: FlightPhase,
    /// Human-readable explanation of the condition that triggered the transition.
    pub reason: String,
    pub state: StateSnapshot,
}
