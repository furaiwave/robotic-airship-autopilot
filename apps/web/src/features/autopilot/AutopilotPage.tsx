import { useCallback } from 'react'

import { PageHeader } from '@/components/common/Display'
import { SectionForm } from '@/components/common/SectionForm'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/overlays'
import { Badge, Input, Label, Switch } from '@/components/ui/primitives'
import { Table, TBody, Td, Th, THead, Tr } from '@/components/ui/table'
import { AUTOPILOT_GROUPS, autopilotSchema, SIMULATION_FIELDS, simulationSchema } from '@/lib/schemas'
import { cn } from '@/lib/utils'
import { issuesFor, useConfigStore } from '@/stores/configStore'
import type { AutopilotConfig, ControllerKind, PhaseClass, SimulationSettings } from '@/types/api'

const LOOPS: { key: 'altitude' | 'vertical_speed' | 'heading' | 'yaw_rate' | 'airspeed'; label: string; units: string }[] = [
  { key: 'altitude', label: 'Висота → завдання верт. швидкості', units: '1/с, 1/с², с' },
  { key: 'vertical_speed', label: 'Вертикальна швидкість → вертикальна сила', units: 'Н·с/м, Н/м, Н·с²/м' },
  { key: 'heading', label: 'Курс → завдання кутової швидкості', units: '1/с, 1/с², с' },
  { key: 'yaw_rate', label: 'Кутова швидкість рискання → момент', units: 'Н·м·с, Н·м, Н·м·с²' },
  { key: 'airspeed', label: 'Повітряна швидкість → поздовжня сила', units: 'Н·с/м, Н/м, Н·с²/м' },
]

const PHASE_CLASSES: { key: PhaseClass; label: string }[] = [
  { key: 'ground', label: 'На землі' },
  { key: 'takeoff', label: 'Зліт' },
  { key: 'climb', label: 'Набір висоти' },
  { key: 'cruise', label: 'Маршрут' },
  { key: 'approach', label: 'Захід' },
  { key: 'landing', label: 'Посадка' },
]

const STATE_MACHINE: { phase: string; loops: string; exit: string; failure: string }[] = [
  {
    phase: 'Передпольотні перевірки',
    loops: '—',
    exit: 'усі перевірки пройдено (датчики, фізика апарата, запас підйомної сили, запас тяги для зниження, вітер, батарея, маршрут)',
    failure: 'будь-яка перевірка не пройдена → Перервано',
  },
  {
    phase: 'Підготовка до зльоту',
    loops: 'розімкнене наростання вертикальної тяги; розворот проти вітру біля щогли',
    exit: 'навантаження на швартування ≥ порогу відшвартування, вітер ≤ межі, корпус ≤ 20° від напрямку вітру',
    failure: 'тайм-аут → Перервано (з указанням причини)',
  },
  { phase: 'Зліт', loops: 'вертикальна швидкість, утримання положення, курс', exit: 'висота шасі ≥ висоти відриву та vz > 0,1 м/с', failure: 'тайм-аут → Перервано' },
  { phase: 'Початковий набір', loops: 'вертикальна швидкість, утримання положення, курс', exit: 'висота шасі ≥ висоти початкового набору', failure: '—' },
  {
    phase: 'Набір висоти',
    loops: 'висота (завдання з обмеженою швидкістю), LOS-наведення, курс, повітряна швидкість',
    exit: '|h − h_ціль| ≤ допуску та |vz| ≤ допуску',
    failure: 'незапланований контакт із землею → Торкання',
  },
  { phase: 'Політ за маршрутом', loops: 'висота, LOS-наведення, курс, повітряна швидкість', exit: 'усі шляхові точки захоплено', failure: 'критичний заряд батареї → Захід на посадку' },
  {
    phase: 'Захід на посадку',
    loops: 'висота, точка входу / лінія кінцевого заходу, курс, повітряна швидкість',
    exit: 'у воротах: у коридорі за боковим відхиленням, висотою, шляховим кутом, швидкістю та вітром',
    failure: 'не стабілізовано у воротах → Відхід на друге коло',
  },
  {
    phase: 'Зниження по глісаді',
    loops: 'висота глісади + пряма компенсація vz, лінія, зменшення швидкості',
    exit: 'низько, у радіусі вирівнювання, повільно',
    failure: 'бокове відхилення, відхилення від глісади, вітер, проліт точки (вище ВПР) → Відхід на друге коло',
  },
  {
    phase: 'Вирівнювання',
    loops: 'вертикальна швидкість (зниження), утримання положення',
    exit: 'навантаження на шасі (модель контакту)',
    failure: 'зміщення > 2× допуску або вітер (вище ВПР) → Відхід на друге коло',
  },
  { phase: 'Торкання', loops: 'розімкнена притискна тяга', exit: 'на землі, повільно, |vz| ≤ 0,2 м/с', failure: 'відскок → Вирівнювання' },
  { phase: 'Стабілізація після посадки', loops: 'розімкнена притискна тяга', exit: 'стабільно протягом часу стабілізації → швартування', failure: 'відрив → Вирівнювання' },
  {
    phase: 'Відхід на друге коло',
    loops: 'висота до висоти заходу, прямо за курсом, курс, повітряна швидкість',
    exit: 'досягнуто висоти заходу → Захід на посадку',
    failure: 'вичерпано спроби → Перервано',
  },
]

export function AutopilotPage() {
  const config = useConfigStore((s) => s.config)
  const update = useConfigStore((s) => s.update)
  const validation = useConfigStore((s) => s.validation)
  const onAutopilot = useCallback((a: AutopilotConfig) => update('autopilot', a), [update])
  const onSim = useCallback((s: SimulationSettings) => update('simulation', s), [update])
  if (!config) return null
  const ctl = config.controller

  const gain = (path: string, value: number, label: string) => (
    <Input
      type="number"
      step="any"
      aria-label={label}
      className="w-24"
      value={value}
      onChange={(e) => {
        const v = Number(e.target.value)
        if (Number.isFinite(v) && v >= 0) update(path, v)
      }}
    />
  )

  return (
    <div>
      <PageHeader
        title="Конфігурація автопілота"
        description="Обидва регулятори мають однакову каскадну структуру контурів, пряму компенсацію, наведення за трикутником швидкостей і розподіл керування; вони відрізняються лише способом отримання коефіцієнтів контурів."
      />
      <Tabs defaultValue="controller">
        <TabsList>
          <TabsTrigger value="controller">Регулятор</TabsTrigger>
          <TabsTrigger value="thresholds">Фази та пороги</TabsTrigger>
          <TabsTrigger value="statemachine">Автомат станів</TabsTrigger>
          <TabsTrigger value="numerics">Чисельні параметри</TabsTrigger>
        </TabsList>

        <TabsContent value="controller" className="space-y-3">
          <div className="grid gap-3 lg:grid-cols-2">
            {(['cascaded_pid', 'gain_scheduled'] as ControllerKind[]).map((k) => (
              <button
                key={k}
                onClick={() => update('controller.kind', k)}
                aria-pressed={ctl.kind === k}
                className={cn('rounded-lg border bg-surface p-3 text-left transition-colors', ctl.kind === k ? 'border-primary ring-1 ring-primary' : 'border-border hover:border-border-strong')}
              >
                <div className="flex items-center justify-between">
                  <span className="text-sm font-semibold">{k === 'cascaded_pid' ? 'Регулятор A — каскадний ПІД' : 'Регулятор B — ПІД з плануванням коефіцієнтів'}</span>
                  {ctl.kind === k ? <Badge variant="info">вибрано</Badge> : null}
                </div>
                <p className="mt-1 text-xs text-muted-foreground">
                  {k === 'cascaded_pid'
                    ? 'Сталі коефіцієнти для всіх фаз і швидкостей. Типові значення отримано розміщенням полюсів в одній номінальній крейсерській точці (повітряна швидкість 4 м/с, крейсерські смуги пропускання) і далі не змінюються.'
                    : 'Коефіцієнти перераховуються на кожному кроці керування: Kp = max(2ζωM − d(σ), f·2ζωM), Ki = ω²M, де d(σ) — лінеаризоване аеродинамічне демпфування за виміряними повітряною швидкістю / вертикальною швидкістю / кутовою швидкістю рискання, ω береться з таблиці фаз, а коефіцієнт за положенням залежить від відстані до точки посадки.'}
                </p>
              </button>
            ))}
          </div>

          <div className="grid gap-3 xl:grid-cols-2">
            <Card>
              <CardHeader>
                <div>
                  <CardTitle>Коефіцієнти регулятора A</CardTitle>
                  <CardDescription>ПІД з антинасиченням інтегратора (умовне інтегрування), фільтрованою похідною за вимірюванням, обмеженням виходу та швидкості його зміни.</CardDescription>
                </div>
              </CardHeader>
              <CardContent className="p-0">
                <Table>
                  <THead>
                    <Tr>
                      <Th>Контур</Th>
                      <Th>Kp</Th>
                      <Th>Ki</Th>
                      <Th>Kd</Th>
                    </Tr>
                  </THead>
                  <TBody>
                    {LOOPS.map((l) => (
                      <Tr key={l.key}>
                        <Td>
                          <div>{l.label}</div>
                          <div className="text-[10px] text-subtle-foreground">{l.units}</div>
                        </Td>
                        {(['kp', 'ki', 'kd'] as const).map((g) => (
                          <Td key={g}>{gain(`controller.cascaded_pid.${l.key}.${g}`, ctl.cascaded_pid[l.key][g], `${l.label} ${g}`)}</Td>
                        ))}
                      </Tr>
                    ))}
                    <Tr>
                      <Td>Положення → шляхова швидкість [1/с]</Td>
                      <Td>{gain('controller.cascaded_pid.position_gain_1_s', ctl.cascaded_pid.position_gain_1_s, 'Коефіцієнт за положенням')}</Td>
                      <Td />
                      <Td />
                    </Tr>
                  </TBody>
                </Table>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <div>
                  <CardTitle>Планування коефіцієнтів регулятора B</CardTitle>
                  <CardDescription>Смуга пропускання замкненого контуру ω [рад/с] для кожного класу фаз; зовнішні контури використовують ω / n_зовн.</CardDescription>
                </div>
              </CardHeader>
              <CardContent className="space-y-3 p-0">
                <Table>
                  <THead>
                    <Tr>
                      <Th>Клас фаз</Th>
                      <Th>ω вертикаль</Th>
                      <Th>ω рискання</Th>
                      <Th>ω швидкість</Th>
                    </Tr>
                  </THead>
                  <TBody>
                    {PHASE_CLASSES.map((p) => (
                      <Tr key={p.key}>
                        <Td>{p.label}</Td>
                        {(['vertical_rad_s', 'yaw_rad_s', 'speed_rad_s'] as const).map((b) => (
                          <Td key={b}>{gain(`controller.gain_scheduled.bandwidths.${p.key}.${b}`, ctl.gain_scheduled.bandwidths[p.key][b], `${p.label} ${b}`)}</Td>
                        ))}
                      </Tr>
                    ))}
                  </TBody>
                </Table>
                <div className="grid grid-cols-2 gap-3 px-4 pb-4">
                  {(
                    [
                      ['damping_ratio', 'Коефіцієнт демпфування ζ'],
                      ['outer_loop_ratio', 'Співвідношення зовн. контуру n'],
                      ['min_proportional_fraction', 'Мін. частка Kp f'],
                      ['position_gain_far_1_s', 'Коеф. за положенням, далеко [1/с]'],
                      ['position_gain_near_1_s', 'Коеф. за положенням, близько [1/с]'],
                      ['position_schedule_distance_m', 'Відстань планування [м]'],
                    ] as const
                  ).map(([k, label]) => (
                    <div key={k} className="space-y-1">
                      <Label>{label}</Label>
                      {gain(`controller.gain_scheduled.${k}`, ctl.gain_scheduled[k], label)}
                    </div>
                  ))}
                </div>
              </CardContent>
            </Card>
          </div>
          <Card>
            <CardHeader>
              <CardTitle>Спільні налаштування</CardTitle>
            </CardHeader>
            <CardContent className="grid gap-3 sm:grid-cols-3">
              {(
                [
                  ['derivative_filter_tau_s', 'Стала фільтра похідної τ [с]'],
                  ['heave_rate_limit_n_s', 'Обмеження швидкості верт. сили [Н/с]'],
                  ['surge_rate_limit_n_s', 'Обмеження швидкості поздовжньої сили [Н/с]'],
                  ['yaw_moment_rate_limit_nm_s', 'Обмеження швидкості моменту рискання [Н·м/с]'],
                  ['max_reverse_airspeed_m_s', 'Макс. швидкість заднього ходу [м/с]'],
                ] as const
              ).map(([k, label]) => (
                <div key={k} className="space-y-1">
                  <Label>{label}</Label>
                  {gain(`controller.common.${k}`, ctl.common[k], label)}
                </div>
              ))}
              <label className="flex items-center gap-2 self-end text-xs text-muted-foreground">
                <Switch checked={ctl.common.drag_feedforward} onCheckedChange={(v) => update('controller.common.drag_feedforward', v)} aria-label="Пряма компенсація опору" />
                Пряма компенсація опору та статичної ваги
              </label>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="thresholds">
          <SectionForm value={config.autopilot} groups={AUTOPILOT_GROUPS} schema={autopilotSchema} onChange={onAutopilot} issuePrefix="autopilot" serverIssues={issuesFor(validation, 'autopilot')} columns={3} />
        </TabsContent>

        <TabsContent value="statemachine">
          <Card>
            <CardHeader>
              <div>
                <CardTitle>Явний автомат станів фаз польоту</CardTitle>
                <CardDescription>
                  Кожен перехід залежить від виміряного стану, цільових значень і обмежень безпеки; таймери лише обмежують обробку відмов. Кожен перехід записується з часом і виміряними
                  значеннями, що задовольнили умову.
                </CardDescription>
              </div>
            </CardHeader>
            <CardContent className="p-0">
              <Table>
                <THead>
                  <Tr>
                    <Th>Фаза</Th>
                    <Th>Активні контури</Th>
                    <Th>Штатна умова виходу</Th>
                    <Th>Обробка відмов / переривання</Th>
                  </Tr>
                </THead>
                <TBody>
                  {STATE_MACHINE.map((r) => (
                    <Tr key={r.phase}>
                      <Td className="font-medium whitespace-nowrap">{r.phase}</Td>
                      <Td className="text-muted-foreground">{r.loops}</Td>
                      <Td>{r.exit}</Td>
                      <Td className="text-muted-foreground">{r.failure}</Td>
                    </Tr>
                  ))}
                </TBody>
              </Table>
              <p className="px-4 py-2 text-[11px] text-subtle-foreground">ВПР — висота прийняття рішення; LOS — наведення за лінією візування.</p>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="numerics">
          <SectionForm
            value={config.simulation}
            groups={[
              {
                title: 'Інтегрування зі сталим кроком',
                description: 'Класичний метод Рунге–Кутти 4-го порядку з кроком dt; регулятор працює кожні 1/частоту керування секунд; швидкість відтворення ніколи не змінює dt.',
                fields: SIMULATION_FIELDS,
              },
            ]}
            schema={simulationSchema}
            onChange={onSim}
            issuePrefix="simulation"
            serverIssues={issuesFor(validation, 'simulation')}
          />
        </TabsContent>
      </Tabs>
    </div>
  )
}
