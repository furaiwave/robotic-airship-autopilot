//! Time-step sensitivity study: runs Experiment D (approach and landing, calm air) with
//! several physics time steps and reports the deviation from a fine reference run.
//!
//! `cargo run --release -p simulation-engine --example dt_study`

use simulation_engine::{Simulation, experiments};

fn state_at(dt: f64, t_end: f64) -> (f64, f64, f64) {
    let mut cfg = experiments()
        .into_iter()
        .find(|e| e.id == "D")
        .expect("experiment D")
        .runs[0]
        .config
        .clone();
    cfg.simulation.dt_s = dt;
    let mut sim = Simulation::new(cfg).expect("valid config");
    sim.advance_time(t_end);
    let s = sim.state();
    (s.position.x, s.position.y, s.position.z)
}

fn main() {
    let t_end = 100.0;
    let reference = state_at(0.001, t_end);
    println!("Experiment D, state at t = {t_end} s compared with dt = 0.001 s");
    println!("{:>8} {:>14}", "dt [s]", "|dp| [m]");
    for dt in [0.025, 0.01, 0.005, 0.0025] {
        let p = state_at(dt, t_end);
        let e = ((p.0 - reference.0).powi(2)
            + (p.1 - reference.1).powi(2)
            + (p.2 - reference.2).powi(2))
        .sqrt();
        println!("{dt:>8} {e:>14.3e}");
    }
}
