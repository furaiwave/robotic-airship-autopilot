//! Cascaded loop structure shared by both flight controllers.
//!
//! ```text
//! Vertical:   h_ref --[altitude PI]--> vz_ref (+ vz_ff, limited) --[vz PI]--> F_z (+ trim + drag ff)
//! Horizontal: p_ref --[position P]--> v_ground,ref (+ guidance ff, limited)
//!             wind triangle: v_air,ref = v_ground,ref - w_est
//!                -> yaw_ref = atan2(v_air,ref) (when |v_air,ref| is large enough)
//!                -> u_ref   = |v_air,ref| cos(yaw_ref - yaw)
//! Heading:    yaw_ref --[heading P]--> r_ref (limited) --[yaw-rate PI]--> N
//! Speed:      u_ref --[airspeed PI]--> F_x (+ axial drag ff)
//! ```
//!
//! *Altitude control* and *vertical-speed control* are distinct loops: the altitude
//! loop is an outer loop whose output is the vertical-speed set-point of the inner
//! loop; when no altitude reference is active (take-off, flare) the inner loop tracks
//! a vertical-speed reference directly. The airship has no lateral actuator, so
//! lateral position errors are corrected by changing the heading (wind triangle).

use airship_model::math::wrap_pi;
use airship_model::{AirshipParams, DerivedProperties, Vec2};
use serde::{Deserialize, Serialize};

use crate::allocation::{AllocationResult, ControlAllocator, VirtualCommand};
use crate::pid::{Pid, PidGains, PidLimits};

/// Coarse flight-phase class visible to the controllers (used by gain scheduling).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhaseClass {
    Ground,
    Takeoff,
    Climb,
    Cruise,
    Approach,
    Landing,
}

/// Measured quantities used by the control loops.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct ControlMeasurement {
    pub position: Vec2,
    pub altitude_agl: f64,
    pub vertical_speed: f64,
    pub ground_velocity: Vec2,
    /// Body-x component of the air-relative velocity (m/s).
    pub airspeed_forward: f64,
    pub airspeed: f64,
    pub yaw: f64,
    pub yaw_rate: f64,
    /// Filtered horizontal wind estimate (m/s).
    pub wind: Vec2,
    pub air_density: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VerticalReference {
    /// Altitude reference above ground (m); `None` selects direct vertical-speed tracking.
    pub altitude_agl: Option<f64>,
    /// Vertical-speed feed-forward, or the direct reference when `altitude_agl` is `None` (m/s).
    pub vertical_speed_ff: f64,
    pub vertical_speed_min: f64,
    pub vertical_speed_max: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HorizontalReference {
    /// Guidance ground-velocity feed-forward (m/s).
    pub ground_velocity_ff: Vec2,
    /// Position to hold (position loop active when `Some`).
    pub hold_position: Option<Vec2>,
    pub max_ground_speed: f64,
    /// Heading to adopt when no significant air-relative motion is demanded (rad).
    pub preferred_yaw: Option<f64>,
    /// Forces a yaw reference regardless of the wind triangle (rad).
    pub yaw_override: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ControlReferences {
    pub vertical: VerticalReference,
    pub horizontal: HorizontalReference,
    /// Static heaviness feed-forward for the heave channel (N).
    pub heave_trim_n: f64,
    /// Open-loop heave force used while moored or on the ground (N). When it is
    /// released the vertical-speed integrator is preset for a bumpless transfer.
    pub heave_override_n: Option<f64>,
    pub max_airspeed: f64,
    pub max_yaw_rate: f64,
    /// When false (on the ground) surge and yaw commands are zero and their loops reset.
    pub horizontal_active: bool,
    /// When false the surge command is zero (yaw-only control, e.g. aligning at the mast).
    pub surge_enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ControlInput {
    pub phase: PhaseClass,
    pub meas: ControlMeasurement,
    pub refs: ControlReferences,
    pub distance_to_landing_m: f64,
}

/// Gains of every loop for one control step.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LoopGains {
    /// Altitude to vertical-speed (1/s, 1/s^2, -).
    pub altitude: PidGains,
    /// Vertical-speed error to heave force (N s/m, N/m, N s^2/m).
    pub vertical_speed: PidGains,
    /// Heading error to yaw-rate (1/s, ...).
    pub heading: PidGains,
    /// Yaw-rate error to yaw moment (N m s, N m, ...).
    pub yaw_rate: PidGains,
    /// Airspeed error to surge force (N s/m, N/m, ...).
    pub airspeed: PidGains,
    /// Position error to ground-velocity (1/s).
    pub position: f64,
}

/// Saturation limits of the virtual commands.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ForceLimits {
    pub surge_min_n: f64,
    pub surge_max_n: f64,
    pub heave_min_n: f64,
    pub heave_max_n: f64,
    pub yaw_max_nm: f64,
}

/// Nominal model constants known to the onboard controller.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ControlModel {
    /// `Cdx Ax` (m^2).
    pub axial_drag_area: f64,
    /// `Cdz Az` (m^2).
    pub vertical_drag_area: f64,
    pub mass_axial_kg: f64,
    pub mass_vertical_kg: f64,
    pub inertia_yaw_kg_m2: f64,
    /// Fin yaw damping per unit airspeed `1/2 rho S CLa l^2` at reference density (N m s / rad / (m/s)).
    pub fin_yaw_damping: f64,
    /// Quadratic hull yaw damping `c_h` (N m s^2).
    pub hull_yaw_damping: f64,
    pub reference_density: f64,
}

impl ControlModel {
    pub fn from_vehicle(p: &AirshipParams, d: &DerivedProperties) -> Self {
        Self {
            axial_drag_area: p.aero.cd_axial * d.frontal_area_m2,
            vertical_drag_area: p.aero.cd_vertical * d.planform_area_m2,
            mass_axial_kg: d.effective_mass_axial_kg,
            mass_vertical_kg: d.effective_mass_vertical_kg,
            inertia_yaw_kg_m2: d.effective_inertia_pitch_yaw_kg_m2,
            fin_yaw_damping: d.fin_yaw_damping_coeff,
            hull_yaw_damping: d.hull_yaw_damping_coeff,
            reference_density: d.reference_density_kg_m3,
        }
    }
}

/// Internal references produced by the cascade (logged as telemetry).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct InternalReferences {
    pub vertical_speed: f64,
    pub yaw: f64,
    pub yaw_rate: f64,
    pub airspeed: f64,
    pub ground_velocity: Vec2,
}

/// A channel is saturated when its loop output is clamped (magnitude or rate limit)
/// or when the allocation cannot realise the requested virtual command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Saturation {
    pub surge: bool,
    pub heave: bool,
    pub yaw: bool,
}

impl Saturation {
    pub fn any(&self) -> bool {
        self.surge || self.heave || self.yaw
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ControlOutput {
    pub virtual_command: VirtualCommand,
    pub allocation: AllocationResult,
    pub references: InternalReferences,
    pub gains: LoopGains,
    pub saturation: Saturation,
}

/// Rate limits and filter constants shared by both controllers.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CascadeSettings {
    pub derivative_filter_tau_s: f64,
    pub heave_rate_limit_n_s: f64,
    pub surge_rate_limit_n_s: f64,
    pub yaw_moment_rate_limit_nm_s: f64,
    pub drag_feedforward: bool,
    /// Largest reverse airspeed reference used for station keeping (m/s).
    pub max_reverse_airspeed_m_s: f64,
}

#[derive(Debug, Clone)]
pub struct CascadeCore {
    pid_altitude: Pid,
    pid_vz: Pid,
    pid_heading: Pid,
    pid_yaw_rate: Pid,
    pid_airspeed: Pid,
    tracking_air_velocity: bool,
    held_yaw: Option<f64>,
    repointing: bool,
    last_heave_override: Option<f64>,
}

impl Default for CascadeCore {
    fn default() -> Self {
        Self::new()
    }
}

impl CascadeCore {
    pub fn new() -> Self {
        Self {
            pid_altitude: Pid::new(),
            pid_vz: Pid::new(),
            pid_heading: Pid::new(),
            pid_yaw_rate: Pid::new(),
            pid_airspeed: Pid::new(),
            tracking_air_velocity: false,
            held_yaw: None,
            repointing: false,
            last_heave_override: None,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    #[allow(clippy::too_many_arguments)]
    pub fn compute(
        &mut self,
        input: &ControlInput,
        gains: &LoopGains,
        limits: &ForceLimits,
        settings: &CascadeSettings,
        model: &ControlModel,
        allocator: &ControlAllocator,
        dt: f64,
    ) -> ControlOutput {
        let m = &input.meas;
        let r = &input.refs;
        let rho = if m.air_density > 0.0 {
            m.air_density
        } else {
            model.reference_density
        };
        let tau = settings.derivative_filter_tau_s;

        // ---------------- Vertical channel ----------------
        let vr = &r.vertical;
        let vz_ref = match vr.altitude_agl {
            Some(h_ref) => {
                let lim = PidLimits {
                    derivative_tau_s: tau,
                    ..PidLimits::range(vr.vertical_speed_min, vr.vertical_speed_max)
                };
                // Integrator range is a fraction of the vertical-speed range.
                let lim = PidLimits {
                    integrator_min: 0.3 * lim.output_min,
                    integrator_max: 0.3 * lim.output_max,
                    ..lim
                };
                self.pid_altitude.update(
                    &gains.altitude,
                    &lim,
                    h_ref - m.altitude_agl,
                    m.altitude_agl,
                    Some(m.vertical_speed),
                    vr.vertical_speed_ff,
                    dt,
                )
            }
            None => {
                self.pid_altitude.reset();
                vr.vertical_speed_ff
                    .clamp(vr.vertical_speed_min, vr.vertical_speed_max)
            }
        };
        let vz_drag_ff = if settings.drag_feedforward {
            0.5 * rho * model.vertical_drag_area * vz_ref * vz_ref.abs()
        } else {
            0.0
        };
        let heave_lim = PidLimits {
            derivative_tau_s: tau,
            rate_limit: settings.heave_rate_limit_n_s,
            ..PidLimits::range(limits.heave_min_n, limits.heave_max_n)
        };
        let heave = if let Some(f) = r.heave_override_n {
            self.pid_vz.reset();
            self.last_heave_override = Some(f);
            f.clamp(limits.heave_min_n, limits.heave_max_n)
        } else {
            if let Some(f) = self.last_heave_override.take() {
                // Bumpless transfer from open-loop to closed-loop heave control.
                let preset =
                    (f - r.heave_trim_n - vz_drag_ff).clamp(limits.heave_min_n, limits.heave_max_n);
                self.pid_vz.preset_integral(preset);
            }
            self.pid_vz.update(
                &gains.vertical_speed,
                &heave_lim,
                vz_ref - m.vertical_speed,
                m.vertical_speed,
                None,
                r.heave_trim_n + vz_drag_ff,
                dt,
            )
        };

        // ---------------- Horizontal channels ----------------
        let hr = &r.horizontal;
        let (surge, yaw_moment, refs_h) = if r.horizontal_active {
            let hold = hr
                .hold_position
                .map(|p| (p - m.position) * gains.position)
                .unwrap_or(Vec2::ZERO);
            let v_ground_ref = (hr.ground_velocity_ff + hold).clamp_norm(hr.max_ground_speed);
            let v_air_ref = v_ground_ref - m.wind;
            let speed_air = v_air_ref.norm();
            if self.tracking_air_velocity {
                if speed_air < 0.25 {
                    self.tracking_air_velocity = false;
                    self.held_yaw = Some(m.yaw);
                }
            } else if speed_air > 0.5 {
                self.tracking_air_velocity = true;
            }
            // Forward-speed factor from the heading error. Path tracking uses cos(e);
            // station keeping uses "turn, then go" (zero beyond 45 deg) because pure
            // pursuit of a nearby point with a turn-rate limit otherwise ends in an orbit.
            let station_keeping = hr.hold_position.is_some();
            let align = |y: f64| {
                let c = wrap_pi(y - m.yaw).cos();
                if station_keeping {
                    ((c - std::f64::consts::FRAC_1_SQRT_2)
                        / (1.0 - std::f64::consts::FRAC_1_SQRT_2))
                        .clamp(0.0, 1.0)
                } else {
                    c.max(0.0)
                }
            };
            let along_heading = v_air_ref.dot(Vec2::from_angle(m.yaw));
            let (yaw_ref, airspeed_ref) = if let Some(y) = hr.yaw_override {
                (y, along_heading)
            } else if let (true, None, Some(target)) =
                (station_keeping, hr.preferred_yaw, hr.hold_position)
            {
                // Calm-air station keeping. The only horizontal force acts along the hull
                // axis: along-axis errors are removed with (reversible) thrust while the
                // heading is held; the nose is re-pointed at the hold position only when
                // the lateral offset exceeds 1.5 m (released below 0.5 m). Re-pointing uses
                // "turn, then go" so that no orbit develops around the point.
                let e = target - m.position;
                let lateral = Vec2::from_angle(m.yaw).cross(e).abs();
                if self.repointing {
                    if lateral < 0.5 || e.norm() < 0.5 {
                        self.repointing = false;
                        self.held_yaw = Some(m.yaw);
                    }
                } else if lateral > 1.5 {
                    self.repointing = true;
                }
                let y = if self.repointing {
                    // Face the point or turn the tail to it, whichever needs less rotation.
                    let a = e.angle();
                    let y = if wrap_pi(a - m.yaw).abs() > std::f64::consts::FRAC_PI_2 {
                        wrap_pi(a + std::f64::consts::PI)
                    } else {
                        a
                    };
                    self.held_yaw = Some(y);
                    y
                } else {
                    *self.held_yaw.get_or_insert(m.yaw)
                };
                (y, along_heading * align(y))
            } else if self.tracking_air_velocity {
                let y = v_air_ref.angle();
                (y, speed_air * align(y))
            } else {
                let y = hr.preferred_yaw.or(self.held_yaw).unwrap_or(m.yaw);
                self.held_yaw = Some(y);
                (y, v_air_ref.dot(Vec2::from_angle(m.yaw)))
            };
            let airspeed_ref =
                airspeed_ref.clamp(-settings.max_reverse_airspeed_m_s, r.max_airspeed);

            let yaw_err = wrap_pi(yaw_ref - m.yaw);
            let heading_lim = PidLimits {
                derivative_tau_s: tau,
                ..PidLimits::symmetric(r.max_yaw_rate)
            };
            let heading_lim = PidLimits {
                integrator_min: -0.2 * r.max_yaw_rate,
                integrator_max: 0.2 * r.max_yaw_rate,
                ..heading_lim
            };
            let r_ref = self.pid_heading.update(
                &gains.heading,
                &heading_lim,
                yaw_err,
                m.yaw,
                Some(m.yaw_rate),
                0.0,
                dt,
            );
            let yaw_lim = PidLimits {
                derivative_tau_s: tau,
                rate_limit: settings.yaw_moment_rate_limit_nm_s,
                ..PidLimits::symmetric(limits.yaw_max_nm)
            };
            let n = self.pid_yaw_rate.update(
                &gains.yaw_rate,
                &yaw_lim,
                r_ref - m.yaw_rate,
                m.yaw_rate,
                None,
                0.0,
                dt,
            );

            let drag_ff = if settings.drag_feedforward {
                0.5 * rho * model.axial_drag_area * airspeed_ref * airspeed_ref.abs()
            } else {
                0.0
            };
            let surge_lim = PidLimits {
                derivative_tau_s: tau,
                rate_limit: settings.surge_rate_limit_n_s,
                ..PidLimits::range(limits.surge_min_n, limits.surge_max_n)
            };
            let fx = if r.surge_enabled {
                self.pid_airspeed.update(
                    &gains.airspeed,
                    &surge_lim,
                    airspeed_ref - m.airspeed_forward,
                    m.airspeed_forward,
                    None,
                    drag_ff,
                    dt,
                )
            } else {
                self.pid_airspeed.reset();
                0.0
            };
            (
                fx,
                n,
                InternalReferences {
                    vertical_speed: vz_ref,
                    yaw: yaw_ref,
                    yaw_rate: r_ref,
                    airspeed: airspeed_ref,
                    ground_velocity: v_ground_ref,
                },
            )
        } else {
            self.pid_heading.reset();
            self.pid_yaw_rate.reset();
            self.pid_airspeed.reset();
            self.tracking_air_velocity = false;
            self.held_yaw = None;
            self.repointing = false;
            (
                0.0,
                0.0,
                InternalReferences {
                    vertical_speed: vz_ref,
                    yaw: m.yaw,
                    ..Default::default()
                },
            )
        };

        let virtual_command = VirtualCommand {
            surge_n: surge,
            heave_n: heave,
            yaw_moment_nm: yaw_moment,
        };
        let allocation = allocator.allocate(&virtual_command, m.airspeed_forward, rho);
        let h_active = r.horizontal_active;
        let saturation = Saturation {
            surge: allocation.surge_saturated
                || (h_active && r.surge_enabled && self.pid_airspeed.last.saturated),
            heave: allocation.heave_saturated
                || (r.heave_override_n.is_none() && self.pid_vz.last.saturated),
            yaw: allocation.yaw_saturated || (h_active && self.pid_yaw_rate.last.saturated),
        };
        ControlOutput {
            virtual_command,
            allocation,
            references: refs_h,
            gains: *gains,
            saturation,
        }
    }
}
