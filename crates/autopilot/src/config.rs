//! Autopilot configuration: targets, transition thresholds, abort conditions and limits.

use airship_model::{AirshipParams, ValidationReport};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TakeoffConfig {
    /// Lift margin added to the estimated static heaviness before release (N).
    pub lift_margin_n: f64,
    /// Rate at which the heave thrust is increased while moored (N/s).
    pub thrust_ramp_n_s: f64,
    /// Upward mooring load required to release the mooring (N).
    pub release_load_threshold_n: f64,
    /// Maximum time in take-off preparation before aborting (s).
    pub preparation_timeout_s: f64,
    /// Gear height above ground that ends the take-off phase (m).
    pub clearance_altitude_m: f64,
    /// Gear height above ground that ends the initial climb (m).
    pub initial_climb_altitude_m: f64,
    /// Maximum time to reach the clearance altitude after release (s).
    pub takeoff_timeout_s: f64,
    /// Maximum wind for take-off (m/s).
    pub max_wind_m_s: f64,
    /// Minimum battery state of charge for take-off (-).
    pub min_battery_soc: f64,
}

impl Default for TakeoffConfig {
    fn default() -> Self {
        Self {
            lift_margin_n: 12.0,
            thrust_ramp_n_s: 15.0,
            release_load_threshold_n: 6.0,
            preparation_timeout_s: 30.0,
            clearance_altitude_m: 4.0,
            initial_climb_altitude_m: 15.0,
            takeoff_timeout_s: 60.0,
            max_wind_m_s: 8.0,
            min_battery_soc: 0.3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApproachConfig {
    /// Final approach course (compass deg); `None` uses the landing-area heading.
    pub course_deg: Option<f64>,
    /// Glide-path angle (deg).
    pub glide_angle_deg: f64,
    /// Altitude of the approach gate and of the go-around (m AGL).
    pub altitude_m: f64,
    /// Ground speed on the approach (m/s).
    pub speed_m_s: f64,
    /// Length of the straight segment before the gate (m).
    pub pregate_extension_m: f64,
    /// Half-width of the approach corridor for establishing on final (m).
    pub corridor_half_width_m: f64,
    /// Altitude tolerance for establishing on final (m).
    pub corridor_altitude_tolerance_m: f64,
    /// Maximum course error for establishing on final (deg).
    pub max_course_error_deg: f64,
    /// Distance over which the speed is reduced linearly before the landing point (m).
    pub deceleration_distance_m: f64,
}

impl Default for ApproachConfig {
    fn default() -> Self {
        Self {
            course_deg: None,
            glide_angle_deg: 8.0,
            altitude_m: 25.0,
            speed_m_s: 3.5,
            pregate_extension_m: 80.0,
            corridor_half_width_m: 15.0,
            corridor_altitude_tolerance_m: 6.0,
            max_course_error_deg: 35.0,
            deceleration_distance_m: 60.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LandingConfig {
    /// Gear height at which the flare (final vertical descent) begins (m).
    pub flare_height_m: f64,
    /// Maximum horizontal distance to the landing point for flare entry (m).
    pub flare_entry_radius_m: f64,
    /// Maximum ground speed for flare entry (m/s).
    pub flare_entry_max_speed_m_s: f64,
    /// Target sink rate at touchdown (m/s, positive down).
    pub touchdown_sink_rate_m_s: f64,
    /// Landing-point tolerance: maximum touchdown distance from the target (m).
    pub landing_point_tolerance_m: f64,
    /// Maximum vertical speed at touchdown (m/s, magnitude).
    pub max_touchdown_vertical_speed_m_s: f64,
    /// Maximum horizontal ground speed at touchdown (m/s).
    pub max_touchdown_horizontal_speed_m_s: f64,
    /// Below this gear height the landing is committed (no go-around) (m).
    pub decision_height_m: f64,
    /// Downward force applied after touchdown to keep the vehicle on the ground (N).
    pub hold_down_force_n: f64,
    /// Ground speed below which the vehicle is considered settled (m/s).
    pub settle_speed_m_s: f64,
    /// Time the settled conditions must hold before mooring (s).
    pub settle_time_s: f64,
    /// Maximum number of approaches (first attempt included).
    pub max_attempts: u32,
    /// Maximum wind for landing; above it the approach is infeasible (m/s).
    pub max_wind_m_s: f64,
    /// Cross-track error that triggers a go-around during descent (m).
    pub max_cross_track_error_m: f64,
    /// Glide-path altitude error that triggers a go-around during descent (m).
    pub max_altitude_error_m: f64,
}

impl Default for LandingConfig {
    fn default() -> Self {
        Self {
            flare_height_m: 3.0,
            flare_entry_radius_m: 6.0,
            flare_entry_max_speed_m_s: 1.2,
            touchdown_sink_rate_m_s: 0.3,
            landing_point_tolerance_m: 5.0,
            max_touchdown_vertical_speed_m_s: 0.8,
            max_touchdown_horizontal_speed_m_s: 1.0,
            decision_height_m: 1.5,
            hold_down_force_n: 30.0,
            settle_speed_m_s: 0.2,
            settle_time_s: 3.0,
            max_attempts: 3,
            max_wind_m_s: 8.0,
            max_cross_track_error_m: 12.0,
            max_altitude_error_m: 8.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransitionTolerances {
    /// Altitude tolerance for "target altitude reached" (m).
    pub altitude_m: f64,
    /// Vertical-speed tolerance for "altitude captured" (m/s).
    pub vertical_speed_m_s: f64,
    /// Altitude tolerance for capturing a hover (zero-speed) waypoint (m).
    pub hover_altitude_m: f64,
    /// Heading tolerance for hover waypoints with a target heading (deg).
    pub hover_heading_deg: f64,
}

impl Default for TransitionTolerances {
    fn default() -> Self {
        Self {
            altitude_m: 2.0,
            vertical_speed_m_s: 0.4,
            hover_altitude_m: 2.0,
            hover_heading_deg: 10.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GuidanceConfig {
    /// Line-of-sight look-ahead distance (m).
    pub lookahead_m: f64,
    /// Maximum ground speed while holding a position (m/s).
    pub hold_max_speed_m_s: f64,
    /// Deceleration used when approaching hover waypoints (m/s^2).
    pub hover_deceleration_m_s2: f64,
    /// Wind-estimate filter time constant (s).
    pub wind_filter_tau_s: f64,
}

impl Default for GuidanceConfig {
    fn default() -> Self {
        Self {
            lookahead_m: 30.0,
            hold_max_speed_m_s: 1.5,
            hover_deceleration_m_s2: 0.08,
            wind_filter_tau_s: 3.0,
        }
    }
}

/// Fractions of the available actuator capacity the controllers may command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlLimitFractions {
    pub surge: f64,
    pub heave: f64,
    pub yaw: f64,
}

impl Default for ControlLimitFractions {
    fn default() -> Self {
        Self {
            surge: 1.0,
            heave: 1.0,
            yaw: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SafetyConfig {
    /// Warning above `limit`, critical above `limit * critical_factor` (-).
    pub critical_factor: f64,
    /// Minimum gear height above ground in en-route phases (m).
    pub min_enroute_altitude_m: f64,
    /// Continuous actuator saturation time that raises a warning (s).
    pub saturation_warning_time_s: f64,
    pub battery_warning_soc: f64,
    pub battery_critical_soc: f64,
    /// Go around on critical events during approach/descent/flare (above decision height).
    pub go_around_on_critical: bool,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            critical_factor: 1.3,
            min_enroute_altitude_m: 10.0,
            saturation_warning_time_s: 8.0,
            battery_warning_soc: 0.25,
            battery_critical_soc: 0.15,
            go_around_on_critical: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutopilotConfig {
    /// Climb target altitude (gear height above ground, m).
    pub target_altitude_m: f64,
    /// Default horizontal ground speed for waypoints (m/s).
    pub cruise_speed_m_s: f64,
    /// Vertical speed during take-off and initial climb (m/s).
    pub takeoff_climb_rate_m_s: f64,
    /// Heading held during vertical take-off when the wind is light (compass deg);
    /// `None` keeps the initial heading. With wind the nose is pointed into the wind.
    pub takeoff_heading_deg: Option<f64>,
    /// Mass error assumed by the onboard trim estimate (kg); 0 = perfect knowledge.
    pub trim_mass_bias_kg: f64,
    pub takeoff: TakeoffConfig,
    pub approach: ApproachConfig,
    pub landing: LandingConfig,
    pub transitions: TransitionTolerances,
    pub guidance: GuidanceConfig,
    pub control_limits: ControlLimitFractions,
    pub safety: SafetyConfig,
}

impl Default for AutopilotConfig {
    fn default() -> Self {
        Self {
            target_altitude_m: 40.0,
            cruise_speed_m_s: 5.0,
            takeoff_climb_rate_m_s: 0.8,
            takeoff_heading_deg: None,
            trim_mass_bias_kg: 0.0,
            takeoff: TakeoffConfig::default(),
            approach: ApproachConfig::default(),
            landing: LandingConfig::default(),
            transitions: TransitionTolerances::default(),
            guidance: GuidanceConfig::default(),
            control_limits: ControlLimitFractions::default(),
            safety: SafetyConfig::default(),
        }
    }
}

impl AutopilotConfig {
    pub fn validate(&self, vehicle: &AirshipParams) -> ValidationReport {
        let mut r = ValidationReport::new();
        let lim = &vehicle.limits;
        r.range(
            "target_altitude_m",
            self.target_altitude_m,
            5.0,
            2000.0,
            "m",
        );
        r.range(
            "cruise_speed_m_s",
            self.cruise_speed_m_s,
            0.5,
            lim.max_airspeed_m_s,
            "m/s",
        );
        r.range(
            "takeoff_climb_rate_m_s",
            self.takeoff_climb_rate_m_s,
            0.1,
            lim.max_climb_rate_m_s,
            "m/s",
        );
        if let Some(h) = self.takeoff_heading_deg {
            r.range("takeoff_heading_deg", h, 0.0, 360.0, "deg");
        }
        r.range(
            "trim_mass_bias_kg",
            self.trim_mass_bias_kg,
            -0.5 * vehicle.total_mass_kg,
            0.5 * vehicle.total_mass_kg,
            "kg",
        );

        let t = &self.takeoff;
        r.range("takeoff.lift_margin_n", t.lift_margin_n, 0.0, 1.0e5, "N");
        r.range(
            "takeoff.thrust_ramp_n_s",
            t.thrust_ramp_n_s,
            0.5,
            1.0e5,
            "N/s",
        );
        r.range(
            "takeoff.release_load_threshold_n",
            t.release_load_threshold_n,
            0.0,
            1.0e5,
            "N",
        );
        r.range(
            "takeoff.preparation_timeout_s",
            t.preparation_timeout_s,
            1.0,
            600.0,
            "s",
        );
        r.range(
            "takeoff.clearance_altitude_m",
            t.clearance_altitude_m,
            0.5,
            100.0,
            "m",
        );
        if r.range(
            "takeoff.initial_climb_altitude_m",
            t.initial_climb_altitude_m,
            1.0,
            500.0,
            "m",
        ) && t.initial_climb_altitude_m <= t.clearance_altitude_m
        {
            r.error(
                "takeoff.initial_climb_altitude_m",
                "must be above the clearance altitude",
            );
        }
        if t.initial_climb_altitude_m >= self.target_altitude_m {
            r.error(
                "takeoff.initial_climb_altitude_m",
                "must be below the target altitude",
            );
        }
        r.range(
            "takeoff.takeoff_timeout_s",
            t.takeoff_timeout_s,
            5.0,
            600.0,
            "s",
        );
        r.range("takeoff.max_wind_m_s", t.max_wind_m_s, 0.0, 30.0, "m/s");
        r.range("takeoff.min_battery_soc", t.min_battery_soc, 0.0, 1.0, "-");

        let a = &self.approach;
        if let Some(c) = a.course_deg {
            r.range("approach.course_deg", c, 0.0, 360.0, "deg");
        }
        r.range(
            "approach.glide_angle_deg",
            a.glide_angle_deg,
            2.0,
            30.0,
            "deg",
        );
        if r.range("approach.altitude_m", a.altitude_m, 5.0, 500.0, "m")
            && a.altitude_m <= self.landing.flare_height_m + 2.0
        {
            r.error(
                "approach.altitude_m",
                "approach altitude must be at least 2 m above the flare height",
            );
        }
        r.range(
            "approach.speed_m_s",
            a.speed_m_s,
            0.5,
            lim.max_airspeed_m_s,
            "m/s",
        );
        r.range(
            "approach.pregate_extension_m",
            a.pregate_extension_m,
            10.0,
            2000.0,
            "m",
        );
        r.range(
            "approach.corridor_half_width_m",
            a.corridor_half_width_m,
            1.0,
            500.0,
            "m",
        );
        r.range(
            "approach.corridor_altitude_tolerance_m",
            a.corridor_altitude_tolerance_m,
            0.5,
            100.0,
            "m",
        );
        r.range(
            "approach.max_course_error_deg",
            a.max_course_error_deg,
            5.0,
            90.0,
            "deg",
        );
        r.range(
            "approach.deceleration_distance_m",
            a.deceleration_distance_m,
            5.0,
            1000.0,
            "m",
        );

        let l = &self.landing;
        r.range("landing.flare_height_m", l.flare_height_m, 0.5, 50.0, "m");
        r.range(
            "landing.flare_entry_radius_m",
            l.flare_entry_radius_m,
            0.5,
            200.0,
            "m",
        );
        r.range(
            "landing.flare_entry_max_speed_m_s",
            l.flare_entry_max_speed_m_s,
            0.1,
            10.0,
            "m/s",
        );
        r.range(
            "landing.touchdown_sink_rate_m_s",
            l.touchdown_sink_rate_m_s,
            0.05,
            lim.max_descent_rate_m_s,
            "m/s",
        );
        r.range(
            "landing.landing_point_tolerance_m",
            l.landing_point_tolerance_m,
            0.5,
            500.0,
            "m",
        );
        r.range(
            "landing.max_touchdown_vertical_speed_m_s",
            l.max_touchdown_vertical_speed_m_s,
            0.05,
            10.0,
            "m/s",
        );
        r.range(
            "landing.max_touchdown_horizontal_speed_m_s",
            l.max_touchdown_horizontal_speed_m_s,
            0.05,
            10.0,
            "m/s",
        );
        if l.touchdown_sink_rate_m_s >= l.max_touchdown_vertical_speed_m_s {
            r.error(
                "landing.touchdown_sink_rate_m_s",
                "target sink rate must be below the touchdown vertical-speed limit",
            );
        }
        if r.range(
            "landing.decision_height_m",
            l.decision_height_m,
            0.0,
            50.0,
            "m",
        ) && l.decision_height_m >= l.flare_height_m
        {
            r.error(
                "landing.decision_height_m",
                "decision height must be below the flare height",
            );
        }
        r.range(
            "landing.hold_down_force_n",
            l.hold_down_force_n,
            0.0,
            1.0e5,
            "N",
        );
        r.range(
            "landing.settle_speed_m_s",
            l.settle_speed_m_s,
            0.01,
            5.0,
            "m/s",
        );
        r.range("landing.settle_time_s", l.settle_time_s, 0.0, 120.0, "s");
        if l.max_attempts == 0 || l.max_attempts > 10 {
            r.error("landing.max_attempts", "must be between 1 and 10");
        }
        r.range("landing.max_wind_m_s", l.max_wind_m_s, 0.0, 30.0, "m/s");
        r.range(
            "landing.max_cross_track_error_m",
            l.max_cross_track_error_m,
            0.5,
            500.0,
            "m",
        );
        r.range(
            "landing.max_altitude_error_m",
            l.max_altitude_error_m,
            0.5,
            500.0,
            "m",
        );

        let tr = &self.transitions;
        r.range("transitions.altitude_m", tr.altitude_m, 0.1, 50.0, "m");
        r.range(
            "transitions.vertical_speed_m_s",
            tr.vertical_speed_m_s,
            0.01,
            5.0,
            "m/s",
        );
        r.range(
            "transitions.hover_altitude_m",
            tr.hover_altitude_m,
            0.1,
            50.0,
            "m",
        );
        r.range(
            "transitions.hover_heading_deg",
            tr.hover_heading_deg,
            1.0,
            90.0,
            "deg",
        );

        let g = &self.guidance;
        r.range("guidance.lookahead_m", g.lookahead_m, 2.0, 1000.0, "m");
        r.range(
            "guidance.hold_max_speed_m_s",
            g.hold_max_speed_m_s,
            0.1,
            lim.max_airspeed_m_s,
            "m/s",
        );
        r.range(
            "guidance.hover_deceleration_m_s2",
            g.hover_deceleration_m_s2,
            0.01,
            2.0,
            "m/s^2",
        );
        r.range(
            "guidance.wind_filter_tau_s",
            g.wind_filter_tau_s,
            0.0,
            60.0,
            "s",
        );

        let c = &self.control_limits;
        r.range("control_limits.surge", c.surge, 0.05, 1.0, "-");
        r.range("control_limits.heave", c.heave, 0.05, 1.0, "-");
        r.range("control_limits.yaw", c.yaw, 0.05, 1.0, "-");

        let s = &self.safety;
        r.range("safety.critical_factor", s.critical_factor, 1.0, 3.0, "-");
        r.range(
            "safety.min_enroute_altitude_m",
            s.min_enroute_altitude_m,
            0.0,
            500.0,
            "m",
        );
        r.range(
            "safety.saturation_warning_time_s",
            s.saturation_warning_time_s,
            0.1,
            600.0,
            "s",
        );
        r.range(
            "safety.battery_warning_soc",
            s.battery_warning_soc,
            0.0,
            1.0,
            "-",
        );
        r.range(
            "safety.battery_critical_soc",
            s.battery_critical_soc,
            0.0,
            1.0,
            "-",
        );
        if s.battery_critical_soc > s.battery_warning_soc {
            r.error(
                "safety.battery_critical_soc",
                "critical level must not exceed the warning level",
            );
        }
        if self.target_altitude_m < s.min_enroute_altitude_m {
            r.error(
                "target_altitude_m",
                "target altitude is below the minimum en-route altitude",
            );
        }
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_valid() {
        let r = AutopilotConfig::default().validate(&AirshipParams::default());
        assert!(!r.has_errors(), "{:?}", r.issues);
    }

    #[test]
    fn inconsistent_thresholds_are_rejected() {
        let mut c = AutopilotConfig::default();
        c.landing.decision_height_m = 5.0;
        assert!(c.validate(&AirshipParams::default()).has_errors());
        let mut c = AutopilotConfig::default();
        c.takeoff.initial_climb_altitude_m = 2.0;
        assert!(c.validate(&AirshipParams::default()).has_errors());
        let mut c = AutopilotConfig::default();
        c.cruise_speed_m_s = 50.0;
        assert!(
            c.validate(&AirshipParams::default()).has_errors(),
            "faster than the vehicle limit"
        );
    }
}
