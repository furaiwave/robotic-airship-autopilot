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
  const unit = f.unit ? ` ${f.unit}` : ''
  let s = z
    .number({ error: `${f.label}: потрібне число` })
    .min(f.min, `${f.label}: мінімум ${f.min}${unit}`)
    .max(f.max, `${f.label}: максимум ${f.max}${unit}`)
  if (f.integer) s = s.int(`${f.label}: потрібне ціле число`)
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
    title: 'Геометрія оболонки та маса',
    fields: [
      { name: 'length_m', label: 'Довжина оболонки L', unit: 'м', min: 1, max: 250, step: 0.1 },
      { name: 'max_diameter_m', label: 'Макс. діаметр D', unit: 'м', min: 0.3, max: 60, step: 0.1 },
      { name: 'hull_volume_m3', label: 'Об’єм оболонки', unit: 'м³', min: 0.01, max: 1e6, step: 0.1, nullable: true, help: 'Порожнє поле = оцінка для витягнутого еліпсоїда π·L·D²/6' },
      { name: 'gondola_volume_m3', label: 'Об’єм гондоли / стабілізаторів', unit: 'м³', min: 0, max: 1e4, step: 0.05, help: 'Додатковий витіснений об’єм поза оболонкою' },
      { name: 'total_mass_kg', label: 'Повна маса m', unit: 'кг', min: 0.1, max: 1e6, step: 0.1, help: 'Конструкція + несучий газ + корисне навантаження + батарея' },
      { name: 'cg_below_cv_m', label: 'ЦМ нижче ЦО (BG)', unit: 'м', min: 0.05, max: 50, step: 0.05, help: 'Плече маятника; визначає відновлювальну жорсткість за тангажем і креном B·BG' },
      { name: 'gear_height_m', label: 'Шасі нижче ЦО', unit: 'м', min: 0.1, max: 60, step: 0.05 },
    ],
  },
  {
    title: 'Аеродинаміка',
    description: 'Характерні площі обчислюються з L і D; коефіцієнти є модельними оцінками.',
    fields: [
      { name: 'aero.cd_axial', label: 'Поздовжній опір C_Dx', min: 0.01, max: 1.5, step: 0.005, kind: 'estimate', help: 'Віднесено до лобової площі πD²/4' },
      { name: 'aero.cd_lateral', label: 'Бічний опір C_Dy', min: 0.1, max: 2, step: 0.05, kind: 'estimate', help: 'Віднесено до бічної площі πLD/4' },
      { name: 'aero.cd_vertical', label: 'Вертикальний опір C_Dz', min: 0.1, max: 2, step: 0.05, kind: 'estimate', help: 'Віднесено до площі в плані πLD/4' },
      { name: 'aero.fin_area_m2', label: 'Площа вертикального оперення S', unit: 'м²', min: 0, max: 1e4, step: 0.1 },
      { name: 'aero.fin_lift_slope_per_rad', label: 'Похідна підйомної сили C_Lα', unit: '1/рад', min: 0, max: 6.3, step: 0.1, kind: 'estimate' },
      { name: 'aero.fin_arm_m', label: 'Плече оперення l_f', unit: 'м', min: 0, max: 200, step: 0.1 },
      { name: 'aero.pitch_roll_damping_ratio', label: 'Демпфування тангажу/крену ζ', min: 0.05, max: 2, step: 0.05, kind: 'estimate' },
    ],
  },
  {
    title: 'Рушійна установка та енергія',
    fields: [
      { name: 'propulsion.main_max_thrust_n', label: 'Маршова тяга (кожен)', unit: 'Н', min: 0.1, max: 1e5, step: 1 },
      { name: 'propulsion.main_reverse_ratio', label: 'Частка реверсу маршових', min: 0, max: 1, step: 0.05 },
      { name: 'propulsion.main_lateral_arm_m', label: 'Бічне плече маршових b', unit: 'м', min: 0.05, max: 50, step: 0.05 },
      { name: 'propulsion.main_vertical_offset_m', label: 'Лінія тяги нижче ЦМ', unit: 'м', min: -20, max: 20, step: 0.05 },
      { name: 'propulsion.heave_max_thrust_n', label: 'Вертикальна тяга (вгору)', unit: 'Н', min: 0, max: 1e5, step: 1 },
      { name: 'propulsion.heave_reverse_ratio', label: 'Частка реверсу вертикального', min: 0, max: 1, step: 0.05 },
      { name: 'propulsion.response_time_s', label: 'Стала часу гвинта τ', unit: 'с', min: 0.02, max: 5, step: 0.01 },
      { name: 'propulsion.propeller_diameter_m', label: 'Діаметр гвинта', unit: 'м', min: 0.05, max: 10, step: 0.01 },
      { name: 'propulsion.efficiency', label: 'Загальний ККД η', min: 0.1, max: 1, step: 0.01, kind: 'estimate' },
      { name: 'propulsion.battery_capacity_wh', label: 'Ємність батареї', unit: 'Вт·год', min: 1, max: 1e7, step: 10 },
      { name: 'propulsion.avionics_power_w', label: 'Потужність бортового обладнання', unit: 'Вт', min: 0, max: 1e5, step: 1 },
    ],
  },
  {
    title: 'Кермо напрямку',
    fields: [
      { name: 'rudder.max_deflection_deg', label: 'Макс. відхилення', unit: '°', min: 0, max: 45, step: 1 },
      { name: 'rudder.max_rate_deg_s', label: 'Макс. швидкість', unit: '°/с', min: 1, max: 500, step: 1 },
      { name: 'rudder.effectiveness_per_rad', label: 'Ефективність C_Lδ', unit: '1/рад', min: 0, max: 6.3, step: 0.1, kind: 'estimate' },
    ],
  },
  {
    title: 'Експлуатаційні обмеження',
    description: 'Використовуються для формування завдань і монітором безпеки.',
    fields: [
      { name: 'limits.max_climb_rate_m_s', label: 'Макс. швидкість набору', unit: 'м/с', min: 0.05, max: 20, step: 0.1 },
      { name: 'limits.max_descent_rate_m_s', label: 'Макс. швидкість зниження', unit: 'м/с', min: 0.05, max: 20, step: 0.1 },
      { name: 'limits.max_airspeed_m_s', label: 'Макс. повітряна швидкість', unit: 'м/с', min: 0.5, max: 60, step: 0.5 },
      { name: 'limits.max_turn_rate_deg_s', label: 'Макс. кутова швидкість розвороту', unit: '°/с', min: 0.5, max: 90, step: 0.5 },
      { name: 'limits.max_pitch_deg', label: 'Макс. тангаж', unit: '°', min: 1, max: 60, step: 1 },
      { name: 'limits.max_roll_deg', label: 'Макс. крен', unit: '°', min: 1, max: 60, step: 1 },
    ],
  },
]

export const airshipSchema = buildSchema(AIRSHIP_GROUPS.flatMap((g) => g.fields)).superRefine((v, ctx) => {
  const a = v as { length_m: number; max_diameter_m: number; gear_height_m: number }
  if (a.length_m < a.max_diameter_m) ctx.addIssue({ code: 'custom', path: ['length_m'], message: 'Довжина не може бути меншою за діаметр' })
  if (a.gear_height_m < a.max_diameter_m / 2) ctx.addIssue({ code: 'custom', path: ['gear_height_m'], message: 'Точка контакту шасі має бути щонайменше на D/2 нижче ЦО' })
})

// ---------------------------------------------------------------- environment

export const ENVIRONMENT_GROUPS: FieldGroup[] = [
  {
    title: 'Рельєф, майданчики та атмосфера',
    fields: [
      { name: 'ground_elevation_m', label: 'Висота місцевості', unit: 'м н.р.м.', min: -400, max: 5000, step: 1 },
      { name: 'atmosphere.temperature_offset_k', label: 'Відхилення температури від МСА', unit: 'K', min: -40, max: 40, step: 1 },
      { name: 'atmosphere.constant_density_kg_m3', label: 'Стала густина', unit: 'кг/м³', min: 0.3, max: 1.5, step: 0.005, help: 'Використовується лише моделлю сталої густини' },
      { name: 'takeoff_site.x', label: 'Місце зльоту, схід', unit: 'м', min: -1e5, max: 1e5, step: 1 },
      { name: 'takeoff_site.y', label: 'Місце зльоту, північ', unit: 'м', min: -1e5, max: 1e5, step: 1 },
      { name: 'landing_site.x', label: 'Точка посадки, схід', unit: 'м', min: -1e5, max: 1e5, step: 1 },
      { name: 'landing_site.y', label: 'Точка посадки, північ', unit: 'м', min: -1e5, max: 1e5, step: 1 },
    ],
  },
  {
    title: 'Посадкова площадка (доступна ділянка землі)',
    fields: [
      { name: 'landing_area.heading_deg', label: 'Курс осі площадки', unit: '°', min: 0, max: 360, step: 1 },
      { name: 'landing_area.undershoot_m', label: 'Довжина до точки', unit: 'м', min: 1, max: 5000, step: 1 },
      { name: 'landing_area.overrun_m', label: 'Довжина після точки', unit: 'м', min: 1, max: 5000, step: 1 },
      { name: 'landing_area.half_width_m', label: 'Напівширина', unit: 'м', min: 1, max: 5000, step: 1 },
    ],
  },
  {
    title: 'Початковий стан',
    fields: [
      { name: 'initial_state.position.x', label: 'Положення, схід', unit: 'м', min: -1e5, max: 1e5, step: 1 },
      { name: 'initial_state.position.y', label: 'Положення, північ', unit: 'м', min: -1e5, max: 1e5, step: 1 },
      { name: 'initial_state.altitude_agl_m', label: 'Висота шасі над землею', unit: 'м', min: 0, max: 3000, step: 0.5 },
      { name: 'initial_state.velocity.x', label: 'Швидкість на схід', unit: 'м/с', min: -30, max: 30, step: 0.1 },
      { name: 'initial_state.velocity.y', label: 'Швидкість на північ', unit: 'м/с', min: -30, max: 30, step: 0.1 },
      { name: 'initial_state.velocity.z', label: 'Швидкість угору', unit: 'м/с', min: -30, max: 30, step: 0.1 },
      { name: 'initial_state.heading_deg', label: 'Курс', unit: '°', min: 0, max: 360, step: 1 },
      { name: 'initial_state.pitch_deg', label: 'Тангаж', unit: '°', min: -30, max: 30, step: 0.5 },
      { name: 'initial_state.roll_deg', label: 'Крен', unit: '°', min: -30, max: 30, step: 0.5 },
    ],
  },
  {
    title: 'Середній вітер',
    fields: [
      { name: 'wind.speed_m_s', label: 'Швидкість вітру (на опорній висоті)', unit: 'м/с', min: 0, max: 30, step: 0.1 },
      { name: 'wind.from_direction_deg', label: 'Напрямок, звідки дме', unit: '°', min: 0, max: 360, step: 1 },
      { name: 'wind.vertical_m_s', label: 'Вертикальний вітер', unit: 'м/с', min: -5, max: 5, step: 0.1 },
      { name: 'wind.shear_exponent', label: 'Показник профілю α', min: 0, max: 0.6, step: 0.01, kind: 'estimate' },
      { name: 'wind.shear_reference_height_m', label: 'Опорна висота профілю', unit: 'м', min: 1, max: 500, step: 1 },
    ],
  },
  {
    title: 'Турбулентність (Гаусса–Маркова)',
    fields: [
      { name: 'wind.turbulence.intensity_m_s', label: 'Інтенсивність σ', unit: 'м/с', min: 0, max: 10, step: 0.05 },
      { name: 'wind.turbulence.vertical_ratio', label: 'Відношення вертикальної до горизонтальної', min: 0, max: 2, step: 0.05 },
      { name: 'wind.turbulence.correlation_time_s', label: 'Час кореляції T_c', unit: 'с', min: 0.1, max: 600, step: 0.1 },
      { name: 'wind.turbulence.seed', label: 'Зерно генератора (seed)', min: 0, max: Number.MAX_SAFE_INTEGER, step: 1, integer: true },
    ],
  },
  {
    title: 'Шум датчиків (1σ)',
    fields: [
      { name: 'sensor_noise.position_m', label: 'Положення', unit: 'м', min: 0, max: 20, step: 0.05 },
      { name: 'sensor_noise.altitude_m', label: 'Висота', unit: 'м', min: 0, max: 20, step: 0.05 },
      { name: 'sensor_noise.velocity_m_s', label: 'Швидкість', unit: 'м/с', min: 0, max: 5, step: 0.01 },
      { name: 'sensor_noise.heading_deg', label: 'Курс', unit: '°', min: 0, max: 30, step: 0.1 },
      { name: 'sensor_noise.airspeed_m_s', label: 'Повітряні дані', unit: 'м/с', min: 0, max: 5, step: 0.01 },
      { name: 'sensor_noise.seed', label: 'Зерно генератора (seed)', min: 0, max: Number.MAX_SAFE_INTEGER, step: 1, integer: true },
    ],
  },
]

export const environmentSchema = buildSchema(ENVIRONMENT_GROUPS.flatMap((g) => g.fields))

// ---------------------------------------------------------------- autopilot

export const AUTOPILOT_GROUPS: FieldGroup[] = [
  {
    title: 'Цільові значення',
    fields: [
      { name: 'target_altitude_m', label: 'Цільова висота', unit: 'м над землею', min: 5, max: 2000, step: 1 },
      { name: 'cruise_speed_m_s', label: 'Цільова горизонтальна швидкість', unit: 'м/с', min: 0.5, max: 60, step: 0.1 },
      { name: 'takeoff_climb_rate_m_s', label: 'Цільова вертикальна швидкість (зліт)', unit: 'м/с', min: 0.1, max: 20, step: 0.05 },
      { name: 'takeoff_heading_deg', label: 'Курс зльоту (у штиль)', unit: '°', min: 0, max: 360, step: 1, nullable: true, help: 'Порожнє поле = зберігати початковий курс; при вітрі ніс повертається проти вітру' },
      { name: 'trim_mass_bias_kg', label: 'Похибка маси для балансування', unit: 'кг', min: -1000, max: 1000, step: 0.5, kind: 'estimate', help: 'Невідповідність бортової оцінки ваги (для досліджень робастності)' },
    ],
  },
  {
    title: 'Пороги зльоту',
    fields: [
      { name: 'takeoff.lift_margin_n', label: 'Запас підйомної сили', unit: 'Н', min: 0, max: 1e5, step: 1 },
      { name: 'takeoff.thrust_ramp_n_s', label: 'Швидкість наростання тяги', unit: 'Н/с', min: 0.5, max: 1e5, step: 0.5 },
      { name: 'takeoff.release_load_threshold_n', label: 'Навантаження для відшвартування', unit: 'Н', min: 0, max: 1e5, step: 0.5 },
      { name: 'takeoff.preparation_timeout_s', label: 'Тайм-аут підготовки', unit: 'с', min: 1, max: 600, step: 1 },
      { name: 'takeoff.clearance_altitude_m', label: 'Висота відриву', unit: 'м', min: 0.5, max: 100, step: 0.5 },
      { name: 'takeoff.initial_climb_altitude_m', label: 'Висота початкового набору', unit: 'м', min: 1, max: 500, step: 0.5 },
      { name: 'takeoff.takeoff_timeout_s', label: 'Тайм-аут зльоту', unit: 'с', min: 5, max: 600, step: 1 },
      { name: 'takeoff.max_wind_m_s', label: 'Макс. вітер для зльоту', unit: 'м/с', min: 0, max: 30, step: 0.5 },
      { name: 'takeoff.min_battery_soc', label: 'Мін. заряд батареї', min: 0, max: 1, step: 0.05 },
    ],
  },
  {
    title: 'Захід на посадку',
    fields: [
      { name: 'approach.course_deg', label: 'Посадковий курс', unit: '°', min: 0, max: 360, step: 1, nullable: true, help: 'Порожнє поле = курс осі посадкової площадки' },
      { name: 'approach.glide_angle_deg', label: 'Кут глісади', unit: '°', min: 2, max: 30, step: 0.5 },
      { name: 'approach.altitude_m', label: 'Висота заходу / відходу', unit: 'м', min: 5, max: 500, step: 1 },
      { name: 'approach.speed_m_s', label: 'Швидкість заходу', unit: 'м/с', min: 0.5, max: 60, step: 0.1 },
      { name: 'approach.pregate_extension_m', label: 'Відрізок перед воротами', unit: 'м', min: 10, max: 2000, step: 5 },
      { name: 'approach.corridor_half_width_m', label: 'Напівширина коридору', unit: 'м', min: 1, max: 500, step: 1 },
      { name: 'approach.corridor_altitude_tolerance_m', label: 'Допуск коридору за висотою', unit: 'м', min: 0.5, max: 100, step: 0.5 },
      { name: 'approach.max_course_error_deg', label: 'Макс. похибка шляхового кута', unit: '°', min: 5, max: 90, step: 1 },
      { name: 'approach.deceleration_distance_m', label: 'Відстань гальмування', unit: 'м', min: 5, max: 1000, step: 5 },
    ],
  },
  {
    title: 'Посадка та умови переривання',
    fields: [
      { name: 'landing.flare_height_m', label: 'Висота вирівнювання', unit: 'м', min: 0.5, max: 50, step: 0.1 },
      { name: 'landing.flare_entry_radius_m', label: 'Радіус входу у вирівнювання', unit: 'м', min: 0.5, max: 200, step: 0.5 },
      { name: 'landing.flare_entry_max_speed_m_s', label: 'Макс. швидкість входу у вирівнювання', unit: 'м/с', min: 0.1, max: 10, step: 0.1 },
      { name: 'landing.touchdown_sink_rate_m_s', label: 'Швидкість зниження при торканні', unit: 'м/с', min: 0.05, max: 20, step: 0.05 },
      { name: 'landing.landing_point_tolerance_m', label: 'Допуск точки посадки', unit: 'м', min: 0.5, max: 500, step: 0.5 },
      { name: 'landing.max_touchdown_vertical_speed_m_s', label: 'Межа вертикальної швидкості торкання', unit: 'м/с', min: 0.05, max: 10, step: 0.05 },
      { name: 'landing.max_touchdown_horizontal_speed_m_s', label: 'Межа горизонтальної швидкості торкання', unit: 'м/с', min: 0.05, max: 10, step: 0.05 },
      { name: 'landing.decision_height_m', label: 'Висота прийняття рішення', unit: 'м', min: 0, max: 50, step: 0.1 },
      { name: 'landing.hold_down_force_n', label: 'Притискна сила', unit: 'Н', min: 0, max: 1e5, step: 1 },
      { name: 'landing.settle_time_s', label: 'Час стабілізації', unit: 'с', min: 0, max: 120, step: 0.5 },
      { name: 'landing.max_attempts', label: 'Макс. кількість спроб посадки', min: 1, max: 10, step: 1, integer: true },
      { name: 'landing.max_wind_m_s', label: 'Макс. вітер для посадки', unit: 'м/с', min: 0, max: 30, step: 0.5 },
      { name: 'landing.max_cross_track_error_m', label: 'Макс. бокове відхилення', unit: 'м', min: 0.5, max: 500, step: 0.5 },
      { name: 'landing.max_altitude_error_m', label: 'Макс. відхилення від глісади', unit: 'м', min: 0.5, max: 500, step: 0.5 },
    ],
  },
  {
    title: 'Обмеження керувальних сигналів і безпека',
    fields: [
      { name: 'control_limits.surge', label: 'Частка поздовжньої тяги', min: 0.05, max: 1, step: 0.05 },
      { name: 'control_limits.heave', label: 'Частка вертикальної тяги', min: 0.05, max: 1, step: 0.05 },
      { name: 'control_limits.yaw', label: 'Частка моменту рискання', min: 0.05, max: 1, step: 0.05 },
      { name: 'safety.critical_factor', label: 'Коефіцієнт критичності', min: 1, max: 3, step: 0.05 },
      { name: 'safety.min_enroute_altitude_m', label: 'Мін. висота на маршруті', unit: 'м', min: 0, max: 500, step: 1 },
      { name: 'safety.saturation_warning_time_s', label: 'Час насичення до попередження', unit: 'с', min: 0.1, max: 600, step: 0.5 },
      { name: 'safety.battery_warning_soc', label: 'Заряд для попередження', min: 0, max: 1, step: 0.05 },
      { name: 'safety.battery_critical_soc', label: 'Критичний заряд', min: 0, max: 1, step: 0.05 },
    ],
  },
]

export const autopilotSchema = buildSchema(AUTOPILOT_GROUPS.flatMap((g) => g.fields)).superRefine((v, ctx) => {
  const a = v as { landing: { decision_height_m: number; flare_height_m: number; touchdown_sink_rate_m_s: number; max_touchdown_vertical_speed_m_s: number }; takeoff: { clearance_altitude_m: number; initial_climb_altitude_m: number } }
  if (a.landing.decision_height_m >= a.landing.flare_height_m)
    ctx.addIssue({ code: 'custom', path: ['landing', 'decision_height_m'], message: 'Висота прийняття рішення має бути нижчою за висоту вирівнювання' })
  if (a.landing.touchdown_sink_rate_m_s >= a.landing.max_touchdown_vertical_speed_m_s)
    ctx.addIssue({ code: 'custom', path: ['landing', 'touchdown_sink_rate_m_s'], message: 'Швидкість зниження має бути меншою за межу швидкості торкання' })
  if (a.takeoff.initial_climb_altitude_m <= a.takeoff.clearance_altitude_m)
    ctx.addIssue({ code: 'custom', path: ['takeoff', 'initial_climb_altitude_m'], message: 'Має бути вище за висоту відриву' })
})

// ---------------------------------------------------------------- simulation settings

export const SIMULATION_FIELDS: FieldSpec[] = [
  { name: 'duration_s', label: 'Тривалість', unit: 'с', min: 1, max: 7200, step: 10 },
  { name: 'dt_s', label: 'Крок інтегрування', unit: 'с', min: 0.001, max: 0.05, step: 0.001 },
  { name: 'control_rate_hz', label: 'Частота керування', unit: 'Гц', min: 1, max: 200, step: 1 },
  { name: 'telemetry_rate_hz', label: 'Частота телеметрії', unit: 'Гц', min: 0.5, max: 100, step: 0.5 },
]

export const simulationSchema = buildSchema(SIMULATION_FIELDS).superRefine((v, ctx) => {
  const s = v as { dt_s: number; control_rate_hz: number; telemetry_rate_hz: number }
  const multiple = (period: number) => {
    const n = period / s.dt_s
    return Math.abs(n - Math.round(n)) < 1e-6 && Math.round(n) >= 1
  }
  if (!multiple(1 / s.control_rate_hz)) ctx.addIssue({ code: 'custom', path: ['control_rate_hz'], message: 'Період керування має бути кратним dt' })
  if (!multiple(1 / s.telemetry_rate_hz)) ctx.addIssue({ code: 'custom', path: ['telemetry_rate_hz'], message: 'Період телеметрії має бути кратним dt' })
})

// ---------------------------------------------------------------- waypoints

export const waypointSchema = z.object({
  x_m: z.number().min(-1e5).max(1e5),
  y_m: z.number().min(-1e5).max(1e5),
  altitude_m: z.number().min(0, 'Висота має бути ≥ 0').max(2000),
  heading_deg: z.number().min(0).max(360).nullable(),
  speed_m_s: z.number().min(0, 'Швидкість має бути ≥ 0').max(60),
  tolerance_m: z.number().min(1, 'Допуск має бути ≥ 1 м').max(500),
})

export const flightPathSchema = z
  .object({ name: z.string().min(1), waypoints: z.array(waypointSchema).max(100) })
  .superRefine((p, ctx) => {
    p.waypoints.forEach((w, i) => {
      const prev = p.waypoints[i - 1]
      if (prev) {
        const d = Math.hypot(w.x_m - prev.x_m, w.y_m - prev.y_m)
        if (d < Math.max(w.tolerance_m, prev.tolerance_m))
          ctx.addIssue({ code: 'custom', path: ['waypoints', i], message: `ШТ${i + 1} лежить у радіусі захоплення ШТ${i}` })
      }
    })
  })

export type WaypointInput = z.infer<typeof waypointSchema>
