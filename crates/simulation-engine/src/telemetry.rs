//! Telemetry samples and CSV export.

use autopilot::FlightPhase;
use serde::{Deserialize, Serialize};

/// One telemetry record. Truth state (`x_m` ... `yaw_rate_deg_s`), references (`ref_*`),
/// tracking errors (`err_*`), commanded virtual forces (`cmd_*`), realised actuator
/// outputs, normalised control usage (`u_*`), saturation flags (`sat_*`) and forces.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TelemetrySample {
    pub t_s: f64,
    pub phase: FlightPhase,
    pub x_m: f64,
    pub y_m: f64,
    pub altitude_msl_m: f64,
    pub altitude_agl_m: f64,
    pub vx_m_s: f64,
    pub vy_m_s: f64,
    pub vz_m_s: f64,
    pub ground_speed_m_s: f64,
    pub airspeed_m_s: f64,
    pub airspeed_forward_m_s: f64,
    pub heading_deg: f64,
    pub pitch_deg: f64,
    pub roll_deg: f64,
    pub yaw_rate_deg_s: f64,
    pub wind_x_m_s: f64,
    pub wind_y_m_s: f64,
    pub wind_z_m_s: f64,
    pub wind_speed_m_s: f64,
    pub wind_from_deg: f64,
    pub ref_altitude_agl_m: Option<f64>,
    pub ref_vz_m_s: f64,
    pub ref_heading_deg: f64,
    pub ref_yaw_rate_deg_s: f64,
    pub ref_airspeed_m_s: f64,
    pub ref_vx_m_s: f64,
    pub ref_vy_m_s: f64,
    pub err_altitude_m: Option<f64>,
    pub err_cross_track_m: Option<f64>,
    pub err_heading_deg: f64,
    pub err_vz_m_s: f64,
    pub err_horizontal_velocity_m_s: f64,
    pub distance_to_landing_m: f64,
    pub distance_to_go_m: f64,
    pub active_waypoint: Option<usize>,
    pub landing_attempt: u32,
    pub cmd_surge_n: f64,
    pub cmd_heave_n: f64,
    pub cmd_yaw_moment_nm: f64,
    pub thrust_left_n: f64,
    pub thrust_right_n: f64,
    pub thrust_heave_n: f64,
    pub rudder_deg: f64,
    pub u_surge: f64,
    pub u_heave: f64,
    pub u_yaw: f64,
    pub sat_surge: bool,
    pub sat_heave: bool,
    pub sat_yaw: bool,
    pub buoyancy_n: f64,
    pub weight_n: f64,
    pub aero_x_n: f64,
    pub aero_y_n: f64,
    pub aero_z_n: f64,
    pub thrust_z_n: f64,
    pub ground_force_n: f64,
    pub mooring_load_n: f64,
    pub contact: bool,
    pub moored: bool,
    pub power_w: f64,
    pub energy_wh: f64,
    pub battery_soc: f64,
    pub air_density_kg_m3: f64,
    pub gain_vz_kp: f64,
    pub gain_yaw_rate_kp: f64,
    pub gain_airspeed_kp: f64,
    pub gain_position: f64,
}

const COLUMNS: &[&str] = &[
    "t_s",
    "phase",
    "x_m",
    "y_m",
    "altitude_msl_m",
    "altitude_agl_m",
    "vx_m_s",
    "vy_m_s",
    "vz_m_s",
    "ground_speed_m_s",
    "airspeed_m_s",
    "airspeed_forward_m_s",
    "heading_deg",
    "pitch_deg",
    "roll_deg",
    "yaw_rate_deg_s",
    "wind_x_m_s",
    "wind_y_m_s",
    "wind_z_m_s",
    "wind_speed_m_s",
    "wind_from_deg",
    "ref_altitude_agl_m",
    "ref_vz_m_s",
    "ref_heading_deg",
    "ref_yaw_rate_deg_s",
    "ref_airspeed_m_s",
    "ref_vx_m_s",
    "ref_vy_m_s",
    "err_altitude_m",
    "err_cross_track_m",
    "err_heading_deg",
    "err_vz_m_s",
    "err_horizontal_velocity_m_s",
    "distance_to_landing_m",
    "distance_to_go_m",
    "active_waypoint",
    "landing_attempt",
    "cmd_surge_n",
    "cmd_heave_n",
    "cmd_yaw_moment_nm",
    "thrust_left_n",
    "thrust_right_n",
    "thrust_heave_n",
    "rudder_deg",
    "u_surge",
    "u_heave",
    "u_yaw",
    "sat_surge",
    "sat_heave",
    "sat_yaw",
    "buoyancy_n",
    "weight_n",
    "aero_x_n",
    "aero_y_n",
    "aero_z_n",
    "thrust_z_n",
    "ground_force_n",
    "mooring_load_n",
    "contact",
    "moored",
    "power_w",
    "energy_wh",
    "battery_soc",
    "air_density_kg_m3",
    "gain_vz_kp",
    "gain_yaw_rate_kp",
    "gain_airspeed_kp",
    "gain_position",
];

fn f(v: f64) -> String {
    format!("{v:.6}")
}

fn opt(v: Option<f64>) -> String {
    v.map(f).unwrap_or_default()
}

fn phase_name(p: FlightPhase) -> String {
    serde_json::to_value(p)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// Render samples as CSV with a header row (units are part of the column names).
pub fn telemetry_csv(samples: &[TelemetrySample]) -> String {
    let mut out = String::with_capacity(samples.len() * 600 + 1024);
    out.push_str(&COLUMNS.join(","));
    out.push('\n');
    for s in samples {
        let row: Vec<String> = vec![
            f(s.t_s),
            phase_name(s.phase),
            f(s.x_m),
            f(s.y_m),
            f(s.altitude_msl_m),
            f(s.altitude_agl_m),
            f(s.vx_m_s),
            f(s.vy_m_s),
            f(s.vz_m_s),
            f(s.ground_speed_m_s),
            f(s.airspeed_m_s),
            f(s.airspeed_forward_m_s),
            f(s.heading_deg),
            f(s.pitch_deg),
            f(s.roll_deg),
            f(s.yaw_rate_deg_s),
            f(s.wind_x_m_s),
            f(s.wind_y_m_s),
            f(s.wind_z_m_s),
            f(s.wind_speed_m_s),
            f(s.wind_from_deg),
            opt(s.ref_altitude_agl_m),
            f(s.ref_vz_m_s),
            f(s.ref_heading_deg),
            f(s.ref_yaw_rate_deg_s),
            f(s.ref_airspeed_m_s),
            f(s.ref_vx_m_s),
            f(s.ref_vy_m_s),
            opt(s.err_altitude_m),
            opt(s.err_cross_track_m),
            f(s.err_heading_deg),
            f(s.err_vz_m_s),
            f(s.err_horizontal_velocity_m_s),
            f(s.distance_to_landing_m),
            f(s.distance_to_go_m),
            s.active_waypoint.map(|w| w.to_string()).unwrap_or_default(),
            s.landing_attempt.to_string(),
            f(s.cmd_surge_n),
            f(s.cmd_heave_n),
            f(s.cmd_yaw_moment_nm),
            f(s.thrust_left_n),
            f(s.thrust_right_n),
            f(s.thrust_heave_n),
            f(s.rudder_deg),
            f(s.u_surge),
            f(s.u_heave),
            f(s.u_yaw),
            u8::from(s.sat_surge).to_string(),
            u8::from(s.sat_heave).to_string(),
            u8::from(s.sat_yaw).to_string(),
            f(s.buoyancy_n),
            f(s.weight_n),
            f(s.aero_x_n),
            f(s.aero_y_n),
            f(s.aero_z_n),
            f(s.thrust_z_n),
            f(s.ground_force_n),
            f(s.mooring_load_n),
            u8::from(s.contact).to_string(),
            u8::from(s.moored).to_string(),
            f(s.power_w),
            f(s.energy_wh),
            f(s.battery_soc),
            f(s.air_density_kg_m3),
            f(s.gain_vz_kp),
            f(s.gain_yaw_rate_kp),
            f(s.gain_airspeed_kp),
            f(s.gain_position),
        ];
        debug_assert_eq!(row.len(), COLUMNS.len());
        out.push_str(&row.join(","));
        out.push('\n');
    }
    out
}

/// Keep at most `max` samples by uniform decimation (first and last sample kept).
pub fn decimate(samples: &[TelemetrySample], max: usize) -> Vec<TelemetrySample> {
    if samples.len() <= max || max < 2 {
        return samples.to_vec();
    }
    let stride = (samples.len() as f64 / max as f64).ceil() as usize;
    let mut out: Vec<TelemetrySample> = samples.iter().step_by(stride).cloned().collect();
    if let Some(last) = samples.last()
        && out.last().map(|s| s.t_s) != Some(last.t_s)
    {
        out.push(last.clone());
    }
    out
}

#[cfg(test)]
pub(crate) fn sample_at(t: f64) -> TelemetrySample {
    let s = r#"{"t_s":0,"phase":"cruise","x_m":0,"y_m":0,"altitude_msl_m":0,"altitude_agl_m":0,"vx_m_s":0,"vy_m_s":0,"vz_m_s":0,
    "ground_speed_m_s":0,"airspeed_m_s":0,"airspeed_forward_m_s":0,"heading_deg":0,"pitch_deg":0,"roll_deg":0,"yaw_rate_deg_s":0,
    "wind_x_m_s":0,"wind_y_m_s":0,"wind_z_m_s":0,"wind_speed_m_s":0,"wind_from_deg":0,"ref_altitude_agl_m":null,"ref_vz_m_s":0,
    "ref_heading_deg":0,"ref_yaw_rate_deg_s":0,"ref_airspeed_m_s":0,"ref_vx_m_s":0,"ref_vy_m_s":0,"err_altitude_m":null,
    "err_cross_track_m":null,"err_heading_deg":0,"err_vz_m_s":0,"err_horizontal_velocity_m_s":0,"distance_to_landing_m":0,
    "distance_to_go_m":0,"active_waypoint":null,"landing_attempt":0,"cmd_surge_n":0,"cmd_heave_n":0,"cmd_yaw_moment_nm":0,
    "thrust_left_n":0,"thrust_right_n":0,"thrust_heave_n":0,"rudder_deg":0,"u_surge":0,"u_heave":0,"u_yaw":0,"sat_surge":false,
    "sat_heave":false,"sat_yaw":false,"buoyancy_n":0,"weight_n":0,"aero_x_n":0,"aero_y_n":0,"aero_z_n":0,"thrust_z_n":0,
    "ground_force_n":0,"mooring_load_n":0,"contact":false,"moored":false,"power_w":0,"energy_wh":0,"battery_soc":1,
    "air_density_kg_m3":1.2,"gain_vz_kp":0,"gain_yaw_rate_kp":0,"gain_airspeed_kp":0,"gain_position":0}"#;
    let mut v: TelemetrySample = serde_json::from_str(s).unwrap();
    v.t_s = t;
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_has_header_and_matching_column_count() {
        let samples = vec![
            sample_at(0.0),
            TelemetrySample {
                err_altitude_m: Some(1.5),
                ..sample_at(0.1)
            },
        ];
        let csv = telemetry_csv(&samples);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 3);
        let n = lines[0].split(',').count();
        assert_eq!(n, COLUMNS.len());
        assert!(lines.iter().all(|l| l.split(',').count() == n));
        assert!(lines[1].contains(",cruise,"));
        assert!(lines[2].contains("1.500000"));
    }

    #[test]
    fn decimation_keeps_endpoints() {
        let samples: Vec<_> = (0..1001).map(|i| sample_at(i as f64 * 0.1)).collect();
        let d = decimate(&samples, 100);
        assert!(d.len() <= 102);
        assert_eq!(d.first().unwrap().t_s, 0.0);
        assert_eq!(d.last().unwrap().t_s, samples.last().unwrap().t_s);
    }
}
