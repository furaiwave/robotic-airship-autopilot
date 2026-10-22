import { AlertTriangle, CheckCircle2, Gauge } from 'lucide-react'
import { useMemo } from 'react'

import { EmptyState, PageHeader, PhaseBadge, RoleLegend, SeverityBadge, StatCard } from '@/components/common/Display'
import { TopView } from '@/components/common/FlightViews'
import { TimeSeriesChart } from '@/components/common/TimeSeriesChart'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/primitives'
import { ChartCard, chartData } from '@/features/simulation/SimulationPage'
import { SimulationControls } from '@/features/simulation/SimulationControls'
import { controllerLabel, fmt, fmtSigned, PHASE_LABEL } from '@/lib/format'
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

  return (
    <div className="space-y-3">
      <PageHeader
        title="Flight operations"
        description={`Live state of the simulated airship (${controllerLabel(view.controller.kind)}). Values are simulated, not measured from a real vehicle.`}
        actions={<RoleLegend />}
      />
      <SimulationControls />
      {!session || !t ? (
        <EmptyState title="Waiting for telemetry" icon={<Gauge />}>
          Configure the vehicle, environment, flight path and autopilot, then press “Load config & start”.
        </EmptyState>
      ) : (
        <>
          <div className="grid grid-cols-2 gap-2 sm:grid-cols-4 xl:grid-cols-8">
            <div className="col-span-2 rounded-md border border-border bg-surface px-3 py-2">
              <div className="text-[11px] text-muted-foreground">Flight phase · autopilot mode</div>
              <div className="mt-1 flex flex-wrap items-center gap-2">
                <PhaseBadge phase={t.phase} />
                <Badge variant="outline">{session.state}</Badge>
              </div>
              <div className="mt-1 truncate text-[11px] text-muted-foreground" title={session.guidance_mode}>
                {session.guidance_mode || '—'}
              </div>
            </div>
            <StatCard label="Altitude (gear AGL)" value={fmt(t.altitude_agl_m, 1)} unit="m" sub={t.ref_altitude_agl_m !== null ? `target ${fmt(t.ref_altitude_agl_m, 1)} m` : 'no altitude target'} />
            <StatCard label="Vertical speed" value={fmtSigned(t.vz_m_s, 2)} unit="m/s" sub={`target ${fmtSigned(t.ref_vz_m_s, 2)} m/s`} />
            <StatCard label="Ground speed" value={fmt(t.ground_speed_m_s, 2)} unit="m/s" sub={`airspeed ${fmt(t.airspeed_m_s, 2)} m/s`} />
            <StatCard label="Heading" value={fmt(t.heading_deg, 0)} unit="deg" sub={`target ${fmt(t.ref_heading_deg, 0)}°`} />
            <StatCard label="Pitch / roll" value={`${fmtSigned(t.pitch_deg, 1)} / ${fmtSigned(t.roll_deg, 1)}`} unit="deg" sub={`limits ±${view.airship.limits.max_pitch_deg} / ±${view.airship.limits.max_roll_deg}°`} />
            <StatCard label="Position E / N" value={`${fmt(t.x_m, 0)} / ${fmt(t.y_m, 0)}`} unit="m" sub={`t = ${fmt(t.t_s, 1)} s`} />

            <StatCard label="Target landing point" value={`${fmt(landing.x, 0)} / ${fmt(landing.y, 0)}`} unit="m" role="target" sub={`tolerance ${view.autopilot.landing.landing_point_tolerance_m} m`} />
            <StatCard label="Distance to landing" value={fmt(t.distance_to_landing_m, 1)} unit="m" sub={`along course ${fmt(t.distance_to_go_m, 0)} m`} />
            <StatCard label="Wind" value={fmt(t.wind_speed_m_s, 1)} unit="m/s" sub={`from ${fmt(t.wind_from_deg, 0)}°, vertical ${fmtSigned(t.wind_z_m_s, 1)}`} />
            <StatCard label="Propulsion output" value={fmt(t.power_w, 0)} unit="W" role="command" sub={`battery ${fmt(t.battery_soc * 100, 1)} %`} />
            <StatCard label="Cross-track error" value={fmtSigned(t.err_cross_track_m, 1)} unit="m" sub={`alt. error ${fmtSigned(t.err_altitude_m, 1)} m`} />
            <StatCard label="Velocity errors" value={fmt(t.err_horizontal_velocity_m_s, 2)} unit="m/s" sub={`vz ${fmtSigned(t.err_vz_m_s, 2)}, hdg ${fmtSigned(t.err_heading_deg, 0)}°`} />
            <StatCard label="Commands Fx / Fz" value={`${fmt(t.cmd_surge_n, 0)} / ${fmt(t.cmd_heave_n, 0)}`} unit="N" role="command" sub={`N ${fmt(t.cmd_yaw_moment_nm, 0)} N·m, rudder ${fmtSigned(t.rudder_deg, 1)}°`} />
            <StatCard
              label="Actuators T_L / T_R / T_h"
              value={`${fmt(t.thrust_left_n, 0)} / ${fmt(t.thrust_right_n, 0)} / ${fmt(t.thrust_heave_n, 0)}`}
              unit="N"
              role="command"
              sub={t.sat_surge || t.sat_heave || t.sat_yaw ? `saturated: ${[t.sat_surge && 'surge', t.sat_heave && 'heave', t.sat_yaw && 'yaw'].filter(Boolean).join(', ')}` : 'within limits'}
            />
          </div>

          <div className="grid gap-3 xl:grid-cols-[minmax(0,1.2fr)_minmax(0,1fr)]">
            <Card>
              <CardHeader>
                <CardTitle>Position</CardTitle>
                <span className="text-xs text-muted-foreground">{t.moored ? 'moored at mast' : t.contact ? 'weight on gear' : 'airborne'}</span>
              </CardHeader>
              <CardContent>
                <TopView config={view} approach={validation?.approach} trajectory={telemetry} current={t} height={360} />
              </CardContent>
            </Card>
            <div className="space-y-3">
              <ChartCard title="Altitude [m] — last 120 s">
                <TimeSeriesChart data={data} unit="m" height={150} series={[
                  { key: 'altitude_agl_m', label: 'Simulated', color: 'var(--series-1)' },
                  { key: 'ref_altitude_agl_m', label: 'Target', color: 'var(--ink-muted)', dashed: true },
                ]} />
              </ChartCard>
              <ChartCard title="Normalised control usage — last 120 s">
                <TimeSeriesChart data={data} unit="" height={150} yDomain={[0, 1]} series={[
                  { key: 'u_heave', label: 'Heave', color: 'var(--series-2)' },
                  { key: 'u_surge', label: 'Surge', color: 'var(--series-1)' },
                  { key: 'u_yaw', label: 'Yaw', color: 'var(--series-3)' },
                ]} />
              </ChartCard>
              <Card>
                <CardHeader>
                  <CardTitle>Flight safety alerts</CardTitle>
                  <span className="text-xs text-muted-foreground">{session.safety_events} total</span>
                </CardHeader>
                <CardContent className="max-h-48 space-y-1.5 overflow-y-auto text-xs">
                  {events.length === 0 ? (
                    <div className="flex items-center gap-2 text-muted-foreground">
                      <CheckCircle2 className="size-3.5 text-good" /> No constraint violations recorded.
                    </div>
                  ) : (
                    events.map((e) => (
                      <div key={`${e.time_s}-${e.constraint}-${e.severity}`} className="flex items-start gap-2">
                        <SeverityBadge severity={e.severity} />
                        <span className="tabular text-muted-foreground">{fmt(e.time_s, 1)} s</span>
                        <span>
                          {e.message}
                          {e.cleared_at_s !== null ? <span className="text-subtle-foreground"> (cleared {fmt(e.cleared_at_s, 1)} s)</span> : null}
                        </span>
                      </div>
                    ))
                  )}
                  {session.last_transition ? (
                    <div className="mt-2 border-t border-border pt-2 text-muted-foreground">
                      <AlertTriangle className="mr-1 inline size-3" />
                      Last transition at {fmt(session.last_transition.time_s, 1)} s to {PHASE_LABEL[session.last_transition.to]}: {session.last_transition.reason}
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
