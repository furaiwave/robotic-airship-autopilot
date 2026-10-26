// Ukrainian names for identifiers returned by the Rust engine (metric keys,
// scenario and experiment ids, template ids, constraint codes, ...). Free-text
// diagnostics produced by the engine (transition reasons, event messages,
// validation messages) are shown as delivered.
import { CONTROLLER_LABEL } from '@/lib/format'
import type { Experiment, Metric, PathTemplate, SafetyResponse, Scenario } from '@/types/api'

/** Disclaimer shown with every result (the engine sends the English original). */
export const DISCLAIMER_UK =
  'Змодельовані дані спрощеної, неверифікованої моделі дирижабля для академічних досліджень. Вони не підтверджують реальні льотні характеристики, льотну придатність чи готовність до польотів.'

const UNITS: Record<string, string> = {
  m: 'м',
  'm/s': 'м/с',
  deg: '°',
  s: 'с',
  ms: 'мс',
  Wh: 'Вт·год',
  'N s': 'Н·с',
  count: 'шт.',
  '-': '–',
}

export function unitUk(u: string): string {
  return UNITS[u] ?? u
}

const METRICS: Record<string, { label: string; group: string; definition: string }> = {
  altitude_rmse_m: { label: 'СКВ похибки висоти', group: 'Відстеження', definition: 'СКВ e_h = h_ref − h на зразках у польоті з активним завданням висоти' },
  altitude_max_error_m: { label: 'Макс. похибка висоти', group: 'Відстеження', definition: 'MAX |h_ref − h| на тому ж інтервалі' },
  cross_track_rmse_m: { label: 'СКВ бокового відхилення', group: 'Відстеження', definition: 'СКВ знакового бокового відхилення від активного відрізка маршруту або лінії заходу (набір, маршрут, лінія заходу, зниження)' },
  cross_track_max_m: { label: 'Макс. бокове відхилення', group: 'Відстеження', definition: 'MAX |бокового відхилення| на тому ж інтервалі' },
  horizontal_velocity_rmse_m_s: { label: 'СКВ похибки горизонтальної швидкості', group: 'Відстеження', definition: 'СКВ |v_ref − v_ground| (2-D шляхова швидкість) на зразках у польоті' },
  vertical_velocity_rmse_m_s: { label: 'СКВ похибки вертикальної швидкості', group: 'Відстеження', definition: 'СКВ vz_ref − vz на зразках у польоті' },
  heading_rmse_deg: { label: 'СКВ похибки курсу', group: 'Відстеження', definition: 'СКВ wrap(ψ_ref − ψ) на зразках у польоті' },
  max_pitch_deg: { label: 'Макс. відхилення тангажу', group: 'Положення', definition: 'MAX |θ| на зразках у польоті' },
  max_roll_deg: { label: 'Макс. відхилення крену', group: 'Положення', definition: 'MAX |φ| на зразках у польоті' },
  time_to_target_altitude_s: { label: 'Час досягнення цільової висоти', group: 'Час', definition: 't(перше |h − h_target| ≤ допуск) − t(відшвартування)' },
  takeoff_duration_s: { label: 'Тривалість зльоту', group: 'Час', definition: 't(вхід у Набір висоти) − t(вхід у Зліт): від відшвартування до висоти початкового набору' },
  approach_duration_s: { label: 'Тривалість заходу', group: 'Час', definition: 't(торкання) − t(останній вхід у Захід на посадку)' },
  time_to_touchdown_s: { label: 'Час до торкання', group: 'Час', definition: 't(перший контакт оціненого торкання) − 0' },
  mission_duration_s: { label: 'Тривалість місії', group: 'Час', definition: 'Модельний час наприкінці прогону' },
  landing_point_error_m: { label: 'Похибка точки посадки', group: 'Посадка', definition: 'Горизонтальна відстань між точкою торкання та цільовою точкою посадки' },
  touchdown_horizontal_speed_m_s: { label: 'Горизонтальна швидкість торкання', group: 'Посадка', definition: '|v_ground| у момент початку контакту' },
  touchdown_vertical_speed_m_s: { label: 'Вертикальна швидкість торкання', group: 'Посадка', definition: '−vz у момент початку контакту (додатне = зниження)' },
  max_surge_effort: { label: 'Макс. використання поздовжньої тяги', group: 'Керування', definition: 'MAX |F_x| / F_x,max (нормована реалізована команда)' },
  max_heave_effort: { label: 'Макс. використання вертикальної тяги', group: 'Керування', definition: 'MAX |F_z| / F_z,max' },
  max_yaw_effort: { label: 'Макс. використання моменту рискання', group: 'Керування', definition: 'MAX |N| / N_max' },
  control_effort_index_s: { label: 'Індекс керувальних зусиль', group: 'Керування', definition: 'INT(u_surge² + u_heave² + u_yaw²) з нормованими командами' },
  saturation_time_s: { label: 'Тривалість насичення керування', group: 'Керування', definition: 'INT(1[будь-який канал у насиченні]) за прогін' },
  saturation_time_heave_s: { label: 'Тривалість насичення вертикальної тяги', group: 'Керування', definition: 'INT(1[вертикальний канал у насиченні])' },
  saturation_time_yaw_s: { label: 'Тривалість насичення рискання', group: 'Керування', definition: 'INT(1[канал рискання у насиченні])' },
  energy_wh: { label: 'Витрачена електроенергія', group: 'Енергія', definition: 'INT(P): потужність рушіїв за імпульсною теорією плюс бортове обладнання' },
  propulsive_impulse_n_s: { label: 'Імпульс тяги', group: 'Енергія', definition: 'INT(|T_L| + |T_R| + |T_h|)' },
  final_battery_soc: { label: 'Кінцевий заряд батареї', group: 'Енергія', definition: '1 − E_used / E_capacity наприкінці прогону' },
  landing_attempts: { label: 'Спроби посадки', group: 'Безпека', definition: 'Кількість входів у фазу Захід на посадку' },
  go_around_count: { label: 'Відходи на друге коло', group: 'Безпека', definition: 'Кількість переходів у фазу Відхід на друге коло' },
  bounce_count: { label: 'Відскоки', group: 'Безпека', definition: 'Втрата контакту після торкання' },
  warning_events: { label: 'Попередження', group: 'Безпека', definition: 'Події безпеки рівня «попередження»' },
  critical_events: { label: 'Критичні події', group: 'Безпека', definition: 'Події безпеки рівня «критична»' },
  fatal_events: { label: 'Фатальні події', group: 'Безпека', definition: 'Події безпеки рівня «фатальна»' },
  wall_time_ms: { label: 'Час обчислення', group: 'Обчислення', definition: 'Реальний (настінний) час кроків моделювання' },
  real_time_factor: { label: 'Коефіцієнт прискорення', group: 'Обчислення', definition: 'Модельний час / реальний час' },
}

/** Metric with Ukrainian label, group, definition and unit (falls back to the engine text). */
export function metricUk(m: Metric): Metric {
  const t = METRICS[m.key]
  return { ...m, label: t?.label ?? m.label, group: t?.group ?? m.group, definition: t?.definition ?? m.definition, unit: unitUk(m.unit) }
}

export function metricLabelUk(key: string, fallback: string): string {
  return METRICS[key]?.label ?? fallback
}

const SCENARIOS: Record<string, { name: string; description: string }> = {
  calm: { name: '1. Штиль', description: 'Без вітру, турбулентності та шуму датчиків.' },
  constant_wind: {
    name: '2. Сталий горизонтальний вітер',
    description: 'Західний вітер 4 м/с на висоті 10 м зі степеневим профілем (α = 0,14); зустрічний вітер на кінцевому заході.',
  },
  changing_wind: {
    name: '3. Зміна вітру під час заходу',
    description: 'Західний вітер 2 м/с; коли апарат уперше наближається на 300 м до точки посадки, вітер за 30 с змінюється до 5,5 м/с з 210° (бокова складова).',
  },
  gusts: {
    name: '4. Помірні пориви',
    description: '3 м/с з 250°, турбулентність Гаусса–Маркова (σ 0,7 м/с, T_c 4 с, seed 42), порив 5 м/с при t = 90 с і порив 3 м/с за 120 м до точки посадки.',
  },
  limited_ground: {
    name: '5. Обмежена посадкова площа',
    description: 'Площадка лише 10 м до і 6 м після точки посадки (±6 м завширшки) з попутним вітром 2,5 м/с на кінцевому заході.',
  },
}

export function scenarioUk(s: Scenario): Scenario {
  const t = SCENARIOS[s.id]
  return t ? { ...s, ...t } : s
}

const EXPERIMENTS: Record<string, { title: string; objective: string; description: string }> = {
  A: {
    title: 'Зліт у штиль',
    objective: 'Оцінити послідовність зльоту, відстеження висоти, швидкість набору та час досягнення цільової висоти.',
    description: 'Еталонна місія без вітру.',
  },
  B: {
    title: 'Зліт при горизонтальному вітрі',
    objective: 'Оцінити здатність виконувати заданий набір висоти при сталому вітровому збуренні.',
    description: 'Еталонна місія з північним вітром 5 м/с (на 10 м, профіль 0,14).',
  },
  C: {
    title: 'Відстеження висоти',
    objective: 'Оцінити точність відстеження заданого профілю висоти.',
    description: 'Старт у повітрі на 40 м; горизонтальні відрізки зі ступенями висоти 40–60–35–45 м при 5 м/с.',
  },
  D: {
    title: 'Захід і посадка в штиль',
    objective: 'Оцінити похибку точки посадки, швидкості торкання та завершеність посадкової послідовності.',
    description: 'Старт у повітрі за 450 м на продовженні посадкового курсу, 30 м убік від осі.',
  },
  E: {
    title: 'Посадка при зміні вітру',
    objective: 'Оцінити реакцію автопілота на зміну вітру під час заходу.',
    description: 'Старт на заході; за 180 м до точки посадки вітер за 20 с змінюється з 1,5 м/с (270°) до 5 м/с з 200°; слабка турбулентність (seed 7).',
  },
  F: {
    title: 'Порівняння регуляторів',
    objective: 'Порівняти каскадний ПІД і регулятор з плануванням коефіцієнтів за однакових початкових умов, параметрів і зерен збурень.',
    description: 'Еталонна місія у сценарії 4 (пориви, зерно турбулентності 42).',
  },
  G: {
    title: 'Відхід на друге коло / перервана посадка',
    objective: 'Ввести порушення обмежень заходу й перевірити, що автомат станів його виявляє та виконує відхід на друге коло.',
    description:
      'Старт на заході; боковий порив 9 м/с (40 с, 1−cos) починається за 110 м до точки посадки. Обмеження вітру для посадки — 7 м/с, тож захід стає нездійсненним; очікується відхід на друге коло і повторний захід.',
  },
}

export function experimentUk(e: Experiment): Experiment {
  const t = EXPERIMENTS[e.id]
  return t ? { ...e, ...t } : e
}

export const TEMPLATE_UK: Record<PathTemplate, { name: string; description: string }> = {
  vertical_climb: { name: 'Вертикальний набір', description: 'Крутий набір висоти над місцем зльоту до точки зависання, далі прямо на захід' },
  climb_and_level: { name: 'Набір і горизонтальний політ', description: 'Набір уздовж курсу зльоту з подальшим горизонтальним відрізком' },
  straight_approach: { name: 'Прямий захід', description: 'Прямий відрізок до точки на продовженні посадкового курсу' },
  curved_approach: { name: 'Криволінійний захід', description: 'Дуга шляхових точок з виходом на посадковий курс' },
  descent_to_landing: { name: 'Зниження до посадки', description: 'Відрізки зі зменшенням висоти до воріт глісади' },
  go_around_climb_out: { name: 'Відхід і набір', description: 'Низький прохід над точкою посадки, набір і коло до заходу' },
}

const PREFLIGHT: Record<string, string> = {
  sensors: 'Датчики',
  'vehicle configuration': 'Конфігурація апарата',
  'take-off lift margin': 'Запас підйомної сили для зльоту',
  'landing descent authority': 'Запас тяги для зниження при посадці',
  wind: 'Вітер',
  battery: 'Батарея',
  'flight path': 'Маршрут польоту',
}

export function preflightUk(name: string): string {
  return PREFLIGHT[name] ?? name
}

const CRITERIA: Record<string, string> = {
  'Target area reached': 'Зону цілі досягнуто',
  'Ground contact': 'Контакт із землею',
  'Touchdown within limits': 'Торкання в межах обмежень',
  'Sequence completed safely': 'Послідовність завершено безпечно',
}

export function criterionUk(name: string): string {
  return CRITERIA[name] ?? name
}

const RESPONSES: Record<SafetyResponse, string> = {
  log_only: 'лише запис',
  go_around: 'відхід на друге коло',
  proceed_to_landing: 'перехід до посадки',
  terminate: 'зупинка моделювання',
}

export function responseUk(r: SafetyResponse): string {
  return RESPONSES[r] ?? r
}

const FINISH: Record<string, string> = {
  landed: 'посадка',
  aborted: 'перервано',
  timeout: 'вичерпано час',
  fatal: 'фатальна помилка',
  stopped: 'зупинено',
}

export function finishUk(r: string | null | undefined): string {
  return r ? (FINISH[r] ?? r) : 'триває'
}

const RUN_STATE: Record<string, string> = {
  created: 'створено',
  running: 'виконується',
  paused: 'пауза',
  finished: 'завершено',
}

export function runStateUk(s: string): string {
  return RUN_STATE[s] ?? s
}

const GUIDANCE_MODES: Record<string, string> = {
  preflight: 'передпольотні перевірки',
  moored: 'пришвартовано',
  'moored: heave thrust ramp': 'пришвартовано: наростання вертикальної тяги',
  'moored: heave thrust ramp, aligning with the wind': 'пришвартовано: наростання тяги, розворот проти вітру',
  'vertical take-off: position hold over the site': 'вертикальний зліт: утримання над майданчиком',
  'climb: path following': 'набір: рух за маршрутом',
  'climb: position hold over the site': 'набір: утримання над майданчиком',
  'en-route: LOS path following': 'маршрут: LOS-наведення',
  'en-route: path complete': 'маршрут: завершено',
  'approach: final-approach line': 'захід: лінія кінцевого заходу',
  'approach: direct to the join point': 'захід: прямо до точки входу',
  'descent: glide path': 'зниження: глісада',
  'descent: converge on the landing point': 'зниження: вихід на точку посадки',
  'flare: final vertical descent': 'вирівнювання: кінцеве вертикальне зниження',
  'flare: descent paused, ground speed too high': 'вирівнювання: зниження призупинено, завелика шляхова швидкість',
  'flare: holding height until over the point': 'вирівнювання: утримання висоти до виходу на точку',
  'on ground: hold-down thrust': 'на землі: притискна тяга',
  'go-around: climb straight ahead': 'відхід на друге коло: набір прямо',
  'aborted: on the ground': 'перервано: на землі',
  'aborted: holding position': 'перервано: утримання положення',
}

export function guidanceModeUk(mode: string): string {
  return GUIDANCE_MODES[mode] ?? mode
}

/** Translate the controller labels the engine uses for experiment runs. */
export function runLabelUk(label: string): string {
  if (label === 'Cascaded PID (A)') return CONTROLLER_LABEL.cascaded_pid
  if (label === 'Gain-scheduled PID (B)') return CONTROLLER_LABEL.gain_scheduled
  return label
}
