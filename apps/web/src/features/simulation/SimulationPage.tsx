import { Activity } from 'lucide-react'
import { type ReactNode, useMemo, useState } from 'react'

import { EmptyState, PageHeader, PhaseBadge, SeverityBadge } from '@/components/common/Display'
import { SideView, TopView } from '@/components/common/FlightViews'
import { TimeSeriesChart } from '@/components/common/TimeSeriesChart'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/overlays'
import { breakAngleWraps, fmt, PHASE_LABEL } from '@/lib/format'
import { guidanceModeUk } from '@/lib/i18n'
import { useConfigStore } from '@/stores/configStore'
import { useSimulationStore } from '@/stores/simulationStore'
import type { TelemetrySample } from '@/types/api'

import { SimulationControls } from './SimulationControls'

/** Down-sample for charts (the full buffer stays available for views/exports). */
export function chartData(samples: TelemetrySample[], max = 900): TelemetrySample[] {
  if (samples.length <= max) return samples
  const stride = Math.ceil(samples.length / max)
  const out = samples.filter((_, i) => i % stride === 0)
  const last = samples[samples.length - 1]
  if (last && out[out.length - 1] !== last) out.push(last)
  return out
}

export function SimulationPage() {
  const config = useConfigStore((s) => s.config)
  const validation = useConfigStore((s) => s.validation)
  const session = useSimulationStore((s) => s.session)
  const telemetry = useSimulationStore((s) => s.telemetry)
  const results = useSimulationStore((s) => s.results)
  const simConfig = useSimulationStore((s) => s.config)
  const [mode, setMode] = useState<'final' | 'east' | 'north'>('final')
  const view = simConfig ?? config
  const current = telemetry[telemetry.length - 1] ?? null
  const data = useMemo(() => chartData(telemetry), [telemetry])
  const headingData = useMemo(() => breakAngleWraps(data, ['heading_deg', 'ref_heading_deg']), [data])
  const markers = useMemo(() => (results?.transitions ?? []).map((t) => ({ x: t.time_s, label: t.to })), [results])
  if (!config || !view) return null

  return (
    <div className="space-y-3">
      <PageHeader
        title="Робоча зона моделювання"
        description="Rust-рушій інтегрує динаміку зі сталим кроком; ця сторінка відображає телеметрію, яку опитує з рушія з частотою 4 Гц незалежно від частоти фізичного моделювання."
      />
      <SimulationControls />
      {!session ? (
        <EmptyState title="Моделювання не завантажено" icon={<Activity />}>
          Завантажте поточну конфігурацію, щоб створити сеанс моделювання в Rust-рушії. Перед створенням сеансу конфігурація перевіряється на сервері.
        </EmptyState>
      ) : (
        <>
          <div className="grid gap-3 xl:grid-cols-[minmax(0,1.35fr)_minmax(0,1fr)]">
            <Card>
              <CardHeader>
                <CardTitle>Траєкторія (вигляд згори)</CardTitle>
                <div className="flex items-center gap-2">
                  <PhaseBadge phase={session.phase} />
                  <span className="max-w-72 truncate text-xs text-muted-foreground">{guidanceModeUk(session.guidance_mode)}</span>
                </div>
              </CardHeader>
              <CardContent>
                <TopView config={view} approach={validation?.approach} trajectory={telemetry} current={current} height={430} />
              </CardContent>
            </Card>
            <div className="space-y-3">
              <Card>
                <CardHeader>
                  <CardTitle>Вигляд збоку</CardTitle>
                  <Tabs value={mode} onValueChange={(v) => setMode(v as typeof mode)}>
                    <TabsList>
                      <TabsTrigger value="final">Посадковий курс</TabsTrigger>
                      <TabsTrigger value="east">Схід</TabsTrigger>
                      <TabsTrigger value="north">Північ</TabsTrigger>
                    </TabsList>
                  </Tabs>
                </CardHeader>
                <CardContent>
                  <SideView config={view} approach={validation?.approach} trajectory={telemetry} current={current} mode={mode} height={200} />
                  <p className="mt-1.5 text-[11px] text-muted-foreground">Синій: модельована висота шасі. Сірий пунктир: завдання висоти. Фіолетовий пунктир: глісада.</p>
                </CardContent>
              </Card>
              <Card>
                <CardHeader>
                  <CardTitle>Переходи між фазами</CardTitle>
                </CardHeader>
                <CardContent className="max-h-56 space-y-1.5 overflow-y-auto text-xs">
                  {session.last_transition ? (
                    <TransitionList />
                  ) : (
                    <p className="text-muted-foreground">Переходів ще немає.</p>
                  )}
                </CardContent>
              </Card>
            </div>
          </div>
          <div className="grid gap-3 lg:grid-cols-2 2xl:grid-cols-3">
            <ChartCard title="Висота над землею [м]">
              <TimeSeriesChart data={data} unit="м" markers={markers} series={[
                { key: 'altitude_agl_m', label: 'Модель', color: 'var(--series-1)' },
                { key: 'ref_altitude_agl_m', label: 'Завдання', color: 'var(--ink-muted)', dashed: true },
              ]} />
            </ChartCard>
            <ChartCard title="Вертикальна швидкість [м/с]">
              <TimeSeriesChart data={data} unit="м/с" series={[
                { key: 'vz_m_s', label: 'Модель', color: 'var(--series-1)' },
                { key: 'ref_vz_m_s', label: 'Завдання', color: 'var(--ink-muted)', dashed: true },
              ]} />
            </ChartCard>
            <ChartCard title="Курс [°]">
              <TimeSeriesChart data={headingData} unit="°" digits={0} yDomain={[0, 360]} series={[
                { key: 'heading_deg', label: 'Модель', color: 'var(--series-1)' },
                { key: 'ref_heading_deg', label: 'Завдання', color: 'var(--ink-muted)', dashed: true },
              ]} />
            </ChartCard>
            <ChartCard title="Шляхова та повітряна швидкість [м/с]">
              <TimeSeriesChart data={data} unit="м/с" series={[
                { key: 'ground_speed_m_s', label: 'Шляхова', color: 'var(--series-1)' },
                { key: 'airspeed_m_s', label: 'Повітряна', color: 'var(--series-3)' },
                { key: 'ref_airspeed_m_s', label: 'Завдання повітряної', color: 'var(--ink-muted)', dashed: true },
              ]} />
            </ChartCard>
            <ChartCard title="Тяга рушіїв [Н]">
              <TimeSeriesChart data={data} unit="Н" series={[
                { key: 'thrust_heave_n', label: 'Вертикальний', color: 'var(--series-2)' },
                { key: 'thrust_left_n', label: 'Лівий маршовий', color: 'var(--series-1)' },
                { key: 'thrust_right_n', label: 'Правий маршовий', color: 'var(--series-3)' },
              ]} />
            </ChartCard>
            <ChartCard title="Вітер у точці апарата [м/с]">
              <TimeSeriesChart data={data} unit="м/с" series={[
                { key: 'wind_speed_m_s', label: 'Горизонтальний', color: 'var(--series-1)' },
                { key: 'wind_z_m_s', label: 'Вертикальний', color: 'var(--series-2)' },
              ]} />
            </ChartCard>
          </div>
          <RecentEvents />
          <p className="text-[11px] text-subtle-foreground">Змодельовані дані. Тангаж {fmt(current?.pitch_deg, 1)}°, крен {fmt(current?.roll_deg, 1)}° при t = {fmt(current?.t_s, 1)} с.</p>
        </>
      )}
    </div>
  )
}

export function ChartCard({ title, children }: { title: string; children: ReactNode }) {
  return (
    <Card>
      <CardHeader className="py-2">
        <CardTitle className="text-xs">{title}</CardTitle>
      </CardHeader>
      <CardContent className="px-2 pt-2 pb-1">{children}</CardContent>
    </Card>
  )
}

function TransitionList() {
  const results = useSimulationStore((s) => s.results)
  const session = useSimulationStore((s) => s.session)
  const list = results?.transitions ?? (session?.last_transition ? [session.last_transition] : [])
  return (
    <>
      {[...list].reverse().map((t) => (
        <div key={`${t.time_s}-${t.to}`} className="border-l-2 border-border-strong pl-2">
          <div className="flex items-center gap-2">
            <span className="tabular text-muted-foreground">{fmt(t.time_s, 1)} с</span>
            <span className="font-medium">
              {PHASE_LABEL[t.from]} → {PHASE_LABEL[t.to]}
            </span>
          </div>
          <div className="text-muted-foreground">{t.reason}</div>
        </div>
      ))}
      {!results ? <p className="text-[11px] text-subtle-foreground">Повний журнал переходів доступний після завершення прогону (сторінка «Результати»).</p> : null}
    </>
  )
}

function RecentEvents() {
  const session = useSimulationStore((s) => s.session)
  const events = session?.recent_events ?? []
  if (events.length === 0) return null
  return (
    <Card>
      <CardHeader>
        <CardTitle>Останні події безпеки</CardTitle>
      </CardHeader>
      <CardContent className="space-y-1.5 text-xs">
        {events.map((e) => (
          <div key={`${e.time_s}-${e.constraint}-${e.severity}`} className="flex items-start gap-2">
            <SeverityBadge severity={e.severity} />
            <span className="tabular text-muted-foreground">{fmt(e.time_s, 1)} с</span>
            <span>{e.message}</span>
          </div>
        ))}
      </CardContent>
    </Card>
  )
}
