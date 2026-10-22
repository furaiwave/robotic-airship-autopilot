import { CheckCircle2, Info, XCircle } from 'lucide-react'
import { useMemo } from 'react'

import { KeyValue, LandingBadge, SeverityBadge } from '@/components/common/Display'
import { TimeSeriesChart } from '@/components/common/TimeSeriesChart'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Tooltip } from '@/components/ui/overlays'
import { Badge } from '@/components/ui/primitives'
import { Table, TBody, Td, Th, THead, Tr } from '@/components/ui/table'
import { ChartCard, chartData } from '@/features/simulation/SimulationPage'
import { fmt, PHASE_LABEL, phaseTone } from '@/lib/format'
import type { Metric, ResultsReport, TelemetrySample } from '@/types/api'

export function ResultsView({ report, telemetry }: { report: ResultsReport; telemetry: TelemetrySample[] }) {
  const data = useMemo(() => chartData(telemetry, 1200), [telemetry])
  const markers = report.transitions.map((t) => ({ x: t.time_s, label: t.to }))
  const groups = [...new Set(report.metrics.map((m) => m.group))]
  const total = Math.max(report.numerical.simulated_time_s, 1e-6)
  const td = report.landing.touchdown

  return (
    <div className="space-y-3">
      <Card>
        <CardHeader>
          <div className="space-y-1">
            <div className="flex flex-wrap items-center gap-2">
              <CardTitle className="text-sm">{report.name}</CardTitle>
              <LandingBadge c={report.landing.classification} />
              {report.partial ? <Badge variant="warning">partial (run in progress)</Badge> : null}
            </div>
            <CardDescription>
              {report.controller_label} · finished: {report.finish_reason ?? 'running'} · config hash <span className="font-mono">{report.config_hash}</span>
              {report.turbulence_seed !== null ? ` · turbulence seed ${report.turbulence_seed}` : ''}
              {report.sensor_noise_seed !== null ? ` · noise seed ${report.sensor_noise_seed}` : ''}
            </CardDescription>
          </div>
        </CardHeader>
        <CardContent className="grid gap-4 lg:grid-cols-[minmax(0,1.2fr)_minmax(0,1fr)]">
          <div className="space-y-2">
            <p className="text-xs">{report.landing.explanation}</p>
            <p className="text-xs text-muted-foreground">{report.finish_message}</p>
            <ul className="space-y-1">
              {report.landing.criteria.map((c) => (
                <li key={c.name} className="flex items-start gap-2 text-xs">
                  {c.passed ? <CheckCircle2 className="mt-0.5 size-3.5 shrink-0 text-good" /> : <XCircle className="mt-0.5 size-3.5 shrink-0 text-critical" />}
                  <span>
                    <span className="font-medium">{c.name}</span> — <span className="text-muted-foreground">{c.detail}</span>
                  </span>
                </li>
              ))}
            </ul>
          </div>
          <div>
            {td ? (
              <KeyValue
                items={[
                  ['Touchdown time', `${fmt(td.time_s, 2)} s (${PHASE_LABEL[td.phase]})`],
                  ['Distance to target', `${fmt(td.distance_to_target_m, 2)} m`],
                  ['Along / cross axis', `${fmt(td.along_axis_m, 2)} / ${fmt(td.cross_axis_m, 2)} m`],
                  ['Horizontal speed', `${fmt(td.horizontal_speed_m_s, 3)} m/s`],
                  ['Vertical speed', `${fmt(td.vertical_speed_m_s, 3)} m/s`],
                  ['Inside landing area', td.inside_landing_area ? 'yes' : 'no'],
                ]}
              />
            ) : (
              <p className="text-xs text-muted-foreground">No touchdown recorded.</p>
            )}
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Phase timeline</CardTitle>
          <span className="text-xs text-muted-foreground">{fmt(total, 1)} s simulated</span>
        </CardHeader>
        <CardContent>
          <div className="flex h-6 w-full overflow-hidden rounded" role="img" aria-label="Phase timeline">
            {report.phase_timeline
              .filter((p) => p.duration_s > 0)
              .map((p, i) => {
                const tone = phaseTone(p.phase)
                const color = tone === 'warning' ? 'var(--status-warning)' : tone === 'danger' ? 'var(--status-critical)' : tone === 'success' ? 'var(--status-good)' : `var(--phase-${tone})`
                return (
                  <Tooltip key={i} content={`${PHASE_LABEL[p.phase]}: ${fmt(p.start_s, 1)}–${fmt(p.end_s, 1)} s (${fmt(p.duration_s, 1)} s)`}>
                    <div className="h-full border-r-2 border-surface" style={{ width: `${(p.duration_s / total) * 100}%`, background: color, opacity: i % 2 ? 0.75 : 0.95 }} />
                  </Tooltip>
                )
              })}
          </div>
          <div className="mt-2 flex flex-wrap gap-x-3 gap-y-1 text-[11px] text-muted-foreground">
            {report.phase_timeline
              .filter((p) => p.duration_s > 0)
              .map((p, i) => (
                <span key={i} className="tabular">
                  {PHASE_LABEL[p.phase]} {fmt(p.duration_s, 1)} s
                </span>
              ))}
          </div>
        </CardContent>
      </Card>

      <div className="grid gap-3 lg:grid-cols-2 2xl:grid-cols-3">
        <ChartCard title="Altitude vs reference [m]">
          <TimeSeriesChart data={data} unit="m" markers={markers} series={[
            { key: 'altitude_agl_m', label: 'Simulated', color: 'var(--series-1)' },
            { key: 'ref_altitude_agl_m', label: 'Reference', color: 'var(--ink-muted)', dashed: true },
          ]} />
        </ChartCard>
        <ChartCard title="Position: East and North [m]">
          <TimeSeriesChart data={data} unit="m" series={[
            { key: 'x_m', label: 'East', color: 'var(--series-1)' },
            { key: 'y_m', label: 'North', color: 'var(--series-2)' },
          ]} />
        </ChartCard>
        <ChartCard title="Horizontal speeds [m/s]">
          <TimeSeriesChart data={data} unit="m/s" series={[
            { key: 'ground_speed_m_s', label: 'Ground speed', color: 'var(--series-1)' },
            { key: 'ref_airspeed_m_s', label: 'Airspeed ref.', color: 'var(--ink-muted)', dashed: true },
            { key: 'airspeed_forward_m_s', label: 'Airspeed (axial)', color: 'var(--series-3)' },
          ]} />
        </ChartCard>
        <ChartCard title="Vertical velocity [m/s]">
          <TimeSeriesChart data={data} unit="m/s" series={[
            { key: 'vz_m_s', label: 'Simulated', color: 'var(--series-1)' },
            { key: 'ref_vz_m_s', label: 'Reference', color: 'var(--ink-muted)', dashed: true },
          ]} />
        </ChartCard>
        <ChartCard title="Tracking errors: altitude and cross-track [m]">
          <TimeSeriesChart data={data} unit="m" series={[
            { key: 'err_altitude_m', label: 'Altitude error', color: 'var(--series-1)' },
            { key: 'err_cross_track_m', label: 'Cross-track error', color: 'var(--series-2)' },
          ]} />
        </ChartCard>
        <ChartCard title="Heading error [deg]">
          <TimeSeriesChart data={data} unit="deg" digits={1} series={[{ key: 'err_heading_deg', label: 'Heading error', color: 'var(--series-1)' }]} />
        </ChartCard>
        <ChartCard title="Wind at the vehicle [m/s]">
          <TimeSeriesChart data={data} unit="m/s" series={[
            { key: 'wind_x_m_s', label: 'East', color: 'var(--series-1)' },
            { key: 'wind_y_m_s', label: 'North', color: 'var(--series-2)' },
            { key: 'wind_z_m_s', label: 'Up', color: 'var(--series-3)' },
          ]} />
        </ChartCard>
        <ChartCard title="Control outputs [N]">
          <TimeSeriesChart data={data} unit="N" series={[
            { key: 'cmd_heave_n', label: 'Heave cmd', color: 'var(--series-2)' },
            { key: 'cmd_surge_n', label: 'Surge cmd', color: 'var(--series-1)' },
          ]} />
        </ChartCard>
        <ChartCard title="Yaw moment command [N·m]">
          <TimeSeriesChart data={data} unit="N·m" series={[{ key: 'cmd_yaw_moment_nm', label: 'Yaw moment cmd', color: 'var(--series-1)' }]} />
        </ChartCard>
      </div>

      <Card>
        <CardHeader>
          <div>
            <CardTitle>Performance metrics</CardTitle>
            <CardDescription>Computed by the engine from recorded telemetry. Hover the info icon for the mathematical definition.</CardDescription>
          </div>
        </CardHeader>
        <CardContent className="grid gap-x-6 gap-y-3 p-0 lg:grid-cols-2">
          {groups.map((g) => (
            <MetricTable key={g} title={g} metrics={report.metrics.filter((m) => m.group === g)} />
          ))}
        </CardContent>
      </Card>

      <div className="grid gap-3 xl:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Phase transitions ({report.transitions.length})</CardTitle>
          </CardHeader>
          <CardContent className="max-h-96 overflow-y-auto p-0">
            <Table>
              <THead>
                <Tr>
                  <Th>t [s]</Th>
                  <Th>Transition</Th>
                  <Th>Reason</Th>
                </Tr>
              </THead>
              <TBody>
                {report.transitions.map((t) => (
                  <Tr key={`${t.time_s}-${t.to}`}>
                    <Td className="tabular">{fmt(t.time_s, 2)}</Td>
                    <Td className="whitespace-nowrap">
                      {PHASE_LABEL[t.from]} → <span className="font-medium">{PHASE_LABEL[t.to]}</span>
                    </Td>
                    <Td className="text-muted-foreground">{t.reason}</Td>
                  </Tr>
                ))}
              </TBody>
            </Table>
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Safety events ({report.safety_events.length})</CardTitle>
          </CardHeader>
          <CardContent className="max-h-96 overflow-y-auto p-0">
            {report.safety_events.length === 0 ? (
              <p className="p-4 text-xs text-muted-foreground">No constraint violations.</p>
            ) : (
              <Table>
                <THead>
                  <Tr>
                    <Th>t [s]</Th>
                    <Th>Severity</Th>
                    <Th>Event</Th>
                    <Th>Response</Th>
                  </Tr>
                </THead>
                <TBody>
                  {report.safety_events.map((e, i) => (
                    <Tr key={i}>
                      <Td className="tabular">{fmt(e.time_s, 1)}</Td>
                      <Td>
                        <SeverityBadge severity={e.severity} />
                      </Td>
                      <Td>
                        <div>{e.message}</div>
                        <div className="text-[11px] text-subtle-foreground">
                          {PHASE_LABEL[e.phase]} · h {fmt(e.state.altitude_agl_m, 1)} m · v {fmt(e.state.ground_speed_m_s, 1)} m/s
                          {e.cleared_at_s !== null ? ` · cleared at ${fmt(e.cleared_at_s, 1)} s` : ''}
                        </div>
                      </Td>
                      <Td className="text-muted-foreground">{e.response.replace(/_/g, ' ')}</Td>
                    </Tr>
                  ))}
                </TBody>
              </Table>
            )}
          </CardContent>
        </Card>
      </div>

      <div className="grid gap-3 xl:grid-cols-3">
        <Card>
          <CardHeader>
            <CardTitle>Preflight checks</CardTitle>
          </CardHeader>
          <CardContent className="space-y-1 text-xs">
            {report.preflight.map((c) => (
              <div key={c.name} className="flex items-start gap-2">
                {c.passed ? <CheckCircle2 className="mt-0.5 size-3.5 shrink-0 text-good" /> : <XCircle className="mt-0.5 size-3.5 shrink-0 text-critical" />}
                <span>
                  <span className="font-medium">{c.name}</span> — <span className="text-muted-foreground">{c.detail}</span>
                </span>
              </div>
            ))}
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Disturbances activated</CardTitle>
          </CardHeader>
          <CardContent className="space-y-1 text-xs">
            {report.disturbances.length === 0 ? <p className="text-muted-foreground">None.</p> : null}
            {report.disturbances.map((d) => (
              <div key={`${d.time_s}${d.description}`}>
                <span className="tabular text-muted-foreground">{fmt(d.time_s, 1)} s</span> {d.description}
              </div>
            ))}
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Numerics</CardTitle>
          </CardHeader>
          <CardContent>
            <KeyValue
              items={[
                ['Integrator', report.numerical.integrator],
                ['Time step', `${report.numerical.dt_s} s`],
                ['Control / telemetry rate', `${report.numerical.control_rate_hz} / ${report.numerical.telemetry_rate_hz} Hz`],
                ['Steps', report.numerical.steps.toLocaleString()],
                ['Wall-clock time', `${fmt(report.numerical.wall_time_ms, 1)} ms`],
                ['Real-time factor', `${fmt(report.numerical.real_time_factor, 0)}×`],
              ]}
            />
          </CardContent>
        </Card>
      </div>
      <p className="text-[11px] text-subtle-foreground">{report.disclaimer}</p>
    </div>
  )
}

function MetricTable({ title, metrics }: { title: string; metrics: Metric[] }) {
  return (
    <div>
      <div className="px-4 pt-3 text-[11px] font-medium tracking-wide text-subtle-foreground uppercase">{title}</div>
      <Table>
        <TBody>
          {metrics.map((m) => (
            <Tr key={m.key}>
              <Td className="w-full">
                <span className="inline-flex items-center gap-1">
                  {m.label}
                  <Tooltip content={m.definition}>
                    <button className="text-subtle-foreground hover:text-foreground" aria-label={`Definition of ${m.label}`}>
                      <Info className="size-3" />
                    </button>
                  </Tooltip>
                </span>
              </Td>
              <Td className="text-right whitespace-nowrap tabular">{m.value === null ? '—' : fmt(m.value, Math.abs(m.value) >= 100 ? 1 : 3)}</Td>
              <Td className="whitespace-nowrap text-muted-foreground">{m.unit}</Td>
            </Tr>
          ))}
        </TBody>
      </Table>
    </div>
  )
}
