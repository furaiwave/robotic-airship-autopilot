// Client-side form validation (Zod). The Rust backend validates every request
// independently; these schemas give immediate feedback and mirror its ranges.
import { z } from 'zod'

export type FieldKind = 'input' | 'estimate'

export interface FieldSpec {
  /** Dotted path relative to the form's section object. */
  name: string
  label: string
  unit?: string
  min: number
  max: number
  step?: number
  help?: string
  /** `input`: user parameter. `estimate`: a modelling assumption the user may tune. */
  kind?: FieldKind
  /** Nullable field: `null` means "automatic" (e.g. derived hull volume). */
  nullable?: boolean
  integer?: boolean
}

export interface FieldGroup {
  title: string
  description?: string
  fields: FieldSpec[]
}

function numberSchema(f: FieldSpec) {
  let s = z.number({ error: `${f.label} must be a number` }).min(f.min, `${f.label}: minimum ${f.min}${f.unit ? ` ${f.unit}` : ''}`).max(f.max, `${f.label}: maximum ${f.max}${f.unit ? ` ${f.unit}` : ''}`)
  if (f.integer) s = s.int(`${f.label} must be an integer`)
  return f.nullable ? s.nullable() : s
}

type Tree = { [k: string]: Tree | z.ZodType }

/** Build a nested Zod object schema from dotted field paths. */
export function buildSchema(fields: FieldSpec[]): z.ZodObject<Record<string, z.ZodType>> {
  const tree: Tree = {}
  for (const f of fields) {
    const parts = f.name.split('.')
    let node = tree
    parts.slice(0, -1).forEach((p) => {
      node[p] ??= {}
      node = node[p] as Tree
    })
    node[parts[parts.length - 1] as string] = numberSchema(f)
  }
  const toZod = (t: Tree): z.ZodObject<Record<string, z.ZodType>> =>
    z.object(Object.fromEntries(Object.entries(t).map(([k, v]) => [k, v instanceof z.ZodType ? v : toZod(v)]))).passthrough()
  return toZod(tree)
}

// ---------------------------------------------------------------- airship

export const AIRSHIP_GROUPS: FieldGroup[] = [
  {
    title: 'Hull geometry and mass',
    fields: [
      { name: 'length_m', label: 'Envelope length L', unit: 'm', min: 1, max: 250, step: 0.1 },
      { name: 'max_diameter_m', label: 'Max. diameter D', unit: 'm', min: 0.3, max: 60, step: 0.1 },
      { name: 'hull_volume_m3', label: 'Hull volume', unit: 'm³', min: 0.01, max: 1e6, step: 0.1, nullable: true, help: 'Empty = prolate-ellipsoid estimate π·L·D²/6' },
      { name: 'gondola_volume_m3', label: 'Gondola / fin volume', unit: 'm³', min: 0, max: 1e4, step: 0.05, help: 'Additional displaced volume outside the envelope' },
      { name: 'total_mass_kg', label: 'Total mass m', unit: 'kg', min: 0.1, max: 1e6, step: 0.1, help: 'Structure + lifting gas + payload + battery' },
      { name: 'cg_below_cv_m', label: 'CG below CV (BG)', unit: 'm', min: 0.05, max: 50, step: 0.05, help: 'Pendulum arm; sets pitch/roll restoring stiffness B·BG' },
      { name: 'gear_height_m', label: 'Gear below CV', unit: 'm', min: 0.1, max: 60, step: 0.05 },
    ],
  },
  {
    title: 'Aerodynamics',
    description: 'Reference areas are derived from L and D; coefficients are modelling estimates.',
    fields: [
      { name: 'aero.cd_axial', label: 'Axial drag C_Dx', min: 0.01, max: 1.5, step: 0.005, kind: 'estimate', help: 'Referenced to the frontal area πD²/4' },
      { name: 'aero.cd_lateral', label: 'Lateral drag C_Dy', min: 0.1, max: 2, step: 0.05, kind: 'estimate', help: 'Referenced to the side area πLD/4' },
      { name: 'aero.cd_vertical', label: 'Vertical drag C_Dz', min: 0.1, max: 2, step: 0.05, kind: 'estimate', help: 'Referenced to the planform area πLD/4' },
      { name: 'aero.fin_area_m2', label: 'Vertical fin area S', unit: 'm²', min: 0, max: 1e4, step: 0.1 },
      { name: 'aero.fin_lift_slope_per_rad', label: 'Fin lift slope C_Lα', unit: '1/rad', min: 0, max: 6.3, step: 0.1, kind: 'estimate' },
      { name: 'aero.fin_arm_m', label: 'Fin arm l_f', unit: 'm', min: 0, max: 200, step: 0.1 },
      { name: 'aero.pitch_roll_damping_ratio', label: 'Pitch/roll damping ζ', min: 0.05, max: 2, step: 0.05, kind: 'estimate' },
    ],
  },
  {
    title: 'Propulsion and energy',
    fields: [
      { name: 'propulsion.main_max_thrust_n', label: 'Main thrust (each)', unit: 'N', min: 0.1, max: 1e5, step: 1 },
      { name: 'propulsion.main_reverse_ratio', label: 'Main reverse ratio', min: 0, max: 1, step: 0.05 },
      { name: 'propulsion.main_lateral_arm_m', label: 'Main lateral arm b', unit: 'm', min: 0.05, max: 50, step: 0.05 },
      { name: 'propulsion.main_vertical_offset_m', label: 'Thrust line below CG', unit: 'm', min: -20, max: 20, step: 0.05 },
      { name: 'propulsion.heave_max_thrust_n', label: 'Heave thrust (up)', unit: 'N', min: 0, max: 1e5, step: 1 },
      { name: 'propulsion.heave_reverse_ratio', label: 'Heave reverse ratio', min: 0, max: 1, step: 0.05 },
      { name: 'propulsion.response_time_s', label: 'Propeller response τ', unit: 's', min: 0.02, max: 5, step: 0.01 },
      { name: 'propulsion.propeller_diameter_m', label: 'Propeller diameter', unit: 'm', min: 0.05, max: 10, step: 0.01 },
      { name: 'propulsion.efficiency', label: 'Overall efficiency η', min: 0.1, max: 1, step: 0.01, kind: 'estimate' },
      { name: 'propulsion.battery_capacity_wh', label: 'Battery capacity', unit: 'Wh', min: 1, max: 1e7, step: 10 },
      { name: 'propulsion.avionics_power_w', label: 'Avionics power', unit: 'W', min: 0, max: 1e5, step: 1 },
    ],
  },
  {
    title: 'Rudder',
    fields: [
      { name: 'rudder.max_deflection_deg', label: 'Max deflection', unit: 'deg', min: 0, max: 45, step: 1 },
      { name: 'rudder.max_rate_deg_s', label: 'Max rate', unit: 'deg/s', min: 1, max: 500, step: 1 },
      { name: 'rudder.effectiveness_per_rad', label: 'Effectiveness C_Lδ', unit: '1/rad', min: 0, max: 6.3, step: 0.1, kind: 'estimate' },
    ],
  },
  {
    title: 'Operating limits',
    description: 'Used by reference generation and the safety monitor.',
    fields: [
      { name: 'limits.max_climb_rate_m_s', label: 'Max climb rate', unit: 'm/s', min: 0.05, max: 20, step: 0.1 },
      { name: 'limits.max_descent_rate_m_s', label: 'Max descent rate', unit: 'm/s', min: 0.05, max: 20, step: 0.1 },
      { name: 'limits.max_airspeed_m_s', label: 'Max airspeed', unit: 'm/s', min: 0.5, max: 60, step: 0.5 },
      { name: 'limits.max_turn_rate_deg_s', label: 'Max turn rate', unit: 'deg/s', min: 0.5, max: 90, step: 0.5 },
      { name: 'limits.max_pitch_deg', label: 'Max pitch', unit: 'deg', min: 1, max: 60, step: 1 },
      { name: 'limits.max_roll_deg', label: 'Max roll', unit: 'deg', min: 1, max: 60, step: 1 },
    ],
  },
]

export const airshipSchema = buildSchema(AIRSHIP_GROUPS.flatMap((g) => g.fields)).superRefine((v, ctx) => {
  const a = v as { length_m: number; max_diameter_m: number; gear_height_m: number }
  if (a.length_m < a.max_diameter_m) ctx.addIssue({ code: 'custom', path: ['length_m'], message: 'Length must not be smaller than the diameter' })
  if (a.gear_height_m < a.max_diameter_m / 2) ctx.addIssue({ code: 'custom', path: ['gear_height_m'], message: 'Gear contact must be at least D/2 below the CV' })
})

// ---------------------------------------------------------------- environment

export const ENVIRONMENT_GROUPS: FieldGroup[] = [
  {
    title: 'Terrain, sites and atmosphere',
    fields: [
      { name: 'ground_elevation_m', label: 'Ground elevation', unit: 'm MSL', min: -400, max: 5000, step: 1 },
      { name: 'atmosphere.temperature_offset_k', label: 'ISA temperature offset', unit: 'K', min: -40, max: 40, step: 1 },
      { name: 'atmosphere.constant_density_kg_m3', label: 'Constant density', unit: 'kg/m³', min: 0.3, max: 1.5, step: 0.005, help: 'Used only by the constant model' },
      { name: 'takeoff_site.x', label: 'Take-off site East', unit: 'm', min: -1e5, max: 1e5, step: 1 },
      { name: 'takeoff_site.y', label: 'Take-off site North', unit: 'm', min: -1e5, max: 1e5, step: 1 },
      { name: 'landing_site.x', label: 'Landing point East', unit: 'm', min: -1e5, max: 1e5, step: 1 },
      { name: 'landing_site.y', label: 'Landing point North', unit: 'm', min: -1e5, max: 1e5, step: 1 },
    ],
  },
  {
    title: 'Landing area (available ground)',
    fields: [
      { name: 'landing_area.heading_deg', label: 'Landing axis heading', unit: 'deg', min: 0, max: 360, step: 1 },
      { name: 'landing_area.undershoot_m', label: 'Usable before point', unit: 'm', min: 1, max: 5000, step: 1 },
      { name: 'landing_area.overrun_m', label: 'Usable beyond point', unit: 'm', min: 1, max: 5000, step: 1 },
      { name: 'landing_area.half_width_m', label: 'Half width', unit: 'm', min: 1, max: 5000, step: 1 },
    ],
  },
  {
    title: 'Initial state',
    fields: [
      { name: 'initial_state.position.x', label: 'Position East', unit: 'm', min: -1e5, max: 1e5, step: 1 },
      { name: 'initial_state.position.y', label: 'Position North', unit: 'm', min: -1e5, max: 1e5, step: 1 },
      { name: 'initial_state.altitude_agl_m', label: 'Gear height AGL', unit: 'm', min: 0, max: 3000, step: 0.5 },
      { name: 'initial_state.velocity.x', label: 'Velocity East', unit: 'm/s', min: -30, max: 30, step: 0.1 },
      { name: 'initial_state.velocity.y', label: 'Velocity North', unit: 'm/s', min: -30, max: 30, step: 0.1 },
      { name: 'initial_state.velocity.z', label: 'Velocity up', unit: 'm/s', min: -30, max: 30, step: 0.1 },
      { name: 'initial_state.heading_deg', label: 'Heading', unit: 'deg', min: 0, max: 360, step: 1 },
      { name: 'initial_state.pitch_deg', label: 'Pitch', unit: 'deg', min: -30, max: 30, step: 0.5 },
      { name: 'initial_state.roll_deg', label: 'Roll', unit: 'deg', min: -30, max: 30, step: 0.5 },
    ],
  },
  {
    title: 'Mean wind',
    fields: [
      { name: 'wind.speed_m_s', label: 'Wind speed (at ref. height)', unit: 'm/s', min: 0, max: 30, step: 0.1 },
      { name: 'wind.from_direction_deg', label: 'Blowing from', unit: 'deg', min: 0, max: 360, step: 1 },
      { name: 'wind.vertical_m_s', label: 'Vertical wind', unit: 'm/s', min: -5, max: 5, step: 0.1 },
      { name: 'wind.shear_exponent', label: 'Shear exponent α', min: 0, max: 0.6, step: 0.01, kind: 'estimate' },
      { name: 'wind.shear_reference_height_m', label: 'Shear reference height', unit: 'm', min: 1, max: 500, step: 1 },
    ],
  },
  {
    title: 'Turbulence (Gauss–Markov)',
    fields: [
      { name: 'wind.turbulence.intensity_m_s', label: 'Intensity σ', unit: 'm/s', min: 0, max: 10, step: 0.05 },
      { name: 'wind.turbulence.vertical_ratio', label: 'Vertical/horizontal ratio', min: 0, max: 2, step: 0.05 },
      { name: 'wind.turbulence.correlation_time_s', label: 'Correlation time T_c', unit: 's', min: 0.1, max: 600, step: 0.1 },
      { name: 'wind.turbulence.seed', label: 'Random seed', min: 0, max: Number.MAX_SAFE_INTEGER, step: 1, integer: true },
    ],
  },
  {
    title: 'Sensor noise (1σ)',
    fields: [
      { name: 'sensor_noise.position_m', label: 'Position', unit: 'm', min: 0, max: 20, step: 0.05 },
      { name: 'sensor_noise.altitude_m', label: 'Altitude', unit: 'm', min: 0, max: 20, step: 0.05 },
      { name: 'sensor_noise.velocity_m_s', label: 'Velocity', unit: 'm/s', min: 0, max: 5, step: 0.01 },
      { name: 'sensor_noise.heading_deg', label: 'Heading', unit: 'deg', min: 0, max: 30, step: 0.1 },
      { name: 'sensor_noise.airspeed_m_s', label: 'Air data', unit: 'm/s', min: 0, max: 5, step: 0.01 },
      { name: 'sensor_noise.seed', label: 'Random seed', min: 0, max: Number.MAX_SAFE_INTEGER, step: 1, integer: true },
    ],
  },
]

export const environmentSchema = buildSchema(ENVIRONMENT_GROUPS.flatMap((g) => g.fields))

// ---------------------------------------------------------------- autopilot

export const AUTOPILOT_GROUPS: FieldGroup[] = [
  {
    title: 'Targets',
    fields: [
      { name: 'target_altitude_m', label: 'Target altitude', unit: 'm AGL', min: 5, max: 2000, step: 1 },
      { name: 'cruise_speed_m_s', label: 'Target horizontal speed', unit: 'm/s', min: 0.5, max: 60, step: 0.1 },
      { name: 'takeoff_climb_rate_m_s', label: 'Target vertical speed (take-off)', unit: 'm/s', min: 0.1, max: 20, step: 0.05 },
      { name: 'takeoff_heading_deg', label: 'Take-off heading (calm)', unit: 'deg', min: 0, max: 360, step: 1, nullable: true, help: 'Empty = keep the initial heading; with wind the nose points into the wind' },
      { name: 'trim_mass_bias_kg', label: 'Trim mass bias', unit: 'kg', min: -1000, max: 1000, step: 0.5, kind: 'estimate', help: 'Model mismatch of the onboard heaviness estimate (robustness studies)' },
    ],
  },
  {
    title: 'Take-off thresholds',
    fields: [
      { name: 'takeoff.lift_margin_n', label: 'Lift margin', unit: 'N', min: 0, max: 1e5, step: 1 },
      { name: 'takeoff.thrust_ramp_n_s', label: 'Heave ramp rate', unit: 'N/s', min: 0.5, max: 1e5, step: 0.5 },
      { name: 'takeoff.release_load_threshold_n', label: 'Release mooring load', unit: 'N', min: 0, max: 1e5, step: 0.5 },
      { name: 'takeoff.preparation_timeout_s', label: 'Preparation timeout', unit: 's', min: 1, max: 600, step: 1 },
      { name: 'takeoff.clearance_altitude_m', label: 'Clearance height', unit: 'm', min: 0.5, max: 100, step: 0.5 },
      { name: 'takeoff.initial_climb_altitude_m', label: 'Initial-climb height', unit: 'm', min: 1, max: 500, step: 0.5 },
      { name: 'takeoff.takeoff_timeout_s', label: 'Take-off timeout', unit: 's', min: 5, max: 600, step: 1 },
      { name: 'takeoff.max_wind_m_s', label: 'Max take-off wind', unit: 'm/s', min: 0, max: 30, step: 0.5 },
      { name: 'takeoff.min_battery_soc', label: 'Min battery SoC', min: 0, max: 1, step: 0.05 },
    ],
  },
  {
    title: 'Approach',
    fields: [
      { name: 'approach.course_deg', label: 'Final course', unit: 'deg', min: 0, max: 360, step: 1, nullable: true, help: 'Empty = landing-area heading' },
      { name: 'approach.glide_angle_deg', label: 'Approach angle', unit: 'deg', min: 2, max: 30, step: 0.5 },
      { name: 'approach.altitude_m', label: 'Approach / go-around altitude', unit: 'm', min: 5, max: 500, step: 1 },
      { name: 'approach.speed_m_s', label: 'Approach speed', unit: 'm/s', min: 0.5, max: 60, step: 0.1 },
      { name: 'approach.pregate_extension_m', label: 'Pre-gate extension', unit: 'm', min: 10, max: 2000, step: 5 },
      { name: 'approach.corridor_half_width_m', label: 'Corridor half-width', unit: 'm', min: 1, max: 500, step: 1 },
      { name: 'approach.corridor_altitude_tolerance_m', label: 'Corridor altitude tol.', unit: 'm', min: 0.5, max: 100, step: 0.5 },
      { name: 'approach.max_course_error_deg', label: 'Max track error', unit: 'deg', min: 5, max: 90, step: 1 },
      { name: 'approach.deceleration_distance_m', label: 'Deceleration distance', unit: 'm', min: 5, max: 1000, step: 5 },
    ],
  },
  {
    title: 'Landing and abort conditions',
    fields: [
      { name: 'landing.flare_height_m', label: 'Flare height', unit: 'm', min: 0.5, max: 50, step: 0.1 },
      { name: 'landing.flare_entry_radius_m', label: 'Flare entry radius', unit: 'm', min: 0.5, max: 200, step: 0.5 },
      { name: 'landing.flare_entry_max_speed_m_s', label: 'Flare entry max speed', unit: 'm/s', min: 0.1, max: 10, step: 0.1 },
      { name: 'landing.touchdown_sink_rate_m_s', label: 'Touchdown sink rate', unit: 'm/s', min: 0.05, max: 20, step: 0.05 },
      { name: 'landing.landing_point_tolerance_m', label: 'Landing-point tolerance', unit: 'm', min: 0.5, max: 500, step: 0.5 },
      { name: 'landing.max_touchdown_vertical_speed_m_s', label: 'Touchdown vz limit', unit: 'm/s', min: 0.05, max: 10, step: 0.05 },
      { name: 'landing.max_touchdown_horizontal_speed_m_s', label: 'Touchdown vh limit', unit: 'm/s', min: 0.05, max: 10, step: 0.05 },
      { name: 'landing.decision_height_m', label: 'Decision height', unit: 'm', min: 0, max: 50, step: 0.1 },
      { name: 'landing.hold_down_force_n', label: 'Hold-down force', unit: 'N', min: 0, max: 1e5, step: 1 },
      { name: 'landing.settle_time_s', label: 'Settle time', unit: 's', min: 0, max: 120, step: 0.5 },
      { name: 'landing.max_attempts', label: 'Max landing attempts', min: 1, max: 10, step: 1, integer: true },
      { name: 'landing.max_wind_m_s', label: 'Max landing wind', unit: 'm/s', min: 0, max: 30, step: 0.5 },
      { name: 'landing.max_cross_track_error_m', label: 'Max cross-track error', unit: 'm', min: 0.5, max: 500, step: 0.5 },
      { name: 'landing.max_altitude_error_m', label: 'Max glide-path error', unit: 'm', min: 0.5, max: 500, step: 0.5 },
    ],
  },
  {
    title: 'Control output limits and safety',
    fields: [
      { name: 'control_limits.surge', label: 'Surge authority', min: 0.05, max: 1, step: 0.05 },
      { name: 'control_limits.heave', label: 'Heave authority', min: 0.05, max: 1, step: 0.05 },
      { name: 'control_limits.yaw', label: 'Yaw authority', min: 0.05, max: 1, step: 0.05 },
      { name: 'safety.critical_factor', label: 'Critical factor', min: 1, max: 3, step: 0.05 },
      { name: 'safety.min_enroute_altitude_m', label: 'Min en-route altitude', unit: 'm', min: 0, max: 500, step: 1 },
      { name: 'safety.saturation_warning_time_s', label: 'Saturation warning time', unit: 's', min: 0.1, max: 600, step: 0.5 },
      { name: 'safety.battery_warning_soc', label: 'Battery warning SoC', min: 0, max: 1, step: 0.05 },
      { name: 'safety.battery_critical_soc', label: 'Battery critical SoC', min: 0, max: 1, step: 0.05 },
    ],
  },
]

export const autopilotSchema = buildSchema(AUTOPILOT_GROUPS.flatMap((g) => g.fields)).superRefine((v, ctx) => {
  const a = v as { landing: { decision_height_m: number; flare_height_m: number; touchdown_sink_rate_m_s: number; max_touchdown_vertical_speed_m_s: number }; takeoff: { clearance_altitude_m: number; initial_climb_altitude_m: number } }
  if (a.landing.decision_height_m >= a.landing.flare_height_m)
    ctx.addIssue({ code: 'custom', path: ['landing', 'decision_height_m'], message: 'Decision height must be below the flare height' })
  if (a.landing.touchdown_sink_rate_m_s >= a.landing.max_touchdown_vertical_speed_m_s)
    ctx.addIssue({ code: 'custom', path: ['landing', 'touchdown_sink_rate_m_s'], message: 'Sink rate must be below the touchdown limit' })
  if (a.takeoff.initial_climb_altitude_m <= a.takeoff.clearance_altitude_m)
    ctx.addIssue({ code: 'custom', path: ['takeoff', 'initial_climb_altitude_m'], message: 'Must be above the clearance height' })
})

// ---------------------------------------------------------------- simulation settings

export const SIMULATION_FIELDS: FieldSpec[] = [
  { name: 'duration_s', label: 'Duration', unit: 's', min: 1, max: 7200, step: 10 },
  { name: 'dt_s', label: 'Physics time step', unit: 's', min: 0.001, max: 0.05, step: 0.001 },
  { name: 'control_rate_hz', label: 'Control rate', unit: 'Hz', min: 1, max: 200, step: 1 },
  { name: 'telemetry_rate_hz', label: 'Telemetry rate', unit: 'Hz', min: 0.5, max: 100, step: 0.5 },
]

export const simulationSchema = buildSchema(SIMULATION_FIELDS).superRefine((v, ctx) => {
  const s = v as { dt_s: number; control_rate_hz: number; telemetry_rate_hz: number }
  const multiple = (period: number) => {
    const n = period / s.dt_s
    return Math.abs(n - Math.round(n)) < 1e-6 && Math.round(n) >= 1
  }
  if (!multiple(1 / s.control_rate_hz)) ctx.addIssue({ code: 'custom', path: ['control_rate_hz'], message: 'Control period must be a multiple of dt' })
  if (!multiple(1 / s.telemetry_rate_hz)) ctx.addIssue({ code: 'custom', path: ['telemetry_rate_hz'], message: 'Telemetry period must be a multiple of dt' })
})

// ---------------------------------------------------------------- waypoints

export const waypointSchema = z.object({
  x_m: z.number().min(-1e5).max(1e5),
  y_m: z.number().min(-1e5).max(1e5),
  altitude_m: z.number().min(0, 'Altitude must be ≥ 0').max(2000),
  heading_deg: z.number().min(0).max(360).nullable(),
  speed_m_s: z.number().min(0, 'Speed must be ≥ 0').max(60),
  tolerance_m: z.number().min(1, 'Tolerance must be ≥ 1 m').max(500),
})

export const flightPathSchema = z
  .object({ name: z.string().min(1), waypoints: z.array(waypointSchema).max(100) })
  .superRefine((p, ctx) => {
    p.waypoints.forEach((w, i) => {
      const prev = p.waypoints[i - 1]
      if (prev) {
        const d = Math.hypot(w.x_m - prev.x_m, w.y_m - prev.y_m)
        if (d < Math.max(w.tolerance_m, prev.tolerance_m))
          ctx.addIssue({ code: 'custom', path: ['waypoints', i], message: `Waypoint ${i + 1} lies inside the acceptance radius of waypoint ${i}` })
      }
    })
  })

export type WaypointInput = z.infer<typeof waypointSchema>
