//! Closed-loop integration tests of the complete simulation (physics + sensors +
//! autopilot + controllers). These exercise the take-off and landing state machine
//! end to end and check reproducibility.

use autopilot::FlightPhase;
use control_algorithms::ControllerKind;
use simulation_engine::{
    FinishReason, LandingClass, Simulation, SimulationConfig, experiments, scenarios,
};

fn run(cfg: SimulationConfig) -> Simulation {
    let mut sim = Simulation::new(cfg).expect("valid configuration");
    sim.run_to_completion();
    sim
}

fn experiment(id: &str) -> Vec<SimulationConfig> {
    experiments()
        .into_iter()
        .find(|e| e.id == id)
        .expect("experiment exists")
        .runs
        .into_iter()
        .map(|r| r.config)
        .collect()
}

fn phases(sim: &Simulation) -> Vec<FlightPhase> {
    sim.autopilot().transitions.iter().map(|t| t.to).collect()
}

#[test]
fn reference_mission_lands_with_both_controllers() {
    for kind in [ControllerKind::CascadedPid, ControllerKind::GainScheduled] {
        let mut cfg = SimulationConfig::default();
        cfg.controller.kind = kind;
        let sim = run(cfg);
        let r = sim.report();
        assert_eq!(
            sim.finished(),
            Some(FinishReason::Landed),
            "{kind:?}: {}",
            r.finish_message
        );
        assert_eq!(
            r.landing.classification,
            LandingClass::Successful,
            "{kind:?}: {}",
            r.landing.explanation
        );
        // The nominal sequence visits every take-off and landing phase in order.
        let expected = [
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
        ];
        assert_eq!(phases(&sim), expected, "{kind:?}");
        assert!(r.metric("landing_point_error_m").unwrap() <= 5.0);
        assert!(r.metric("touchdown_vertical_speed_m_s").unwrap() <= 0.8);
        assert!(r.metric("time_to_target_altitude_s").is_some());
        assert_eq!(r.metric("fatal_events"), Some(0.0));
    }
}

#[test]
fn transitions_carry_reasons_and_timestamps() {
    let sim = run(SimulationConfig::default());
    let tr = &sim.autopilot().transitions;
    assert!(tr.windows(2).all(|w| w[0].time_s <= w[1].time_s));
    assert!(tr.iter().all(|t| !t.reason.is_empty()));
    // Take-off is triggered by the measured mooring load, not by a timer.
    let release = tr.iter().find(|t| t.to == FlightPhase::Takeoff).unwrap();
    assert!(
        release.reason.contains("mooring load"),
        "{}",
        release.reason
    );
}

#[test]
fn identical_configuration_gives_bit_identical_results() {
    // Turbulence and sensor noise enabled: determinism must hold for seeded randomness.
    let mut cfg = SimulationConfig::default();
    cfg.environment = scenarios()
        .into_iter()
        .find(|s| s.id == "gusts")
        .unwrap()
        .environment;
    cfg.environment.sensor_noise.enabled = true;
    cfg.simulation.duration_s = 120.0;
    let a = run(cfg.clone());
    // Advance the second run in irregular chunks: chunking must not change the physics.
    let mut b = Simulation::new(cfg.clone()).unwrap();
    let mut k = 1;
    while b.finished().is_none() {
        b.advance(k);
        k = k % 97 + 13;
    }
    assert_eq!(a.telemetry(), b.telemetry());
    // A different turbulence seed changes the trajectory.
    cfg.environment.wind.turbulence.seed += 1;
    let c = run(cfg);
    assert_ne!(
        a.telemetry().last().unwrap().x_m,
        c.telemetry().last().unwrap().x_m
    );
}

#[test]
fn integration_error_decreases_with_time_step() {
    let at = |dt: f64| {
        let mut cfg = experiment("D").remove(0);
        cfg.simulation.dt_s = dt;
        let mut sim = Simulation::new(cfg).unwrap();
        sim.advance_time(60.0);
        sim.state().position
    };
    let reference = at(0.001);
    let err = |dt: f64| (at(dt) - reference).norm();
    let (e1, e2, e3) = (err(0.025), err(0.01), err(0.005));
    assert!(e1 > e2 && e2 > e3, "errors {e1:e} {e2:e} {e3:e}");
    assert!(e2 < 0.01, "dt = 0.01 s error {e2} m");
}

#[test]
fn go_around_is_executed_when_approach_becomes_infeasible() {
    let sim = run(experiment("G").remove(0));
    let r = sim.report();
    assert!(r.metric("go_around_count").unwrap() >= 1.0);
    let ga = sim
        .autopilot()
        .transitions
        .iter()
        .find(|t| t.to == FlightPhase::GoAround)
        .unwrap();
    assert_eq!(ga.from, FlightPhase::Descent);
    assert!(sim.autopilot().landing_attempts() >= 2);
    assert_eq!(
        sim.finished(),
        Some(FinishReason::Landed),
        "{}",
        r.finish_message
    );
}

#[test]
fn all_experiments_complete_without_fatal_events() {
    for e in experiments() {
        for run_cfg in e.runs {
            let sim = run(run_cfg.config);
            let r = sim.report();
            assert_ne!(
                sim.finished(),
                Some(FinishReason::Fatal),
                "{}: {}",
                e.id,
                r.finish_message
            );
            assert!(sim.finished().is_some(), "{} did not finish", e.id);
        }
    }
}

#[test]
fn insufficient_heave_authority_fails_preflight() {
    let mut cfg = SimulationConfig::default();
    // Only 20 % of the heave thruster may be used: below heaviness + lift margin.
    cfg.autopilot.control_limits.heave = 0.2;
    let sim = run(cfg);
    assert_eq!(sim.finished(), Some(FinishReason::Aborted));
    let ap = sim.autopilot();
    assert!(
        ap.preflight
            .iter()
            .any(|c| c.name == "take-off lift margin" && !c.passed)
    );
    assert_eq!(ap.transitions.last().unwrap().to, FlightPhase::Aborted);
    assert_eq!(
        sim.report().landing.classification,
        LandingClass::NotAttempted
    );
}

#[test]
fn trim_error_is_compensated_by_the_thrust_ramp() {
    // The onboard mass estimate is 4 kg too low: the thrust ramp continues until the
    // measured mooring load confirms positive lift.
    let mut cfg = SimulationConfig::default();
    cfg.autopilot.trim_mass_bias_kg = -4.0;
    let sim = run(cfg);
    assert_eq!(
        sim.finished(),
        Some(FinishReason::Landed),
        "{}",
        sim.report().finish_message
    );
}

#[test]
fn statically_light_vehicle_takes_off_and_is_held_down_after_landing() {
    let mut cfg = SimulationConfig::default();
    let rho = cfg.environment.ground_density();
    cfg.airship.total_mass_kg = rho * cfg.airship.displaced_volume() * 0.97; // 3 % light
    let sim = run(cfg);
    let r = sim.report();
    assert_eq!(
        sim.finished(),
        Some(FinishReason::Landed),
        "{}",
        r.finish_message
    );
    assert!(r.derived.static_lift_n > 0.0);
    // While on the ground before mooring the heave thruster pushes down.
    let ground = sim
        .telemetry()
        .iter()
        .rev()
        .find(|s| s.phase == FlightPhase::PostLanding)
        .unwrap();
    assert!(ground.thrust_heave_n < 0.0);
}

#[test]
fn hard_impact_is_detected_as_fatal() {
    let mut cfg = experiment("D").remove(0);
    cfg.environment.initial_state.altitude_agl_m = 2.0;
    cfg.environment.initial_state.velocity.z = -6.0;
    let sim = run(cfg);
    assert_eq!(sim.finished(), Some(FinishReason::Fatal));
    let r = sim.report();
    assert!(
        r.safety_events
            .iter()
            .any(|e| e.constraint == autopilot::SafetyConstraint::GroundCollision)
    );
    assert_eq!(r.landing.classification, LandingClass::Unsuccessful);
}

#[test]
fn limited_ground_scenario_is_evaluated_against_the_landing_area() {
    let mut cfg = SimulationConfig::default();
    cfg.environment = scenarios()
        .into_iter()
        .find(|s| s.id == "limited_ground")
        .unwrap()
        .environment;
    let sim = run(cfg);
    let r = sim.report();
    let td = r.landing.touchdown.as_ref().expect("a touchdown occurred");
    // Classification must agree with the containment check of the touchdown point.
    if !td.inside_landing_area {
        assert_eq!(r.landing.classification, LandingClass::Unsuccessful);
    }
    assert!(
        r.landing
            .criteria
            .iter()
            .any(|c| c.name == "Touchdown within limits")
    );
}

#[test]
fn invalid_configurations_are_rejected() {
    let mut cfg = SimulationConfig::default();
    cfg.simulation.dt_s = 0.0;
    assert!(Simulation::new(cfg).is_err());
    let mut cfg = SimulationConfig::default();
    cfg.airship.total_mass_kg = 150.0; // far heavier than the buoyancy: not a meaningful LTA vehicle
    assert!(Simulation::new(cfg).is_err());
}

#[test]
fn csv_export_matches_telemetry() {
    let mut cfg = SimulationConfig::default();
    cfg.simulation.duration_s = 30.0;
    let sim = run(cfg);
    let csv = simulation_engine::telemetry_csv(sim.telemetry());
    assert_eq!(csv.lines().count(), sim.telemetry().len() + 1);
    assert!(csv.starts_with("t_s,phase,"));
}
