import { AlertTriangle, CheckCircle2, Gauge } from 'lucide-react'
import { useMemo } from 'react'

import { EmptyState, PageHeader, PhaseBadge, RoleLegend, SeverityBadge, StatCard } from '@/components/common/Display'
import { TopView } from '@/components/common/FlightViews'
import { TimeSeriesChart } from '@/components/common/TimeSeriesChart'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/primitives'
import { SimulationControls } from '@/features/simulation/SimulationControls'
import { ChartCard, chartData } from '@/features/simulation/SimulationPage'
import { controllerLabel, fmt, fmtSigned, PHASE_LABEL } from '@/lib/format'
import { guidanceModeUk, runStateUk } from '@/lib/i18n'
import { useConfigStore } from '@/stores/configStore'
import { useSimulationStore } from '@/stores/simulationStore'

export function DashboardPage() {
  const config = useConfigStore((s) => s.config)
  const validation = useConfigStore((s) => s.validation)
  const session = useSimulationStore((s) => s.session)
  const telemetry = useSimulationStore((s) => s.telemetry)
  const simConfig = useSimulationStore((s) => s.config)
  const data = useMemo(() => chartData(telemetry.slice(-1200), 600), [telemetry])
  if (!config) return null
  const view = simConfig ?? config
  const t = telemetry[telemetry.length - 1] ?? session?.latest ?? null
  const landing = view.environment.landing_site
  const events = session?.recent_events ?? []
  const saturated = t ? [t.sat_surge && 'поздовжня', t.sat_heave && 'вертикальна', t.sat_yaw && 'рискання'].filter(Boolean) : []

  return (
    <div className="space-y-3">
      <PageHeader
        title="Керування польотом"
        description={`Поточний стан модельованого дирижабля (${controllerLabel(view.controller.kind)}). Значення отримано моделюванням, а не виміряно на реальному апараті.`}
        actions={<RoleLegend />}
      />
      <SimulationControls />
      {!session || !t ? (
        <EmptyState title="Очікування телеметрії" icon={<Gauge />}>
          Налаштуйте апарат, середовище, маршрут і автопілот, потім натисніть «Завантажити й запустити».
        </EmptyState>
      ) : (
        <>
          <div className="grid grid-cols-2 gap-2 sm:grid-cols-4 xl:grid-cols-8">
            <div className="col-span-2 rounded-md border border-border bg-surface px-3 py-2">
              <div className="text-[11px] text-muted-foreground">Фаза польоту · режим автопілота</div>
              <div className="mt-1 flex flex-wrap items-center gap-2">
                <PhaseBadge phase={t.phase} />
                <Badge variant="outline">{runStateUk(session.state)}</Badge>
              </div>
              <div className="mt-1 truncate text-[11px] text-muted-foreground" title={guidanceModeUk(session.guidance_mode)}>
                {session.guidance_mode ? guidanceModeUk(session.guidance_mode) : '—'}
              </div>
            </div>
            <StatCard
              label="Висота над землею"
              value={fmt(t.altitude_agl_m, 1)}
              unit="м"
              sub={t.ref_altitude_agl_m !== null ? `завдання ${fmt(t.ref_altitude_agl_m, 1)} м` : 'без завдання висоти'}
            />
            <StatCard label="Вертикальна швидкість" value={fmtSigned(t.vz_m_s, 2)} unit="м/с" sub={`завдання ${fmtSigned(t.ref_vz_m_s, 2)} м/с`} />
            <StatCard label="Шляхова швидкість" value={fmt(t.ground_speed_m_s, 2)} unit="м/с" sub={`повітряна ${fmt(t.airspeed_m_s, 2)} м/с`} />
            <StatCard label="Курс" value={fmt(t.heading_deg, 0)} unit="°" sub={`завдання ${fmt(t.ref_heading_deg, 0)}°`} />
            <StatCard
              label="Тангаж / крен"
              value={`${fmtSigned(t.pitch_deg, 1)} / ${fmtSigned(t.roll_deg, 1)}`}
              unit="°"
              sub={`межі ±${view.airship.limits.max_pitch_deg} / ±${view.airship.limits.max_roll_deg}°`}
            />
            <StatCard label="Положення Сх / Пн" value={`${fmt(t.x_m, 0)} / ${fmt(t.y_m, 0)}`} unit="м" sub={`t = ${fmt(t.t_s, 1)} с`} />

            <StatCard
              label="Цільова точка посадки"
              value={`${fmt(landing.x, 0)} / ${fmt(landing.y, 0)}`}
              unit="м"
              role="target"
              sub={`допуск ${view.autopilot.landing.landing_point_tolerance_m} м`}
            />
            <StatCard label="До точки посадки" value={fmt(t.distance_to_landing_m, 1)} unit="м" sub={`уздовж курсу ${fmt(t.distance_to_go_m, 0)} м`} />
            <StatCard label="Вітер" value={fmt(t.wind_speed_m_s, 1)} unit="м/с" sub={`з ${fmt(t.wind_from_deg, 0)}°, вертикальний ${fmtSigned(t.wind_z_m_s, 1)}`} />
            <StatCard label="Потужність рушіїв" value={fmt(t.power_w, 0)} unit="Вт" role="command" sub={`батарея ${fmt(t.battery_soc * 100, 1)} %`} />
            <StatCard label="Бокове відхилення" value={fmtSigned(t.err_cross_track_m, 1)} unit="м" sub={`похибка висоти ${fmtSigned(t.err_altitude_m, 1)} м`} />
            <StatCard
              label="Похибки швидкості"
              value={fmt(t.err_horizontal_velocity_m_s, 2)}
              unit="м/с"
              sub={`vz ${fmtSigned(t.err_vz_m_s, 2)}, курс ${fmtSigned(t.err_heading_deg, 0)}°`}
            />
            <StatCard
              label="Команди Fx / Fz"
              value={`${fmt(t.cmd_surge_n, 0)} / ${fmt(t.cmd_heave_n, 0)}`}
              unit="Н"
              role="command"
              sub={`N ${fmt(t.cmd_yaw_moment_nm, 0)} Н·м, кермо ${fmtSigned(t.rudder_deg, 1)}°`}
            />
            <StatCard
              label="Рушії T_L / T_R / T_h"
              value={`${fmt(t.thrust_left_n, 0)} / ${fmt(t.thrust_right_n, 0)} / ${fmt(t.thrust_heave_n, 0)}`}
              unit="Н"
              role="command"
              sub={saturated.length > 0 ? `насичення: ${saturated.join(', ')}` : 'у межах обмежень'}
            />
          </div>

          <div className="grid gap-3 xl:grid-cols-[minmax(0,1.2fr)_minmax(0,1fr)]">
            <Card>
              <CardHeader>
                <CardTitle>Положення</CardTitle>
                <span className="text-xs text-muted-foreground">{t.moored ? 'пришвартовано до щогли' : t.contact ? 'навантаження на шасі' : 'у польоті'}</span>
              </CardHeader>
              <CardContent>
                <TopView config={view} approach={validation?.approach} trajectory={telemetry} current={t} height={360} />
              </CardContent>
            </Card>
            <div className="space-y-3">
              <ChartCard title="Висота [м] — останні 120 с">
                <TimeSeriesChart
                  data={data}
                  unit="м"
                  height={150}
                  series={[
                    { key: 'altitude_agl_m', label: 'Модель', color: 'var(--series-1)' },
                    { key: 'ref_altitude_agl_m', label: 'Завдання', color: 'var(--ink-muted)', dashed: true },
                  ]}
                />
              </ChartCard>
              <ChartCard title="Нормоване використання керування — останні 120 с">
                <TimeSeriesChart
                  data={data}
                  unit=""
                  height={150}
                  yDomain={[0, 1]}
                  series={[
                    { key: 'u_heave', label: 'Вертикальна тяга', color: 'var(--series-2)' },
                    { key: 'u_surge', label: 'Поздовжня тяга', color: 'var(--series-1)' },
                    { key: 'u_yaw', label: 'Рискання', color: 'var(--series-3)' },
                  ]}
                />
              </ChartCard>
              <Card>
                <CardHeader>
                  <CardTitle>Попередження безпеки польоту</CardTitle>
                  <span className="text-xs text-muted-foreground">усього {session.safety_events}</span>
                </CardHeader>
                <CardContent className="max-h-48 space-y-1.5 overflow-y-auto text-xs">
                  {events.length === 0 ? (
                    <div className="flex items-center gap-2 text-muted-foreground">
                      <CheckCircle2 className="size-3.5 text-good" /> Порушень обмежень не зафіксовано.
                    </div>
                  ) : (
                    events.map((e) => (
                      <div key={`${e.time_s}-${e.constraint}-${e.severity}`} className="flex items-start gap-2">
                        <SeverityBadge severity={e.severity} />
                        <span className="tabular text-muted-foreground">{fmt(e.time_s, 1)} с</span>
                        <span>
                          {e.message}
                          {e.cleared_at_s !== null ? <span className="text-subtle-foreground"> (знято на {fmt(e.cleared_at_s, 1)} с)</span> : null}
                        </span>
                      </div>
                    ))
                  )}
                  {session.last_transition ? (
                    <div className="mt-2 border-t border-border pt-2 text-muted-foreground">
                      <AlertTriangle className="mr-1 inline size-3" />
                      Останній перехід на {fmt(session.last_transition.time_s, 1)} с до фази «{PHASE_LABEL[session.last_transition.to]}»: {session.last_transition.reason}
                    </div>
                  ) : null}
                </CardContent>
              </Card>
            </div>
          </div>
        </>
      )}
    </div>
  )
}
