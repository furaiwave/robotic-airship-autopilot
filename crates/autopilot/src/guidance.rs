//! Guidance laws: line-of-sight waypoint following and the final-approach geometry.
//!
//! **Line-of-sight (LOS) path following.** For the active leg from `a` to `b` with unit
//! tangent `t`, the vehicle at `p` steers toward the look-ahead point
//! `a + t min(s + Delta, L)`, where `s = (p - a) . t` is the along-track position and
//! `Delta` the look-ahead distance. The cross-track error `e = t x (p - a)` converges
//! to zero; the ground-velocity feed-forward has the leg speed and points at the
//! look-ahead point. The altitude reference is interpolated linearly along the leg,
//! which gives a constant flight-path angle per leg.
//!
//! **Final approach.** A straight line along the approach course `u` ending at the
//! landing point `L`. The glide path is `h(d) = h_flare + d tan(gamma)` with
//! `d = (L - p) . u` the distance to go, capped at the approach altitude. The gate
//! lies at `d_gate = (h_app - h_flare) / tan(gamma)` and the pre-gate a further
//! `pregate_extension_m` out.

use airship_model::Vec2;
use airship_model::math::{heading_deg_to_yaw, wrap_pi};
use serde::{Deserialize, Serialize};

use crate::config::AutopilotConfig;
use crate::flight_path::Waypoint;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ApproachGeometry {
    pub landing: Vec2,
    /// Direction of travel on final (rad, CCW from East).
    pub course_yaw: f64,
    pub course_dir: Vec2,
    pub glide_tan: f64,
    pub approach_altitude: f64,
    pub flare_height: f64,
    pub gate_distance: f64,
    pub pregate_extension: f64,
}

impl ApproachGeometry {
    pub fn new(landing: Vec2, course_yaw: f64, ap: &AutopilotConfig) -> Self {
        let glide_tan = ap.approach.glide_angle_deg.to_radians().tan();
        let gate = (ap.approach.altitude_m - ap.landing.flare_height_m).max(0.0) / glide_tan;
        Self {
            landing,
            course_yaw,
            course_dir: Vec2::from_angle(course_yaw),
            glide_tan,
            approach_altitude: ap.approach.altitude_m,
            flare_height: ap.landing.flare_height_m,
            gate_distance: gate,
            pregate_extension: ap.approach.pregate_extension_m,
        }
    }

    /// Build from the autopilot configuration and the landing-area heading.
    pub fn from_config(landing: Vec2, landing_area_heading_deg: f64, ap: &AutopilotConfig) -> Self {
        let course = ap.approach.course_deg.unwrap_or(landing_area_heading_deg);
        Self::new(landing, heading_deg_to_yaw(course), ap)
    }

    pub fn gate(&self) -> Vec2 {
        self.landing - self.course_dir * self.gate_distance
    }

    pub fn pregate(&self) -> Vec2 {
        self.landing - self.course_dir * (self.gate_distance + self.pregate_extension)
    }

    /// Intercept point on the extended centre line, 40 m outside the pre-gate.
    pub fn join_point(&self) -> Vec2 {
        self.pregate() - self.course_dir * 40.0
    }

    /// Distance to go along the course (positive before the landing point).
    pub fn distance_to_go(&self, p: Vec2) -> f64 {
        (self.landing - p).dot(self.course_dir)
    }

    /// Signed cross-track error (positive left of the course).
    pub fn cross_track(&self, p: Vec2) -> f64 {
        self.course_dir.cross(p - self.landing)
    }

    /// Glide-path altitude (gear height above ground) at distance to go `d`.
    pub fn glide_altitude(&self, d: f64) -> f64 {
        (self.flare_height + d.max(0.0) * self.glide_tan).min(self.approach_altitude)
    }

    /// LOS ground-velocity command along the final-approach line.
    pub fn line_velocity(&self, p: Vec2, lookahead: f64, speed: f64) -> Vec2 {
        let d = self.distance_to_go(p);
        let target = self.landing - self.course_dir * (d - lookahead).max(0.0);
        let dir = (target - p).normalized();
        let dir = if dir.norm() < 0.5 {
            self.course_dir
        } else {
            dir
        };
        dir * speed
    }
}

/// Look-ahead distance that grows with the turn radius `V / r_max` so that the LOS
/// law does not demand turns tighter than the vehicle can fly.
pub fn adaptive_lookahead(base: f64, speed: f64, max_turn_rate: f64) -> f64 {
    if max_turn_rate > 0.0 {
        base.max(1.5 * speed / max_turn_rate)
    } else {
        base
    }
}

/// Output of the path follower for one control step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PathCommand {
    pub ground_velocity_ff: Vec2,
    pub hold_position: Option<Vec2>,
    pub max_ground_speed: f64,
    pub altitude: f64,
    pub vertical_speed_ff: f64,
    pub cross_track_error: f64,
    pub yaw_override: Option<f64>,
    pub waypoint_index: usize,
}

#[derive(Debug, Clone)]
pub struct PathFollower {
    waypoints: Vec<Waypoint>,
    index: usize,
    leg_start: Vec2,
    anchor_along: f64,
    anchor_altitude: f64,
    /// Waypoints captured during the last update (index, reason).
    pub captured: Vec<(usize, String)>,
}

impl PathFollower {
    pub fn new(waypoints: Vec<Waypoint>, start: Vec2, start_altitude: f64) -> Self {
        Self {
            waypoints,
            index: 0,
            leg_start: start,
            anchor_along: 0.0,
            anchor_altitude: start_altitude,
            captured: Vec::new(),
        }
    }

    pub fn is_complete(&self) -> bool {
        self.index >= self.waypoints.len()
    }

    pub fn active_index(&self) -> Option<usize> {
        (!self.is_complete()).then_some(self.index)
    }

    pub fn remaining(&self) -> usize {
        self.waypoints.len().saturating_sub(self.index)
    }

    /// Restart the altitude interpolation of the active leg from the current state.
    pub fn anchor_altitude(&mut self, p: Vec2, h: f64) {
        if let Some(w) = self.waypoints.get(self.index) {
            let seg = w.position() - self.leg_start;
            let len = seg.norm();
            self.anchor_along = if len > 1e-6 {
                (p - self.leg_start).dot(seg * (1.0 / len))
            } else {
                0.0
            };
            self.anchor_altitude = h;
        }
    }

    fn advance(&mut self, reason: String) {
        let w = &self.waypoints[self.index];
        self.captured.push((self.index, reason));
        self.leg_start = w.position();
        self.anchor_along = 0.0;
        self.anchor_altitude = w.altitude_m;
        self.index += 1;
    }

    /// Compute the guidance command. Returns `None` once every waypoint is captured.
    /// `max_turn_rate` (rad/s) sets the minimum look-ahead `1.5 V / r_max`.
    pub fn update(
        &mut self,
        p: Vec2,
        h: f64,
        yaw: f64,
        max_turn_rate: f64,
        cfg: &AutopilotConfig,
    ) -> Option<PathCommand> {
        self.captured.clear();
        let tol = &cfg.transitions;
        loop {
            let w = self.waypoints.get(self.index)?.clone();
            let seg = w.position() - self.leg_start;
            let len = seg.norm();
            let t = if len > 1e-6 {
                seg * (1.0 / len)
            } else {
                Vec2::from_angle(yaw)
            };
            let along = (p - self.leg_start).dot(t);
            let dist = (w.position() - p).norm();
            if w.is_hover() {
                let heading_ok = w
                    .heading_deg
                    .map(|hd| {
                        wrap_pi(heading_deg_to_yaw(hd) - yaw).abs()
                            <= tol.hover_heading_deg.to_radians()
                    })
                    .unwrap_or(true);
                if dist <= w.tolerance_m
                    && (h - w.altitude_m).abs() <= tol.hover_altitude_m
                    && heading_ok
                {
                    self.advance(format!(
                        "hover waypoint {} captured: {:.1} m from the point, altitude error {:.1} m",
                        self.index + 1,
                        dist,
                        h - w.altitude_m
                    ));
                    continue;
                }
            } else if dist <= w.tolerance_m || along >= len {
                let why = if dist <= w.tolerance_m {
                    "inside acceptance radius"
                } else {
                    "passed abeam"
                };
                self.advance(format!(
                    "waypoint {} reached ({why}, {:.1} m)",
                    self.index + 1,
                    dist
                ));
                continue;
            }

            let cross = t.cross(p - self.leg_start);
            let remaining = len - self.anchor_along;
            let frac = if remaining > 1.0 {
                ((along - self.anchor_along) / remaining).clamp(0.0, 1.0)
            } else {
                1.0
            };
            let altitude = self.anchor_altitude + (w.altitude_m - self.anchor_altitude) * frac;
            let gradient = if remaining > 1.0 && frac < 1.0 {
                (w.altitude_m - self.anchor_altitude) / remaining
            } else {
                0.0
            };

            let g = &cfg.guidance;
            if w.is_hover() && dist < (2.0 * w.tolerance_m).max(15.0) {
                let speed = g
                    .hold_max_speed_m_s
                    .min((2.0 * g.hover_deceleration_m_s2 * dist).sqrt())
                    .max(0.3);
                let yaw_override = if dist <= w.tolerance_m {
                    w.heading_deg.map(heading_deg_to_yaw)
                } else {
                    None
                };
                return Some(PathCommand {
                    ground_velocity_ff: Vec2::ZERO,
                    hold_position: Some(w.position()),
                    max_ground_speed: speed,
                    altitude: w.altitude_m,
                    vertical_speed_ff: 0.0,
                    cross_track_error: cross,
                    yaw_override,
                    waypoint_index: self.index,
                });
            }
            let speed = if w.is_hover() {
                cfg.cruise_speed_m_s
                    .min((2.0 * g.hover_deceleration_m_s2 * dist).sqrt())
                    .max(0.5)
            } else {
                w.speed_m_s
            };
            let lookahead = adaptive_lookahead(g.lookahead_m, speed, max_turn_rate);
            let target = if along + lookahead >= len {
                w.position()
            } else {
                self.leg_start + t * (along + lookahead).max(0.0)
            };
            let dir = (target - p).normalized();
            let dir = if dir.norm() < 0.5 { t } else { dir };
            return Some(PathCommand {
                ground_velocity_ff: dir * speed,
                hold_position: None,
                max_ground_speed: speed.max(0.5),
                altitude,
                vertical_speed_ff: gradient * speed,
                cross_track_error: cross,
                yaw_override: None,
                waypoint_index: self.index,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wp(x: f64, y: f64, alt: f64, speed: f64) -> Waypoint {
        Waypoint {
            x_m: x,
            y_m: y,
            altitude_m: alt,
            heading_deg: None,
            speed_m_s: speed,
            tolerance_m: 10.0,
        }
    }

    #[test]
    fn los_steers_back_to_the_track() {
        let cfg = AutopilotConfig::default();
        let mut f = PathFollower::new(vec![wp(200.0, 0.0, 40.0, 5.0)], Vec2::ZERO, 40.0);
        // Vehicle 20 m left (north) of the track heading East.
        let c = f
            .update(Vec2::new(50.0, 20.0), 40.0, 0.0, 1.0, &cfg)
            .unwrap();
        assert!(c.cross_track_error > 0.0);
        assert!(
            c.ground_velocity_ff.y < 0.0,
            "steers right toward the track"
        );
        assert!((c.ground_velocity_ff.norm() - 5.0).abs() < 1e-9);
    }

    #[test]
    fn altitude_is_interpolated_along_the_leg() {
        let cfg = AutopilotConfig::default();
        let mut f = PathFollower::new(vec![wp(100.0, 0.0, 60.0, 5.0)], Vec2::ZERO, 40.0);
        let c = f
            .update(Vec2::new(50.0, 0.0), 40.0, 0.0, 1.0, &cfg)
            .unwrap();
        assert!((c.altitude - 50.0).abs() < 1e-9);
        assert!(
            (c.vertical_speed_ff - 1.0).abs() < 1e-9,
            "20 m over 100 m at 5 m/s = 1 m/s"
        );
    }

    #[test]
    fn waypoints_are_captured_in_order() {
        let cfg = AutopilotConfig::default();
        let mut f = PathFollower::new(
            vec![wp(100.0, 0.0, 40.0, 5.0), wp(100.0, 100.0, 40.0, 5.0)],
            Vec2::ZERO,
            40.0,
        );
        f.update(Vec2::new(95.0, 0.0), 40.0, 0.0, 1.0, &cfg)
            .unwrap();
        assert_eq!(f.active_index(), Some(1));
        assert_eq!(f.captured.len(), 1);
        assert!(
            f.update(Vec2::new(100.0, 95.0), 40.0, 1.57, 1.0, &cfg)
                .is_none()
        );
        assert!(f.is_complete());
    }

    #[test]
    fn hover_waypoint_requires_altitude_and_heading() {
        let cfg = AutopilotConfig::default();
        let hover = Waypoint {
            heading_deg: Some(0.0),
            speed_m_s: 0.0,
            ..wp(0.0, 0.0, 50.0, 0.0)
        };
        let mut f = PathFollower::new(vec![hover], Vec2::ZERO, 20.0);
        let c = f.update(Vec2::ZERO, 30.0, 0.0, 1.0, &cfg).unwrap();
        assert_eq!(c.hold_position, Some(Vec2::ZERO));
        assert!(c.yaw_override.is_some());
        // Altitude reached but heading East instead of North: not captured.
        assert!(f.update(Vec2::ZERO, 50.0, 0.0, 1.0, &cfg).is_some());
        // Heading North (yaw = pi/2): captured.
        assert!(
            f.update(Vec2::ZERO, 50.0, std::f64::consts::FRAC_PI_2, 1.0, &cfg)
                .is_none()
        );
    }

    #[test]
    fn approach_geometry() {
        let cfg = AutopilotConfig::default();
        let g = ApproachGeometry::new(Vec2::new(100.0, 0.0), 0.0, &cfg);
        let tan = cfg.approach.glide_angle_deg.to_radians().tan();
        assert!(
            (g.gate_distance - (cfg.approach.altitude_m - cfg.landing.flare_height_m) / tan).abs()
                < 1e-9
        );
        assert!((g.distance_to_go(Vec2::new(40.0, 5.0)) - 60.0).abs() < 1e-9);
        assert!((g.cross_track(Vec2::new(40.0, 5.0)) - 5.0).abs() < 1e-9);
        assert!((g.glide_altitude(0.0) - cfg.landing.flare_height_m).abs() < 1e-9);
        assert_eq!(g.glide_altitude(1e4), cfg.approach.altitude_m);
        assert!((g.glide_altitude(g.gate_distance) - cfg.approach.altitude_m).abs() < 1e-9);
        let v = g.line_velocity(Vec2::new(0.0, 10.0), 30.0, 3.0);
        assert!(
            v.x > 0.0 && v.y < 0.0,
            "steers forward and back toward the line"
        );
    }
}
