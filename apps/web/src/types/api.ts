// TypeScript mirror of the Rust API contract (serde, snake_case).
// Units are part of the field names. The Rust engine is authoritative; the client
// never re-implements the physical model.

export interface Vec2 {
  x: number
  y: number
}

export interface Vec3 {
  x: number
  y: number
  z: number
}

// ---------------------------------------------------------------- vehicle

export type LiftingGas = 'helium' | 'hydrogen'

export interface AeroParams {
  cd_axial: number
  cd_lateral: number
  cd_vertical: number
  fin_area_m2: number
  fin_lift_slope_per_rad: number
  fin_arm_m: number
  pitch_roll_damping_ratio: number
}

export interface PropulsionParams {
  main_max_thrust_n: number
  main_reverse_ratio: number
  main_lateral_arm_m: number
  main_vertical_offset_m: number
  heave_max_thrust_n: number
  heave_reverse_ratio: number
  propeller_diameter_m: number
  response_time_s: number
  efficiency: number
  battery_capacity_wh: number
  avionics_power_w: number
}

export interface RudderParams {
  max_deflection_deg: number
  max_rate_deg_s: number
  effectiveness_per_rad: number
}

export interface OperatingLimits {
  max_climb_rate_m_s: number
  max_descent_rate_m_s: number
  max_airspeed_m_s: number
  max_turn_rate_deg_s: number
  max_pitch_deg: number
  max_roll_deg: number
}

export interface AirshipParams {
  name: string
  length_m: number
  max_diameter_m: number
  hull_volume_m3: number | null
  gondola_volume_m3: number
  total_mass_kg: number
  lifting_gas: LiftingGas
  cg_below_cv_m: number
  gear_height_m: number
  aero: AeroParams
  propulsion: PropulsionParams
  rudder: RudderParams
  limits: OperatingLimits
}

export type BuoyancyState = 'light' | 'neutral' | 'heavy'

export interface DerivedProperties {
  reference_density_kg_m3: number
  hull_volume_m3: number
  hull_volume_is_estimate: boolean
  displaced_volume_m3: number
  fineness_ratio: number
  frontal_area_m2: number
  lateral_area_m2: number
  planform_area_m2: number
  buoyancy_n: number
  weight_n: number
  static_lift_n: number
  buoyancy_ratio: number
  buoyancy_state: BuoyancyState
  gas_mass_estimate_kg: number
  lamb_k1: number
  lamb_k2: number
  lamb_k_rot: number
  added_mass_axial_kg: number
  added_mass_transverse_kg: number
  effective_mass_axial_kg: number
  effective_mass_lateral_kg: number
  effective_mass_vertical_kg: number
  inertia_roll_kg_m2: number
  inertia_pitch_yaw_kg_m2: number
  added_inertia_pitch_yaw_kg_m2: number
  effective_inertia_roll_kg_m2: number
  effective_inertia_pitch_yaw_kg_m2: number
  pendulum_stiffness_nm_rad: number
  pitch_natural_frequency_rad_s: number
  roll_natural_frequency_rad_s: number
  pitch_damping_nms_rad: number
  roll_damping_nms_rad: number
  hull_yaw_damping_coeff: number
  fin_yaw_damping_coeff: number
  max_forward_thrust_n: number
  max_reverse_thrust_n: number
  max_heave_up_n: number
  max_heave_down_n: number
  max_differential_yaw_moment_nm: number
  rudder_moment_per_pa: number
  rudder_crossover_airspeed_m_s: number
  propeller_disk_area_m2: number
  estimated_max_level_speed_m_s: number
  estimated_max_climb_rate_m_s: number
  estimated_max_descent_rate_m_s: number
  hover_heave_thrust_n: number
  hover_power_w: number
}

// ---------------------------------------------------------------- environment

export type AtmosphereModel = 'constant' | 'isa'

export interface AtmosphereConfig {
  model: AtmosphereModel
  constant_density_kg_m3: number
  temperature_offset_k: number
}

export type DisturbanceTrigger =
  | { kind: 'at_time'; time_s: number }
  | { kind: 'distance_to_landing'; distance_m: number }

export interface WindChange {
  trigger: DisturbanceTrigger
  ramp_duration_s: number
  speed_m_s: number
  from_direction_deg: number
  vertical_m_s: number
}

export interface DiscreteGust {
  trigger: DisturbanceTrigger
  duration_s: number
  speed_m_s: number
  from_direction_deg: number
  vertical_m_s: number
}

export interface TurbulenceConfig {
  enabled: boolean
  intensity_m_s: number
  vertical_ratio: number
  correlation_time_s: number
  seed: number
}

export interface WindConfig {
  speed_m_s: number
  from_direction_deg: number
  vertical_m_s: number
  shear_exponent: number
  shear_reference_height_m: number
  changes: WindChange[]
  gusts: DiscreteGust[]
  turbulence: TurbulenceConfig
}

export interface SensorNoiseConfig {
  enabled: boolean
  seed: number
  position_m: number
  altitude_m: number
  velocity_m_s: number
  heading_deg: number
  airspeed_m_s: number
}

export interface LandingArea {
  heading_deg: number
  undershoot_m: number
  overrun_m: number
  half_width_m: number
}

export interface InitialStateConfig {
  position: Vec2
  altitude_agl_m: number
  velocity: Vec3
  heading_deg: number
  pitch_deg: number
  roll_deg: number
  moored: boolean
}

export interface EnvironmentConfig {
  atmosphere: AtmosphereConfig
  ground_elevation_m: number
  takeoff_site: Vec2
  landing_site: Vec2
  landing_area: LandingArea
  initial_state: InitialStateConfig
  wind: WindConfig
  sensor_noise: SensorNoiseConfig
}

// ---------------------------------------------------------------- flight path

export interface Waypoint {
  x_m: number
  y_m: number
  altitude_m: number
  heading_deg: number | null
  speed_m_s: number
  tolerance_m: number
}

export interface FlightPath {
  name: string
  waypoints: Waypoint[]
}

export type PathTemplate =
  | 'vertical_climb'
  | 'climb_and_level'
  | 'straight_approach'
  | 'curved_approach'
  | 'descent_to_landing'
  | 'go_around_climb_out'

export interface TemplateResponse {
  template: PathTemplate
  description: string
  path: FlightPath
}

// ---------------------------------------------------------------- autopilot

export interface TakeoffConfig {
  lift_margin_n: number
  thrust_ramp_n_s: number
  release_load_threshold_n: number
  preparation_timeout_s: number
  clearance_altitude_m: number
  initial_climb_altitude_m: number
  takeoff_timeout_s: number
  max_wind_m_s: number
  min_battery_soc: number
}

export interface ApproachConfig {
  course_deg: number | null
  glide_angle_deg: number
  altitude_m: number
  speed_m_s: number
  pregate_extension_m: number
  corridor_half_width_m: number
  corridor_altitude_tolerance_m: number
  max_course_error_deg: number
  deceleration_distance_m: number
}

export interface LandingConfig {
  flare_height_m: number
  flare_entry_radius_m: number
  flare_entry_max_speed_m_s: number
  touchdown_sink_rate_m_s: number
  landing_point_tolerance_m: number
  max_touchdown_vertical_speed_m_s: number
  max_touchdown_horizontal_speed_m_s: number
  decision_height_m: number
  hold_down_force_n: number
  settle_speed_m_s: number
  settle_time_s: number
  max_attempts: number
  max_wind_m_s: number
  max_cross_track_error_m: number
  max_altitude_error_m: number
}

export interface TransitionTolerances {
  altitude_m: number
  vertical_speed_m_s: number
  hover_altitude_m: number
  hover_heading_deg: number
}

export interface GuidanceConfig {
  lookahead_m: number
  hold_max_speed_m_s: number
  hover_deceleration_m_s2: number
  wind_filter_tau_s: number
}

export interface ControlLimitFractions {
  surge: number
  heave: number
  yaw: number
}

export interface SafetyConfig {
  critical_factor: number
  min_enroute_altitude_m: number
  saturation_warning_time_s: number
  battery_warning_soc: number
  battery_critical_soc: number
  go_around_on_critical: boolean
}

export interface AutopilotConfig {
  target_altitude_m: number
  cruise_speed_m_s: number
  takeoff_climb_rate_m_s: number
  takeoff_heading_deg: number | null
  trim_mass_bias_kg: number
  takeoff: TakeoffConfig
  approach: ApproachConfig
  landing: LandingConfig
  transitions: TransitionTolerances
  guidance: GuidanceConfig
  control_limits: ControlLimitFractions
  safety: SafetyConfig
}

// ---------------------------------------------------------------- controllers

export type ControllerKind = 'cascaded_pid' | 'gain_scheduled'

export interface PidGains {
  kp: number
  ki: number
  kd: number
}

export interface CascadedPidConfig {
  altitude: PidGains
  vertical_speed: PidGains
  heading: PidGains
  yaw_rate: PidGains
  airspeed: PidGains
  position_gain_1_s: number
}

export interface CommonControlSettings {
  derivative_filter_tau_s: number
  heave_rate_limit_n_s: number
  surge_rate_limit_n_s: number
  yaw_moment_rate_limit_nm_s: number
  drag_feedforward: boolean
  max_reverse_airspeed_m_s: number
}

export interface Bandwidths {
  vertical_rad_s: number
  yaw_rad_s: number
  speed_rad_s: number
}

export type PhaseClass = 'ground' | 'takeoff' | 'climb' | 'cruise' | 'approach' | 'landing'

export type PhaseBandwidthTable = Record<PhaseClass, Bandwidths>

export interface GainScheduleConfig {
  damping_ratio: number
  outer_loop_ratio: number
  min_proportional_fraction: number
  bandwidths: PhaseBandwidthTable
  altitude_integral_fraction: number
  position_gain_far_1_s: number
  position_gain_near_1_s: number
  position_schedule_distance_m: number
}

export interface ControllerConfig {
  kind: ControllerKind
  common: CommonControlSettings
  cascaded_pid: CascadedPidConfig
  gain_scheduled: GainScheduleConfig
}

// ---------------------------------------------------------------- simulation

export interface SimulationSettings {
  duration_s: number
  dt_s: number
  control_rate_hz: number
  telemetry_rate_hz: number
}

export interface SimulationConfig {
  name: string
  description: string
  airship: AirshipParams
  environment: EnvironmentConfig
  flight_path: FlightPath
  autopilot: AutopilotConfig
  controller: ControllerConfig
  simulation: SimulationSettings
}

export type Severity = 'error' | 'warning'

export interface ValidationIssue {
  field: string
  message: string
  severity: Severity
}

export interface ApproachGeometry {
  landing: Vec2
  course_yaw: number
  course_dir: Vec2
  glide_tan: number
  approach_altitude: number
  flare_height: number
  gate_distance: number
  pregate_extension: number
}

export interface ValidationResponse {
  valid: boolean
  issues: ValidationIssue[]
  derived: DerivedProperties | null
  ground_density_kg_m3: number
  approach: ApproachGeometry | null
}

// ---------------------------------------------------------------- runtime

export type FlightPhase =
  | 'preflight'
  | 'takeoff_preparation'
  | 'takeoff'
  | 'initial_climb'
  | 'climb'
  | 'cruise'
  | 'approach'
  | 'descent'
  | 'flare'
  | 'touchdown'
  | 'post_landing'
  | 'landed'
  | 'go_around'
  | 'aborted'

export type RunState = 'created' | 'running' | 'paused' | 'finished'
export type FinishReason = 'landed' | 'aborted' | 'timeout' | 'fatal' | 'stopped'

export interface TelemetrySample {
  t_s: number
  phase: FlightPhase
  x_m: number
  y_m: number
  altitude_msl_m: number
  altitude_agl_m: number
  vx_m_s: number
  vy_m_s: number
  vz_m_s: number
  ground_speed_m_s: number
  airspeed_m_s: number
  airspeed_forward_m_s: number
  heading_deg: number
  pitch_deg: number
  roll_deg: number
  yaw_rate_deg_s: number
  wind_x_m_s: number
  wind_y_m_s: number
  wind_z_m_s: number
  wind_speed_m_s: number
  wind_from_deg: number
  ref_altitude_agl_m: number | null
  ref_vz_m_s: number
  ref_heading_deg: number
  ref_yaw_rate_deg_s: number
  ref_airspeed_m_s: number
  ref_vx_m_s: number
  ref_vy_m_s: number
  err_altitude_m: number | null
  err_cross_track_m: number | null
  err_heading_deg: number
  err_vz_m_s: number
  err_horizontal_velocity_m_s: number
  distance_to_landing_m: number
  distance_to_go_m: number
  active_waypoint: number | null
  landing_attempt: number
  cmd_surge_n: number
  cmd_heave_n: number
  cmd_yaw_moment_nm: number
  thrust_left_n: number
  thrust_right_n: number
  thrust_heave_n: number
  rudder_deg: number
  u_surge: number
  u_heave: number
  u_yaw: number
  sat_surge: boolean
  sat_heave: boolean
  sat_yaw: boolean
  buoyancy_n: number
  weight_n: number
  aero_x_n: number
  aero_y_n: number
  aero_z_n: number
  thrust_z_n: number
  ground_force_n: number
  mooring_load_n: number
  contact: boolean
  moored: boolean
  power_w: number
  energy_wh: number
  battery_soc: number
  air_density_kg_m3: number
  gain_vz_kp: number
  gain_yaw_rate_kp: number
  gain_airspeed_kp: number
  gain_position: number
}

export interface StateSnapshot {
  x_m: number
  y_m: number
  altitude_agl_m: number
  ground_speed_m_s: number
  vertical_speed_m_s: number
  airspeed_m_s: number
  heading_deg: number
  pitch_deg: number
  roll_deg: number
  distance_to_landing_m: number
}

export interface PhaseTransition {
  time_s: number
  from: FlightPhase
  to: FlightPhase
  reason: string
  state: StateSnapshot
}

export type SafetySeverity = 'info' | 'warning' | 'critical' | 'fatal'
export type SafetyResponse = 'log_only' | 'go_around' | 'proceed_to_landing' | 'terminate'

export interface SafetyEvent {
  time_s: number
  severity: SafetySeverity
  constraint: string
  message: string
  phase: FlightPhase
  value: number
  limit: number
  response: SafetyResponse
  state: StateSnapshot
  cleared_at_s: number | null
}

export interface SessionStatus {
  id: string
  name: string
  controller: ControllerKind
  state: RunState
  speed: number
  created_at_ms: number
  progress: number
  time_s: number
  duration_s: number
  steps: number
  phase: FlightPhase
  guidance_mode: string
  finished: FinishReason | null
  finish_message: string | null
  telemetry_count: number
  transitions: number
  safety_events: number
  wall_time_ms: number
  latest: TelemetrySample | null
  last_transition: PhaseTransition | null
  recent_events: SafetyEvent[]
}

export interface TelemetryResponse {
  total: number
  next_index: number
  samples: TelemetrySample[]
}

// ---------------------------------------------------------------- results

export interface Metric {
  key: string
  label: string
  group: string
  unit: string
  value: number | null
  definition: string
}

export interface MetricDefinition {
  key: string
  label: string
  group: string
  unit: string
  definition: string
}

export type LandingClass = 'successful' | 'unsuccessful' | 'not_attempted'

export interface TouchdownRecord {
  time_s: number
  phase: FlightPhase
  x_m: number
  y_m: number
  distance_to_target_m: number
  along_axis_m: number
  cross_axis_m: number
  horizontal_speed_m_s: number
  vertical_speed_m_s: number
  inside_landing_area: boolean
}

export interface Criterion {
  name: string
  passed: boolean
  detail: string
}

export interface LandingAssessment {
  classification: LandingClass
  reached_target_area: boolean
  ground_contact: boolean
  touchdown_within_limits: boolean
  sequence_completed: boolean
  criteria: Criterion[]
  touchdown: TouchdownRecord | null
  explanation: string
}

export interface PhaseInterval {
  phase: FlightPhase
  start_s: number
  end_s: number
  duration_s: number
}

export interface PreflightCheck {
  name: string
  passed: boolean
  detail: string
}

export interface DisturbanceActivation {
  time_s: number
  description: string
}

export interface NumericalInfo {
  integrator: string
  dt_s: number
  control_rate_hz: number
  telemetry_rate_hz: number
  steps: number
  simulated_time_s: number
  wall_time_ms: number
  real_time_factor: number
}

export interface ResultsReport {
  name: string
  controller: ControllerKind
  controller_label: string
  config_hash: string
  turbulence_seed: number | null
  sensor_noise_seed: number | null
  finish_reason: FinishReason | null
  finish_message: string
  final_phase: FlightPhase
  landing: LandingAssessment
  metrics: Metric[]
  phase_timeline: PhaseInterval[]
  transitions: PhaseTransition[]
  safety_events: SafetyEvent[]
  disturbances: DisturbanceActivation[]
  touchdowns: TouchdownRecord[]
  preflight: PreflightCheck[]
  flight_path_warnings: ValidationIssue[]
  numerical: NumericalInfo
  derived: DerivedProperties
  partial: boolean
  disclaimer: string
}

export interface ComparisonRun {
  label: string
  results: ResultsReport
  telemetry: TelemetrySample[]
}

export interface ComparisonReport {
  runs: ComparisonRun[]
  identical_conditions: boolean
  differences: string[]
  disclaimer: string
}

export interface ExperimentRun {
  label: string
  config: SimulationConfig
}

export interface Experiment {
  id: string
  title: string
  objective: string
  description: string
  focus_metrics: string[]
  runs: ExperimentRun[]
}

export interface Scenario {
  id: string
  name: string
  description: string
  environment: EnvironmentConfig
}

export interface Health {
  status: string
  version: string
  uptime_s: number
  sessions: number
}

export interface ApiErrorBody {
  error: {
    code: string
    message: string
    details?: ValidationIssue[]
  }
}
