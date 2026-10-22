//! Running several configurations and comparing them under identical conditions.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::SimulationConfig;
use crate::engine::{Simulation, SimulationError};
use crate::metrics::ResultsReport;
use crate::telemetry::{TelemetrySample, decimate};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComparisonRun {
    pub label: String,
    pub results: ResultsReport,
    /// Decimated telemetry for plotting.
    pub telemetry: Vec<TelemetrySample>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComparisonReport {
    pub runs: Vec<ComparisonRun>,
    /// True when the configurations differ only in the controller section and names.
    pub identical_conditions: bool,
    /// Dotted paths of configuration fields that differ between runs (controller excluded).
    pub differences: Vec<String>,
    pub disclaimer: String,
}

fn strip(cfg: &SimulationConfig) -> Value {
    let mut v = serde_json::to_value(cfg).unwrap_or(Value::Null);
    if let Value::Object(m) = &mut v {
        m.remove("controller");
        m.remove("name");
        m.remove("description");
    }
    v
}

fn diff(a: &Value, b: &Value, path: &str, out: &mut Vec<String>) {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            let mut keys: Vec<&String> = x.keys().chain(y.keys()).collect();
            keys.sort();
            keys.dedup();
            for k in keys {
                let p = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{path}.{k}")
                };
                match (x.get(k), y.get(k)) {
                    (Some(xa), Some(yb)) => diff(xa, yb, &p, out),
                    _ => out.push(p),
                }
            }
        }
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => {
            for (i, (xa, yb)) in x.iter().zip(y).enumerate() {
                diff(xa, yb, &format!("{path}[{i}]"), out);
            }
        }
        _ if a != b => out.push(path.to_string()),
        _ => {}
    }
}

/// Configuration fields (outside the controller section) that differ between configs.
pub fn config_differences(configs: &[&SimulationConfig]) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(first) = configs.first() {
        let base = strip(first);
        for c in &configs[1..] {
            diff(&base, &strip(c), "", &mut out);
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Run every configuration to completion (in parallel threads) and compare them.
pub fn compare_runs(
    runs: Vec<(String, SimulationConfig)>,
    max_points: usize,
) -> Result<ComparisonReport, SimulationError> {
    let configs: Vec<&SimulationConfig> = runs.iter().map(|r| &r.1).collect();
    let differences = config_differences(&configs);
    let mut sims = Vec::with_capacity(runs.len());
    for (_, cfg) in &runs {
        sims.push(Simulation::new(cfg.clone())?);
    }
    std::thread::scope(|scope| {
        for sim in sims.iter_mut() {
            scope.spawn(move || sim.run_to_completion());
        }
    });
    let out = runs
        .iter()
        .zip(sims.iter())
        .map(|((label, _), sim)| ComparisonRun {
            label: label.clone(),
            results: sim.report(),
            telemetry: decimate(sim.telemetry(), max_points),
        })
        .collect();
    Ok(ComparisonReport {
        runs: out,
        identical_conditions: differences.is_empty(),
        differences,
        disclaimer: crate::DISCLAIMER.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use control_algorithms::ControllerKind;

    #[test]
    fn controller_changes_are_not_differences() {
        let a = SimulationConfig::default();
        let mut b = a.clone();
        b.controller.kind = ControllerKind::GainScheduled;
        b.name = "other".into();
        assert!(config_differences(&[&a, &b]).is_empty());
        b.environment.wind.speed_m_s = 3.0;
        assert_eq!(
            config_differences(&[&a, &b]),
            vec!["environment.wind.speed_m_s".to_string()]
        );
    }
}
