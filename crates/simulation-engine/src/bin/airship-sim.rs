//! Headless command-line runner.
//!
//! ```text
//! airship-sim run <config.json> [--csv out.csv] [--json out.json] [--controller cascaded_pid|gain_scheduled]
//! airship-sim experiment <A..G> [--out-dir dir]
//! airship-sim export-presets <dir>
//! airship-sim default-config
//! ```

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use control_algorithms::ControllerKind;
use simulation_engine::{ResultsReport, Simulation, SimulationConfig, experiments, telemetry_csv};

fn usage() -> ExitCode {
    eprintln!(
        "usage:\n  airship-sim run <config.json> [--csv out.csv] [--json out.json] [--controller cascaded_pid|gain_scheduled]\n  airship-sim experiment <A..G> [--out-dir dir]\n  airship-sim export-presets <dir>\n  airship-sim default-config"
    );
    ExitCode::from(2)
}

fn arg_value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn print_summary(label: &str, r: &ResultsReport) {
    println!("== {label} [{}] ==", r.controller_label);
    println!("finish: {:?} - {}", r.finish_reason, r.finish_message);
    println!(
        "landing: {:?} - {}",
        r.landing.classification, r.landing.explanation
    );
    for t in &r.transitions {
        println!(
            "  t={:7.2}s  {:?} -> {:?}: {}",
            t.time_s, t.from, t.to, t.reason
        );
    }
    for e in &r.safety_events {
        println!(
            "  [{:?}] t={:.1}s {:?}: {}",
            e.severity, e.time_s, e.constraint, e.message
        );
    }
    for d in &r.disturbances {
        println!("  disturbance t={:.1}s: {}", d.time_s, d.description);
    }
    for m in &r.metrics {
        if let Some(v) = m.value {
            println!("  {:<34} {:>12.3} {}", m.key, v, m.unit);
        }
    }
}

fn run_config(
    cfg: SimulationConfig,
    label: &str,
    csv: Option<PathBuf>,
    json: Option<PathBuf>,
) -> Result<ResultsReport, String> {
    let mut sim = Simulation::new(cfg).map_err(|e| e.to_string())?;
    sim.run_to_completion();
    let report = sim.report();
    print_summary(label, &report);
    if let Some(p) = csv {
        std::fs::write(&p, telemetry_csv(sim.telemetry())).map_err(|e| e.to_string())?;
        println!("telemetry written to {}", p.display());
    }
    if let Some(p) = json {
        let doc = serde_json::json!({ "config": sim.config(), "results": report, "telemetry": sim.telemetry() });
        std::fs::write(
            &p,
            serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        println!("experiment written to {}", p.display());
    }
    Ok(report)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = args.first() else {
        return usage();
    };
    let result: Result<(), String> = match cmd.as_str() {
        "default-config" => {
            println!(
                "{}",
                serde_json::to_string_pretty(&SimulationConfig::default()).unwrap_or_default()
            );
            Ok(())
        }
        "run" => {
            let Some(path) = args.get(1) else {
                return usage();
            };
            (|| {
                let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
                let mut cfg: SimulationConfig =
                    serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))?;
                if let Some(k) = arg_value(&args, "--controller") {
                    cfg.controller.kind = serde_json::from_value(serde_json::Value::String(k))
                        .map_err(|e| e.to_string())?;
                }
                let label = cfg.name.clone();
                run_config(
                    cfg,
                    &label,
                    arg_value(&args, "--csv").map(PathBuf::from),
                    arg_value(&args, "--json").map(PathBuf::from),
                )
                .map(|_| ())
            })()
        }
        "experiment" => {
            let Some(id) = args.get(1) else {
                return usage();
            };
            let out_dir = arg_value(&args, "--out-dir").map(PathBuf::from);
            (|| {
                let exp = experiments()
                    .into_iter()
                    .find(|e| e.id.eq_ignore_ascii_case(id))
                    .ok_or(format!("unknown experiment {id}"))?;
                println!(
                    "Experiment {}: {}\nObjective: {}\n",
                    exp.id, exp.title, exp.objective
                );
                for run in exp.runs {
                    let slug = match run.config.controller.kind {
                        ControllerKind::CascadedPid => "pid",
                        ControllerKind::GainScheduled => "scheduled",
                    };
                    let (csv, json) = match &out_dir {
                        Some(d) => {
                            std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
                            (
                                Some(d.join(format!("experiment-{}-{slug}.csv", exp.id))),
                                Some(d.join(format!("experiment-{}-{slug}.json", exp.id))),
                            )
                        }
                        None => (None, None),
                    };
                    run_config(run.config, &run.label, csv, json)?;
                    println!();
                }
                Ok(())
            })()
        }
        "export-presets" => {
            let Some(dir) = args.get(1) else {
                return usage();
            };
            (|| {
                let dir = Path::new(dir);
                std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
                for exp in experiments() {
                    for run in &exp.runs {
                        let slug = match run.config.controller.kind {
                            ControllerKind::CascadedPid => "pid",
                            ControllerKind::GainScheduled => "scheduled",
                        };
                        let p =
                            dir.join(format!("experiment-{}-{slug}.json", exp.id.to_lowercase()));
                        std::fs::write(
                            &p,
                            serde_json::to_string_pretty(&run.config).map_err(|e| e.to_string())?,
                        )
                        .map_err(|e| e.to_string())?;
                        println!("wrote {}", p.display());
                    }
                }
                Ok(())
            })()
        }
        _ => return usage(),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
