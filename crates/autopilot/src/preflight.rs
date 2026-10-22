//! Preflight checks executed in the `Preflight` phase. Every check is evaluated on the
//! configuration and the current measurement; the phase is left only if all pass.

use airship_model::{AirshipParams, DerivedProperties, ValidationReport};
use flight_dynamics::Measurement;
use serde::{Deserialize, Serialize};

use crate::config::AutopilotConfig;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreflightCheck {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

fn check(name: &str, passed: bool, detail: String) -> PreflightCheck {
    PreflightCheck {
        name: name.into(),
        passed,
        detail,
    }
}

pub fn run_preflight(
    cfg: &AutopilotConfig,
    vehicle: &AirshipParams,
    derived: &DerivedProperties,
    path_report: &ValidationReport,
    m: &Measurement,
    wind_m_s: f64,
) -> Vec<PreflightCheck> {
    let mut out = Vec::new();
    let finite = m.position.is_finite()
        && m.velocity.is_finite()
        && m.yaw.is_finite()
        && m.altitude_agl_m.is_finite();
    out.push(check(
        "sensors",
        finite,
        if finite {
            "all measurements finite".into()
        } else {
            "non-finite measurement".into()
        },
    ));

    let physics = derived.validate_physics(vehicle);
    let errs: Vec<String> = physics.errors().map(|e| e.message.clone()).collect();
    out.push(check(
        "vehicle configuration",
        errs.is_empty(),
        if errs.is_empty() {
            format!(
                "buoyancy/weight {:.3}, static lift {:+.1} N",
                derived.buoyancy_ratio, derived.static_lift_n
            )
        } else {
            errs.join("; ")
        },
    ));

    let heaviness = -derived.static_lift_n;
    let up_needed = heaviness + cfg.takeoff.lift_margin_n;
    let up_avail = derived.max_heave_up_n * cfg.control_limits.heave;
    let ground_start = m.moored;
    let lift_ok = !ground_start || up_avail >= up_needed;
    out.push(check(
        "take-off lift margin",
        lift_ok,
        format!("heave thrust available {:.0} N, required {:.0} N (heaviness {:+.0} N + margin {:.0} N)", up_avail, up_needed, heaviness, cfg.takeoff.lift_margin_n),
    ));

    let down_needed = derived.static_lift_n;
    let down_avail = derived.max_heave_down_n * cfg.control_limits.heave;
    let land_ok = down_avail > down_needed;
    out.push(check(
        "landing descent authority",
        land_ok,
        format!(
            "downward heave thrust available {:.0} N, static lift to overcome {:.0} N",
            down_avail,
            down_needed.max(0.0)
        ),
    ));

    let wind_limit = if ground_start {
        cfg.takeoff.max_wind_m_s
    } else {
        cfg.landing.max_wind_m_s
    };
    out.push(check(
        "wind",
        wind_m_s <= wind_limit,
        format!("estimated wind {wind_m_s:.1} m/s, limit {wind_limit:.1} m/s"),
    ));

    let soc_ok = m.battery_soc >= cfg.takeoff.min_battery_soc;
    out.push(check(
        "battery",
        soc_ok,
        format!(
            "state of charge {:.0} %, minimum {:.0} %",
            m.battery_soc * 100.0,
            cfg.takeoff.min_battery_soc * 100.0
        ),
    ));

    let path_errs: Vec<String> = path_report
        .errors()
        .map(|e| format!("{}: {}", e.field, e.message))
        .collect();
    out.push(check(
        "flight path",
        path_errs.is_empty(),
        if path_errs.is_empty() {
            format!("{} warning(s)", path_report.warnings().count())
        } else {
            path_errs.join("; ")
        },
    ));
    out
}
