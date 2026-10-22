import { Activity } from 'lucide-react'
import { type ReactNode, useMemo, useState } from 'react'

import { EmptyState, PageHeader, PhaseBadge, SeverityBadge } from '@/components/common/Display'
import { SideView, TopView } from '@/components/common/FlightViews'
import { TimeSeriesChart } from '@/components/common/TimeSeriesChart'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/overlays'
import { breakAngleWraps, fmt, PHASE_LABEL } from '@/lib/format'
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
        title="Simulation workspace"
        description="The Rust engine integrates the dynamics at a fixed time step; this page renders telemetry polled from the engine at 4 Hz, independent of the physics rate."
      />
      <SimulationControls />
      {!session ? (
        <EmptyState title="No simulation loaded" icon={<Activity />}>
          Load the current configuration to create a simulation session in the Rust engine. The configuration is validated server-side before the session is created.
        </EmptyState>
      ) : (
        <>
          <div className="grid gap-3 xl:grid-cols-[minmax(0,1.35fr)_minmax(0,1fr)]">
            <Card>
              <CardHeader>
                <CardTitle>Top-down trajectory</CardTitle>
                <div className="flex items-center gap-2">
                  <PhaseBadge phase={session.phase} />
                  <span className="max-w-72 truncate text-xs text-muted-foreground">{session.guidance_mode}</span>
                </div>
              </CardHeader>
              <CardContent>
                <TopView config={view} approach={validation?.approach} trajectory={telemetry} current={current} height={430} />
              </CardContent>
            </Card>
            <div className="space-y-3">
              <Card>
                <CardHeader>
                  <CardTitle>Side view</CardTitle>
                  <Tabs value={mode} onValueChange={(v) => setMode(v as typeof mode)}>
                    <TabsList>
                      <TabsTrigger value="final">Final course</TabsTrigger>
                      <TabsTrigger value="east">East</TabsTrigger>
                      <TabsTrigger value="north">North</TabsTrigger>
                    </TabsList>
                  </Tabs>
                </CardHeader>
                <CardContent>
                  <SideView config={view} approach={validation?.approach} trajectory={telemetry} current={current} mode={mode} height={200} />
                  <p className="mt-1.5 text-[11px] text-muted-foreground">Blue: simulated gear height. Dashed grey: altitude reference. Dashed violet: glide path.</p>
                </CardContent>
              </Card>
              <Card>
                <CardHeader>
                  <CardTitle>Phase transitions</CardTitle>
                </CardHeader>
                <CardContent className="max-h-56 space-y-1.5 overflow-y-auto text-xs">
                  {session.last_transition ? (
                    <TransitionList />
                  ) : (
                    <p className="text-muted-foreground">No transitions yet.</p>
                  )}
                </CardContent>
              </Card>
            </div>
          </div>
          <div className="grid gap-3 lg:grid-cols-2 2xl:grid-cols-3">
            <ChartCard title="Altitude above ground [m]">
              <TimeSeriesChart data={data} unit="m" markers={markers} series={[
                { key: 'altitude_agl_m', label: 'Simulated', color: 'var(--series-1)' },
                { key: 'ref_altitude_agl_m', label: 'Reference', color: 'var(--ink-muted)', dashed: true },
              ]} />
            </ChartCard>
            <ChartCard title="Vertical speed [m/s]">
              <TimeSeriesChart data={data} unit="m/s" series={[
                { key: 'vz_m_s', label: 'Simulated', color: 'var(--series-1)' },
                { key: 'ref_vz_m_s', label: 'Reference', color: 'var(--ink-muted)', dashed: true },
              ]} />
            </ChartCard>
            <ChartCard title="Heading [deg]">
              <TimeSeriesChart data={headingData} unit="deg" digits={0} yDomain={[0, 360]} series={[
                { key: 'heading_deg', label: 'Simulated', color: 'var(--series-1)' },
                { key: 'ref_heading_deg', label: 'Reference', color: 'var(--ink-muted)', dashed: true },
              ]} />
            </ChartCard>
            <ChartCard title="Ground speed and airspeed [m/s]">
              <TimeSeriesChart data={data} unit="m/s" series={[
                { key: 'ground_speed_m_s', label: 'Ground speed', color: 'var(--series-1)' },
                { key: 'airspeed_m_s', label: 'Airspeed', color: 'var(--series-3)' },
                { key: 'ref_airspeed_m_s', label: 'Airspeed ref.', color: 'var(--ink-muted)', dashed: true },
              ]} />
            </ChartCard>
            <ChartCard title="Thrust commands [N]">
              <TimeSeriesChart data={data} unit="N" series={[
                { key: 'thrust_heave_n', label: 'Heave', color: 'var(--series-2)' },
                { key: 'thrust_left_n', label: 'Port main', color: 'var(--series-1)' },
                { key: 'thrust_right_n', label: 'Starboard main', color: 'var(--series-3)' },
              ]} />
            </ChartCard>
            <ChartCard title="Wind at the vehicle [m/s]">
              <TimeSeriesChart data={data} unit="m/s" series={[
                { key: 'wind_speed_m_s', label: 'Horizontal', color: 'var(--series-1)' },
                { key: 'wind_z_m_s', label: 'Vertical', color: 'var(--series-2)' },
              ]} />
            </ChartCard>
          </div>
          <RecentEvents />
          <p className="text-[11px] text-subtle-foreground">Simulated data. Pitch {fmt(current?.pitch_deg, 1)}°, roll {fmt(current?.roll_deg, 1)}° at t = {fmt(current?.t_s, 1)} s.</p>
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
            <span className="tabular text-muted-foreground">{fmt(t.time_s, 1)} s</span>
            <span className="font-medium">
              {PHASE_LABEL[t.from]} → {PHASE_LABEL[t.to]}
            </span>
          </div>
          <div className="text-muted-foreground">{t.reason}</div>
        </div>
      ))}
      {!results ? <p className="text-[11px] text-subtle-foreground">Full transition log available when the run ends (Results page).</p> : null}
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
        <CardTitle>Recent safety events</CardTitle>
      </CardHeader>
      <CardContent className="space-y-1.5 text-xs">
        {events.map((e) => (
          <div key={`${e.time_s}-${e.constraint}-${e.severity}`} className="flex items-start gap-2">
            <SeverityBadge severity={e.severity} />
            <span className="tabular text-muted-foreground">{fmt(e.time_s, 1)} s</span>
            <span>{e.message}</span>
          </div>
        ))}
      </CardContent>
    </Card>
  )
}
