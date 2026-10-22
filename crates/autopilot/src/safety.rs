//! Onboard constraint monitoring.
//!
//! Each monitored constraint has a level: `ok`, `warning` (limit exceeded) or
//! `critical` (limit times `critical_factor` exceeded). An event is recorded when a
//! constraint's level *rises*; when it returns to `ok` the open event is marked as
//! cleared with its time. Critical events may request a response from the state
//! machine (go-around, proceed to landing). Fatal events (numerical failure, ground
//! collision) are raised by the simulation engine and end the run.
//!
//! These software checks are part of a simulation study. They do not replace
//! certified flight-control software, independent flight-termination systems or
//! hardware safety mechanisms.

use airship_model::OperatingLimits;
use flight_dynamics::Measurement;
use serde::{Deserialize, Serialize};

use crate::config::SafetyConfig;
use crate::phase::{FlightPhase, StateSnapshot};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetySeverity {
    Info,
    Warning,
    Critical,
    Fatal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetyConstraint {
    Airspeed,
    VerticalSpeed,
    Pitch,
    Roll,
    MinimumAltitude,
    ActuatorSaturation,
    Battery,
    ApproachFeasibility,
    LandingZone,
    TouchdownSpeed,
    UnplannedGroundContact,
    GroundCollision,
    NumericalValidity,
    TakeoffPerformance,
}

impl SafetyConstraint {
    pub fn description(self) -> &'static str {
        match self {
            SafetyConstraint::Airspeed => "airspeed above the vehicle limit",
            SafetyConstraint::VerticalSpeed => "vertical speed above the climb/descent limit",
            SafetyConstraint::Pitch => "pitch angle above the permitted attitude",
            SafetyConstraint::Roll => "roll angle above the permitted attitude",
            SafetyConstraint::MinimumAltitude => "height below the minimum en-route altitude",
            SafetyConstraint::ActuatorSaturation => "actuators continuously saturated",
            SafetyConstraint::Battery => "battery state of charge low",
            SafetyConstraint::ApproachFeasibility => "approach conditions not feasible",
            SafetyConstraint::LandingZone => "touchdown outside the landing zone",
            SafetyConstraint::TouchdownSpeed => "touchdown speed above the limit",
            SafetyConstraint::UnplannedGroundContact => {
                "ground contact outside the landing sequence"
            }
            SafetyConstraint::GroundCollision => "ground impact beyond the structural limit",
            SafetyConstraint::NumericalValidity => "numerical state invalid or divergent",
            SafetyConstraint::TakeoffPerformance => "insufficient take-off performance",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetyResponse {
    /// Record the event only.
    LogOnly,
    /// Abandon the approach and climb to the approach altitude.
    GoAround,
    /// Skip the remaining waypoints and start the approach.
    ProceedToLanding,
    /// Stop the simulation (fatal).
    Terminate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SafetyEvent {
    pub time_s: f64,
    pub severity: SafetySeverity,
    pub constraint: SafetyConstraint,
    pub message: String,
    pub phase: FlightPhase,
    /// Measured value and the limit it was compared against (units in `message`).
    pub value: f64,
    pub limit: f64,
    pub response: SafetyResponse,
    pub state: StateSnapshot,
    pub cleared_at_s: Option<f64>,
}

#[derive(Debug, Clone, Default)]
pub struct SafetyMonitor {
    active: Vec<(SafetyConstraint, SafetySeverity, usize)>,
    pub events: Vec<SafetyEvent>,
    saturation_since: Option<f64>,
}

/// Inputs of one monitoring step.
pub struct SafetyInputs<'a> {
    pub meas: &'a Measurement,
    pub phase: FlightPhase,
    pub saturated: bool,
    pub distance_to_landing_m: f64,
    pub decision_height_m: f64,
    pub limits: &'a OperatingLimits,
    pub cfg: &'a SafetyConfig,
}

impl SafetyMonitor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an externally detected event (touchdown assessment, engine checks).
    pub fn push(&mut self, event: SafetyEvent) {
        self.events.push(event);
    }

    pub fn count(&self, severity: SafetySeverity) -> usize {
        self.events
            .iter()
            .filter(|e| e.severity == severity)
            .count()
    }

    #[allow(clippy::too_many_arguments)]
    fn level(
        &mut self,
        inp: &SafetyInputs,
        c: SafetyConstraint,
        value: f64,
        limit: f64,
        unit: &str,
        enabled: bool,
    ) -> Option<SafetyEvent> {
        let t = inp.meas.time_s;
        let sev = if !enabled || value <= limit {
            None
        } else if value > limit * inp.cfg.critical_factor {
            Some(SafetySeverity::Critical)
        } else {
            Some(SafetySeverity::Warning)
        };
        self.update(
            inp,
            c,
            sev,
            value,
            limit,
            format!(
                "{}: {:.2} {unit} > {:.2} {unit}",
                c.description(),
                value,
                limit
            ),
            t,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn update(
        &mut self,
        inp: &SafetyInputs,
        c: SafetyConstraint,
        sev: Option<SafetySeverity>,
        value: f64,
        limit: f64,
        message: String,
        t: f64,
    ) -> Option<SafetyEvent> {
        let pos = self.active.iter().position(|(k, _, _)| *k == c);
        match (sev, pos) {
            (None, Some(i)) => {
                let (_, _, idx) = self.active.remove(i);
                self.events[idx].cleared_at_s = Some(t);
                None
            }
            (None, None) => None,
            (Some(s), Some(i)) if s <= self.active[i].1 => None,
            (Some(s), pos) => {
                if let Some(i) = pos {
                    let (_, _, idx) = self.active.remove(i);
                    self.events[idx].cleared_at_s = Some(t);
                }
                let response = response_for(inp, c, s);
                let e = SafetyEvent {
                    time_s: t,
                    severity: s,
                    constraint: c,
                    message,
                    phase: inp.phase,
                    value,
                    limit,
                    response,
                    state: StateSnapshot::from_measurement(inp.meas, inp.distance_to_landing_m),
                    cleared_at_s: None,
                };
                self.events.push(e.clone());
                self.active.push((c, s, self.events.len() - 1));
                Some(e)
            }
        }
    }

    /// Evaluate all onboard constraints. Returns the newly raised events.
    pub fn evaluate(&mut self, inp: &SafetyInputs) -> Vec<SafetyEvent> {
        let m = inp.meas;
        let lim = inp.limits;
        let phase = inp.phase;
        let airborne = phase.is_airborne();
        let mut raised = Vec::new();

        let airspeed = m.air_velocity_body.horizontal().norm();
        raised.extend(self.level(
            inp,
            SafetyConstraint::Airspeed,
            airspeed,
            lim.max_airspeed_m_s,
            "m/s",
            airborne,
        ));
        let (vz, vlim) = if m.velocity.z >= 0.0 {
            (m.velocity.z, lim.max_climb_rate_m_s)
        } else {
            (-m.velocity.z, lim.max_descent_rate_m_s)
        };
        raised.extend(self.level(
            inp,
            SafetyConstraint::VerticalSpeed,
            vz,
            vlim,
            "m/s",
            airborne,
        ));
        let on_ground = m.moored || m.ground_contact;
        raised.extend(self.level(
            inp,
            SafetyConstraint::Pitch,
            m.pitch.to_degrees().abs(),
            lim.max_pitch_deg,
            "deg",
            !on_ground,
        ));
        raised.extend(self.level(
            inp,
            SafetyConstraint::Roll,
            m.roll.to_degrees().abs(),
            lim.max_roll_deg,
            "deg",
            !on_ground,
        ));

        let enroute = matches!(
            phase,
            FlightPhase::Climb | FlightPhase::Cruise | FlightPhase::Approach
        );
        let min_alt = inp.cfg.min_enroute_altitude_m;
        let alt_sev = (enroute && m.altitude_agl_m < min_alt).then_some(SafetySeverity::Critical);
        let alt_msg = format!(
            "{}: {:.1} m < {:.1} m",
            SafetyConstraint::MinimumAltitude.description(),
            m.altitude_agl_m,
            min_alt
        );
        raised.extend(self.update(
            inp,
            SafetyConstraint::MinimumAltitude,
            alt_sev,
            m.altitude_agl_m,
            min_alt,
            alt_msg,
            m.time_s,
        ));

        let sat_time = if inp.saturated && airborne {
            m.time_s - *self.saturation_since.get_or_insert(m.time_s)
        } else {
            self.saturation_since = None;
            0.0
        };
        let sat_limit = inp.cfg.saturation_warning_time_s;
        let sat_sev = (sat_time >= sat_limit).then_some(SafetySeverity::Warning);
        let sat_msg = format!(
            "{} for {:.1} s (limit {:.1} s): requested manoeuvre exceeds control authority",
            SafetyConstraint::ActuatorSaturation.description(),
            sat_time,
            sat_limit
        );
        raised.extend(self.update(
            inp,
            SafetyConstraint::ActuatorSaturation,
            sat_sev,
            sat_time,
            sat_limit,
            sat_msg,
            m.time_s,
        ));

        let soc = m.battery_soc;
        let bat_sev = if soc <= inp.cfg.battery_critical_soc {
            Some(SafetySeverity::Critical)
        } else if soc <= inp.cfg.battery_warning_soc {
            Some(SafetySeverity::Warning)
        } else {
            None
        };
        let bat_msg = format!(
            "{}: {:.0} %",
            SafetyConstraint::Battery.description(),
            soc * 100.0
        );
        raised.extend(self.update(
            inp,
            SafetyConstraint::Battery,
            bat_sev,
            soc,
            inp.cfg.battery_warning_soc,
            bat_msg,
            m.time_s,
        ));
        raised
    }

    /// Strongest response requested by a set of events.
    pub fn strongest_response(events: &[SafetyEvent]) -> SafetyResponse {
        let rank = |r: SafetyResponse| match r {
            SafetyResponse::LogOnly => 0,
            SafetyResponse::ProceedToLanding => 1,
            SafetyResponse::GoAround => 2,
            SafetyResponse::Terminate => 3,
        };
        events
            .iter()
            .map(|e| e.response)
            .max_by_key(|r| rank(*r))
            .unwrap_or(SafetyResponse::LogOnly)
    }
}

fn response_for(inp: &SafetyInputs, c: SafetyConstraint, s: SafetySeverity) -> SafetyResponse {
    if s < SafetySeverity::Critical {
        return SafetyResponse::LogOnly;
    }
    let phase = inp.phase;
    if c == SafetyConstraint::Battery && matches!(phase, FlightPhase::Climb | FlightPhase::Cruise) {
        return SafetyResponse::ProceedToLanding;
    }
    if inp.cfg.go_around_on_critical
        && phase.is_landing_sequence()
        && inp.meas.altitude_agl_m > inp.decision_height_m
        && c != SafetyConstraint::Battery
    {
        return SafetyResponse::GoAround;
    }
    SafetyResponse::LogOnly
}

#[cfg(test)]
mod tests {
    use super::*;
    use airship_model::Vec3;

    fn meas(t: f64, vx: f64, agl: f64) -> Measurement {
        Measurement {
            time_s: t,
            altitude_agl_m: agl,
            velocity: Vec3::new(vx, 0.0, 0.0),
            air_velocity_body: Vec3::new(vx, 0.0, 0.0),
            battery_soc: 0.9,
            ..Default::default()
        }
    }

    fn eval(m: &mut SafetyMonitor, me: &Measurement, phase: FlightPhase) -> Vec<SafetyEvent> {
        let lim = OperatingLimits::default();
        let cfg = SafetyConfig::default();
        m.evaluate(&SafetyInputs {
            meas: me,
            phase,
            saturated: false,
            distance_to_landing_m: 100.0,
            decision_height_m: 1.5,
            limits: &lim,
            cfg: &cfg,
        })
    }

    #[test]
    fn airspeed_warning_escalates_and_clears() {
        let mut s = SafetyMonitor::new();
        assert!(eval(&mut s, &meas(0.0, 5.0, 40.0), FlightPhase::Cruise).is_empty());
        let e = eval(&mut s, &meas(1.0, 11.0, 40.0), FlightPhase::Cruise);
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].severity, SafetySeverity::Warning);
        // Persisting at the same level raises nothing new.
        assert!(eval(&mut s, &meas(2.0, 11.0, 40.0), FlightPhase::Cruise).is_empty());
        let e = eval(&mut s, &meas(3.0, 14.0, 40.0), FlightPhase::Cruise);
        assert_eq!(e[0].severity, SafetySeverity::Critical);
        eval(&mut s, &meas(4.0, 5.0, 40.0), FlightPhase::Cruise);
        assert!(s.events.iter().all(|e| e.cleared_at_s.is_some()));
        assert_eq!(s.events.len(), 2);
    }

    #[test]
    fn critical_event_in_approach_requests_go_around() {
        let mut s = SafetyMonitor::new();
        let e = eval(&mut s, &meas(0.0, 14.0, 20.0), FlightPhase::Descent);
        assert_eq!(e[0].response, SafetyResponse::GoAround);
        // Below the decision height the landing is committed.
        let mut s = SafetyMonitor::new();
        let e = eval(&mut s, &meas(0.0, 14.0, 1.0), FlightPhase::Flare);
        assert_eq!(e[0].response, SafetyResponse::LogOnly);
    }

    #[test]
    fn minimum_altitude_only_enroute() {
        let mut s = SafetyMonitor::new();
        assert!(eval(&mut s, &meas(0.0, 3.0, 5.0), FlightPhase::Descent).is_empty());
        let e = eval(&mut s, &meas(1.0, 3.0, 5.0), FlightPhase::Cruise);
        assert_eq!(e[0].constraint, SafetyConstraint::MinimumAltitude);
    }

    #[test]
    fn low_battery_in_cruise_proceeds_to_landing() {
        let mut s = SafetyMonitor::new();
        let mut m = meas(0.0, 3.0, 40.0);
        m.battery_soc = 0.1;
        let e = eval(&mut s, &m, FlightPhase::Cruise);
        assert_eq!(
            SafetyMonitor::strongest_response(&e),
            SafetyResponse::ProceedToLanding
        );
    }
}
