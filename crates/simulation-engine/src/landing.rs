//! Touchdown records and landing classification.
//!
//! A landing is **successful** only if all four criteria hold:
//!
//! 1. *Target area reached* - the gear came within the landing-point tolerance of the
//!    target while below `flare height + 1 m`.
//! 2. *Ground contact* - the ground-contact model registered contact during the landing
//!    sequence (not merely proximity to the point).
//! 3. *Touchdown within limits* - at the first contact of the final attempt the
//!    distance to the target, the vertical and horizontal speeds are within their limits
//!    and the contact point lies inside the prepared landing area.
//! 4. *Sequence completed safely* - the state machine reached `Landed` (settled and
//!    moored) and no fatal or critical safety event occurred during the final approach.

use airship_model::Vec2;
use autopilot::{FlightPhase, LandingConfig, PhaseTransition, SafetyEvent, SafetySeverity};
use flight_dynamics::LandingArea;
use serde::{Deserialize, Serialize};

/// Truth state at the onset of a ground contact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TouchdownRecord {
    pub time_s: f64,
    pub phase: FlightPhase,
    pub x_m: f64,
    pub y_m: f64,
    pub distance_to_target_m: f64,
    /// Along-axis / cross-axis position relative to the landing point (m).
    pub along_axis_m: f64,
    pub cross_axis_m: f64,
    pub horizontal_speed_m_s: f64,
    /// Vertical speed at contact (m/s, negative = descending).
    pub vertical_speed_m_s: f64,
    pub inside_landing_area: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LandingClass {
    Successful,
    Unsuccessful,
    NotAttempted,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Criterion {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LandingAssessment {
    pub classification: LandingClass,
    pub reached_target_area: bool,
    pub ground_contact: bool,
    pub touchdown_within_limits: bool,
    pub sequence_completed: bool,
    pub criteria: Vec<Criterion>,
    pub touchdown: Option<TouchdownRecord>,
    pub explanation: String,
}

pub struct AssessmentInput<'a> {
    pub landing: &'a LandingConfig,
    pub area: &'a LandingArea,
    pub target: Vec2,
    pub touchdowns: &'a [TouchdownRecord],
    pub transitions: &'a [PhaseTransition],
    pub events: &'a [SafetyEvent],
    pub final_phase: FlightPhase,
    pub reached_target_area: bool,
}

pub fn assess(inp: &AssessmentInput) -> LandingAssessment {
    let l = inp.landing;
    let last_approach = inp
        .transitions
        .iter()
        .rev()
        .find(|t| t.to == FlightPhase::Approach)
        .map(|t| t.time_s);
    let landing_contacts: Vec<&TouchdownRecord> = inp
        .touchdowns
        .iter()
        .filter(|t| {
            last_approach.is_some_and(|a| t.time_s >= a)
                || matches!(t.phase, FlightPhase::Descent | FlightPhase::Flare)
        })
        .collect();
    let td = landing_contacts.first().map(|t| (*t).clone());

    let mut criteria = Vec::new();
    criteria.push(Criterion {
        name: "Target area reached".into(),
        passed: inp.reached_target_area,
        detail: if inp.reached_target_area {
            format!(
                "came within {:.1} m of the target below flare height",
                l.landing_point_tolerance_m
            )
        } else {
            format!(
                "never within {:.1} m of the target below flare height",
                l.landing_point_tolerance_m
            )
        },
    });
    let contact = td.is_some();
    criteria.push(Criterion {
        name: "Ground contact".into(),
        passed: contact,
        detail: match &td {
            Some(t) => format!(
                "contact at t = {:.1} s in phase {}",
                t.time_s,
                t.phase.label()
            ),
            None => "no ground contact during the landing sequence".into(),
        },
    });
    let within = match &td {
        Some(t) => {
            let mut fails = Vec::new();
            if t.distance_to_target_m > l.landing_point_tolerance_m {
                fails.push(format!(
                    "distance {:.1} m > {:.1} m",
                    t.distance_to_target_m, l.landing_point_tolerance_m
                ));
            }
            if -t.vertical_speed_m_s > l.max_touchdown_vertical_speed_m_s {
                fails.push(format!(
                    "sink rate {:.2} m/s > {:.2} m/s",
                    -t.vertical_speed_m_s, l.max_touchdown_vertical_speed_m_s
                ));
            }
            if t.horizontal_speed_m_s > l.max_touchdown_horizontal_speed_m_s {
                fails.push(format!(
                    "horizontal speed {:.2} m/s > {:.2} m/s",
                    t.horizontal_speed_m_s, l.max_touchdown_horizontal_speed_m_s
                ));
            }
            if !t.inside_landing_area {
                fails.push(format!(
                    "outside the prepared landing area (along {:+.1} m, cross {:+.1} m)",
                    t.along_axis_m, t.cross_axis_m
                ));
            }
            let ok = fails.is_empty();
            criteria.push(Criterion {
                name: "Touchdown within limits".into(),
                passed: ok,
                detail: if ok {
                    format!(
                        "distance {:.2} m, sink rate {:.2} m/s, horizontal speed {:.2} m/s, inside landing area",
                        t.distance_to_target_m, -t.vertical_speed_m_s, t.horizontal_speed_m_s
                    )
                } else {
                    fails.join("; ")
                },
            });
            ok
        }
        None => {
            criteria.push(Criterion {
                name: "Touchdown within limits".into(),
                passed: false,
                detail: "no touchdown".into(),
            });
            false
        }
    };
    let since = last_approach.unwrap_or(0.0);
    let serious: Vec<&SafetyEvent> = inp
        .events
        .iter()
        .filter(|e| e.time_s >= since && e.severity >= SafetySeverity::Critical)
        .collect();
    let landed = inp.final_phase == FlightPhase::Landed;
    let completed = landed && serious.is_empty();
    criteria.push(Criterion {
        name: "Sequence completed safely".into(),
        passed: completed,
        detail: if completed {
            "settled, moored, no critical or fatal events during the final approach".into()
        } else if !landed {
            format!("final phase: {}", inp.final_phase.label())
        } else {
            format!(
                "{} critical/fatal event(s) during the final approach: {}",
                serious.len(),
                serious
                    .iter()
                    .map(|e| e.message.clone())
                    .collect::<Vec<_>>()
                    .join("; ")
            )
        },
    });

    let attempted = last_approach.is_some() || contact;
    let classification = if inp.reached_target_area && contact && within && completed {
        LandingClass::Successful
    } else if attempted {
        LandingClass::Unsuccessful
    } else {
        LandingClass::NotAttempted
    };
    let explanation = match classification {
        LandingClass::Successful => "All four landing criteria satisfied.".to_string(),
        LandingClass::NotAttempted => format!(
            "No landing was attempted; the mission ended in phase {}.",
            inp.final_phase.label()
        ),
        LandingClass::Unsuccessful => {
            let failed: Vec<String> = criteria
                .iter()
                .filter(|c| !c.passed)
                .map(|c| format!("{}: {}", c.name, c.detail))
                .collect();
            format!("Landing failed. {}", failed.join(" | "))
        }
    };
    LandingAssessment {
        classification,
        reached_target_area: inp.reached_target_area,
        ground_contact: contact,
        touchdown_within_limits: within,
        sequence_completed: completed,
        criteria,
        touchdown: td,
        explanation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use autopilot::StateSnapshot;

    fn td(dist: f64, vz: f64, vh: f64, inside: bool) -> TouchdownRecord {
        TouchdownRecord {
            time_s: 100.0,
            phase: FlightPhase::Flare,
            x_m: dist,
            y_m: 0.0,
            distance_to_target_m: dist,
            along_axis_m: dist,
            cross_axis_m: 0.0,
            horizontal_speed_m_s: vh,
            vertical_speed_m_s: vz,
            inside_landing_area: inside,
        }
    }

    fn approach_transition() -> PhaseTransition {
        PhaseTransition {
            time_s: 50.0,
            from: FlightPhase::Cruise,
            to: FlightPhase::Approach,
            reason: String::new(),
            state: StateSnapshot::default(),
        }
    }

    fn run(t: &[TouchdownRecord], final_phase: FlightPhase, reached: bool) -> LandingAssessment {
        let tr = [approach_transition()];
        assess(&AssessmentInput {
            landing: &LandingConfig::default(),
            area: &LandingArea::default(),
            target: Vec2::ZERO,
            touchdowns: t,
            transitions: &tr,
            events: &[],
            final_phase,
            reached_target_area: reached,
        })
    }

    #[test]
    fn nominal_landing_is_successful() {
        let a = run(&[td(1.0, -0.3, 0.2, true)], FlightPhase::Landed, true);
        assert_eq!(a.classification, LandingClass::Successful);
    }

    #[test]
    fn proximity_alone_is_not_success() {
        // Reached the target area but never touched down.
        let a = run(&[], FlightPhase::Flare, true);
        assert_eq!(a.classification, LandingClass::Unsuccessful);
        assert!(!a.ground_contact);
    }

    #[test]
    fn hard_or_fast_or_far_touchdowns_fail() {
        for t in [
            td(1.0, -1.5, 0.2, true),
            td(1.0, -0.3, 2.0, true),
            td(9.0, -0.3, 0.2, true),
            td(1.0, -0.3, 0.2, false),
        ] {
            let a = run(&[t], FlightPhase::Landed, true);
            assert_eq!(a.classification, LandingClass::Unsuccessful);
            assert!(!a.touchdown_within_limits);
        }
    }

    #[test]
    fn incomplete_sequence_fails() {
        let a = run(&[td(1.0, -0.3, 0.2, true)], FlightPhase::PostLanding, true);
        assert_eq!(a.classification, LandingClass::Unsuccessful);
        assert!(!a.sequence_completed);
    }
}
