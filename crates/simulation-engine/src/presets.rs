//! Predefined environment scenarios and research experiments A-G.
//!
//! All presets are deterministic: wind changes and gusts use explicit triggers and
//! stochastic turbulence uses a recorded seed.

use airship_model::{Vec2, Vec3};
use autopilot::{FlightPath, Waypoint};
use control_algorithms::ControllerKind;
use flight_dynamics::{
    DiscreteGust, DisturbanceTrigger, EnvironmentConfig, InitialStateConfig, TurbulenceConfig,
    WindChange, WindConfig,
};
use serde::{Deserialize, Serialize};

use crate::config::SimulationConfig;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scenario {
    pub id: String,
    pub name: String,
    pub description: String,
    pub environment: EnvironmentConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExperimentRun {
    pub label: String,
    pub config: SimulationConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Experiment {
    pub id: String,
    pub title: String,
    pub objective: String,
    pub description: String,
    pub focus_metrics: Vec<String>,
    pub runs: Vec<ExperimentRun>,
}

fn base_env() -> EnvironmentConfig {
    SimulationConfig::default().environment
}

fn calm() -> EnvironmentConfig {
    base_env()
}

fn constant_wind() -> EnvironmentConfig {
    let mut e = base_env();
    e.wind = WindConfig {
        speed_m_s: 4.0,
        from_direction_deg: 270.0,
        shear_exponent: 0.14,
        ..Default::default()
    };
    e
}

fn changing_wind() -> EnvironmentConfig {
    let mut e = base_env();
    e.wind = WindConfig {
        speed_m_s: 2.0,
        from_direction_deg: 270.0,
        changes: vec![WindChange {
            trigger: DisturbanceTrigger::DistanceToLanding { distance_m: 300.0 },
            ramp_duration_s: 30.0,
            speed_m_s: 5.5,
            from_direction_deg: 210.0,
            vertical_m_s: 0.0,
        }],
        ..Default::default()
    };
    e
}

fn gusts() -> EnvironmentConfig {
    let mut e = base_env();
    e.wind = WindConfig {
        speed_m_s: 3.0,
        from_direction_deg: 250.0,
        gusts: vec![
            DiscreteGust {
                trigger: DisturbanceTrigger::AtTime { time_s: 90.0 },
                duration_s: 12.0,
                speed_m_s: 5.0,
                from_direction_deg: 180.0,
                vertical_m_s: 0.5,
            },
            DiscreteGust {
                trigger: DisturbanceTrigger::DistanceToLanding { distance_m: 120.0 },
                duration_s: 10.0,
                speed_m_s: 3.0,
                from_direction_deg: 320.0,
                vertical_m_s: -0.3,
            },
        ],
        turbulence: TurbulenceConfig {
            enabled: true,
            intensity_m_s: 0.7,
            vertical_ratio: 0.5,
            correlation_time_s: 4.0,
            seed: 42,
        },
        ..Default::default()
    };
    e
}

fn limited_ground() -> EnvironmentConfig {
    let mut e = base_env();
    e.landing_area.undershoot_m = 10.0;
    e.landing_area.overrun_m = 6.0;
    e.landing_area.half_width_m = 6.0;
    e.wind = WindConfig {
        speed_m_s: 2.5,
        from_direction_deg: 90.0,
        ..Default::default()
    };
    e
}

pub fn scenarios() -> Vec<Scenario> {
    let s = |id: &str, name: &str, description: &str, environment: EnvironmentConfig| Scenario {
        id: id.into(),
        name: name.into(),
        description: description.into(),
        environment,
    };
    vec![
        s(
            "calm",
            "1. Calm air",
            "No wind, no turbulence, no sensor noise.",
            calm(),
        ),
        s(
            "constant_wind",
            "2. Constant horizontal wind",
            "4 m/s westerly wind at 10 m with power-law shear (alpha = 0.14); headwind on the final approach.",
            constant_wind(),
        ),
        s(
            "changing_wind",
            "3. Wind changing during approach",
            "2 m/s westerly; when the vehicle first comes within 300 m of the landing point the wind ramps over 30 s to 5.5 m/s from 210 deg (crosswind component).",
            changing_wind(),
        ),
        s(
            "gusts",
            "4. Moderate gust disturbance",
            "3 m/s from 250 deg, Gauss-Markov turbulence (sigma 0.7 m/s, T_c 4 s, seed 42), a 5 m/s 1-cosine gust at t = 90 s and a 3 m/s gust 120 m before the landing point.",
            gusts(),
        ),
        s(
            "limited_ground",
            "5. Limited available ground distance",
            "Landing area only 10 m before and 6 m beyond the landing point (+/-6 m wide) with a 2.5 m/s tailwind on the final approach.",
            limited_ground(),
        ),
    ]
}

fn with_env(name: &str, description: &str, env: EnvironmentConfig) -> SimulationConfig {
    SimulationConfig {
        name: name.into(),
        description: description.into(),
        environment: env,
        ..SimulationConfig::default()
    }
}

fn controller(mut c: SimulationConfig, kind: ControllerKind) -> SimulationConfig {
    c.controller.kind = kind;
    c
}

/// Airborne start on the extended final-approach course, 30 m right of the line.
fn approach_start(env: &mut EnvironmentConfig) {
    let l = env.landing_site;
    env.initial_state = InitialStateConfig {
        position: Vec2::new(l.x + 450.0, l.y + 30.0),
        altitude_agl_m: 25.0,
        velocity: Vec3::new(-3.5, 0.0, 0.0),
        heading_deg: 270.0,
        pitch_deg: 0.0,
        roll_deg: 0.0,
        moored: false,
    };
}

fn approach_config(name: &str, description: &str, mut env: EnvironmentConfig) -> SimulationConfig {
    approach_start(&mut env);
    let mut c = with_env(name, description, env);
    c.flight_path = FlightPath {
        name: "Approach only".into(),
        waypoints: vec![],
    };
    c.simulation.duration_s = 500.0;
    c
}

pub fn experiments() -> Vec<Experiment> {
    let one = |c: SimulationConfig| {
        vec![ExperimentRun {
            label: ControllerKind::CascadedPid.label().into(),
            config: c,
        }]
    };
    let m = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();

    // A: take-off in calm air.
    let a = with_env(
        "Experiment A - take-off in calm air",
        "Reference mission in calm air.",
        calm(),
    );

    // B: take-off under horizontal wind (crosswind relative to the first leg).
    let mut b_env = calm();
    b_env.wind = WindConfig {
        speed_m_s: 5.0,
        from_direction_deg: 0.0,
        shear_exponent: 0.14,
        ..Default::default()
    };
    let b = with_env(
        "Experiment B - take-off under horizontal wind",
        "Reference mission with a 5 m/s northerly wind (at 10 m, shear 0.14).",
        b_env,
    );

    // C: altitude tracking of a commanded profile.
    let mut c_env = calm();
    c_env.initial_state = InitialStateConfig {
        position: Vec2::ZERO,
        altitude_agl_m: 40.0,
        velocity: Vec3::new(5.0, 0.0, 0.0),
        heading_deg: 90.0,
        pitch_deg: 0.0,
        roll_deg: 0.0,
        moored: false,
    };
    c_env.landing_site = Vec2::new(900.0, -150.0);
    c_env.landing_area.heading_deg = 270.0;
    let wp = |x: f64, alt: f64| Waypoint {
        x_m: x,
        y_m: 0.0,
        altitude_m: alt,
        heading_deg: None,
        speed_m_s: 5.0,
        tolerance_m: 12.0,
    };
    let mut c = with_env(
        "Experiment C - altitude tracking",
        "Airborne start at 40 m; level legs with altitude steps 40-60-35-45 m at 5 m/s.",
        c_env,
    );
    c.flight_path = FlightPath {
        name: "Altitude profile".into(),
        waypoints: vec![
            wp(150.0, 40.0),
            wp(250.0, 60.0),
            wp(450.0, 60.0),
            wp(550.0, 35.0),
            wp(750.0, 35.0),
            wp(850.0, 45.0),
            wp(1000.0, 45.0),
        ],
    };

    // D: approach and landing in calm air.
    let d = approach_config(
        "Experiment D - approach and landing in calm air",
        "Airborne start 450 m out on the extended final course, 30 m off the centre line.",
        calm(),
    );

    // E: landing under changing wind.
    let mut e_env = calm();
    e_env.wind = WindConfig {
        speed_m_s: 1.5,
        from_direction_deg: 270.0,
        changes: vec![WindChange {
            trigger: DisturbanceTrigger::DistanceToLanding { distance_m: 180.0 },
            ramp_duration_s: 20.0,
            speed_m_s: 5.0,
            from_direction_deg: 200.0,
            vertical_m_s: 0.0,
        }],
        turbulence: TurbulenceConfig {
            enabled: true,
            intensity_m_s: 0.4,
            vertical_ratio: 0.5,
            correlation_time_s: 4.0,
            seed: 7,
        },
        ..Default::default()
    };
    let e = approach_config(
        "Experiment E - landing under changing wind",
        "Approach-only start; the wind ramps from 1.5 m/s (270 deg) to 5 m/s from 200 deg once the vehicle is within 180 m of the landing point; light turbulence (seed 7).",
        e_env,
    );

    // F: controller comparison in gusts.
    let f_base = with_env(
        "Experiment F - controller comparison",
        "Reference mission in scenario 4 (gusts, turbulence seed 42).",
        gusts(),
    );

    // G: go-around after an approach-constraint violation.
    let mut g_env = calm();
    g_env.wind = WindConfig {
        gusts: vec![DiscreteGust {
            trigger: DisturbanceTrigger::DistanceToLanding { distance_m: 110.0 },
            duration_s: 40.0,
            speed_m_s: 9.0,
            from_direction_deg: 0.0,
            vertical_m_s: 0.0,
        }],
        ..Default::default()
    };
    let mut g = approach_config(
        "Experiment G - go-around",
        "Approach-only start; a 9 m/s crosswind gust (40 s, 1-cosine) starts 110 m before the landing point. The landing wind limit is 7 m/s, so the approach becomes infeasible and a go-around is expected, followed by a second approach.",
        g_env,
    );
    g.autopilot.landing.max_wind_m_s = 7.0;
    g.simulation.duration_s = 900.0;

    vec![
        Experiment {
            id: "A".into(),
            title: "Take-off in calm air".into(),
            objective: "Evaluate the take-off sequence, altitude tracking, climb rate and time to reach the target altitude.".into(),
            description: a.description.clone(),
            focus_metrics: m(&["time_to_target_altitude_s", "takeoff_duration_s", "altitude_rmse_m", "vertical_velocity_rmse_m_s", "max_heave_effort"]),
            runs: one(a),
        },
        Experiment {
            id: "B".into(),
            title: "Take-off under horizontal wind".into(),
            objective: "Evaluate the ability to follow the desired climb trajectory with a constant wind disturbance.".into(),
            description: b.description.clone(),
            focus_metrics: m(&["time_to_target_altitude_s", "cross_track_rmse_m", "altitude_rmse_m", "max_roll_deg", "saturation_time_s"]),
            runs: one(b),
        },
        Experiment {
            id: "C".into(),
            title: "Altitude tracking".into(),
            objective: "Evaluate tracking accuracy for a commanded altitude profile.".into(),
            description: c.description.clone(),
            focus_metrics: m(&["altitude_rmse_m", "altitude_max_error_m", "vertical_velocity_rmse_m_s", "max_heave_effort", "energy_wh"]),
            runs: one(c),
        },
        Experiment {
            id: "D".into(),
            title: "Approach and landing in calm air".into(),
            objective: "Evaluate landing-point error, touchdown speeds and completion of the landing sequence.".into(),
            description: d.description.clone(),
            focus_metrics: m(&["landing_point_error_m", "touchdown_vertical_speed_m_s", "touchdown_horizontal_speed_m_s", "approach_duration_s", "cross_track_rmse_m"]),
            runs: one(d),
        },
        Experiment {
            id: "E".into(),
            title: "Landing under changing wind".into(),
            objective: "Evaluate how the autopilot handles a wind change during the approach.".into(),
            description: e.description.clone(),
            focus_metrics: m(&["landing_point_error_m", "touchdown_horizontal_speed_m_s", "cross_track_max_m", "go_around_count", "saturation_time_s"]),
            runs: one(e),
        },
        Experiment {
            id: "F".into(),
            title: "Controller comparison".into(),
            objective: "Compare the cascaded PID and the gain-scheduled controller with identical initial conditions, parameters and disturbance seeds.".into(),
            description: f_base.description.clone(),
            focus_metrics: m(&[
                "altitude_rmse_m",
                "cross_track_rmse_m",
                "landing_point_error_m",
                "touchdown_vertical_speed_m_s",
                "control_effort_index_s",
                "saturation_time_s",
                "energy_wh",
                "mission_duration_s",
            ]),
            runs: vec![
                ExperimentRun { label: ControllerKind::CascadedPid.label().into(), config: controller(f_base.clone(), ControllerKind::CascadedPid) },
                ExperimentRun { label: ControllerKind::GainScheduled.label().into(), config: controller(f_base, ControllerKind::GainScheduled) },
            ],
        },
        Experiment {
            id: "G".into(),
            title: "Go-around / aborted landing".into(),
            objective: "Introduce an approach-constraint violation and verify that the state machine detects it and performs the configured go-around.".into(),
            description: g.description.clone(),
            focus_metrics: m(&["go_around_count", "landing_attempts", "landing_point_error_m", "mission_duration_s", "warning_events"]),
            runs: one(g),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_presets_validate() {
        for s in scenarios() {
            let c = with_env(&s.name, "", s.environment);
            let r = c.validate();
            assert!(!r.has_errors(), "{}: {:?}", s.id, r.issues);
        }
        for e in experiments() {
            for run in &e.runs {
                let r = run.config.validate();
                assert!(
                    !r.has_errors(),
                    "{}: {:?}",
                    e.id,
                    r.errors().collect::<Vec<_>>()
                );
            }
        }
    }
}
