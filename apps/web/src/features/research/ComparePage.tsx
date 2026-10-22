import { AlertTriangle, CheckCircle2, Download, FileJson, FlaskConical, GitCompare, Loader2, Play, Upload } from 'lucide-react'
import { useMemo, useState } from 'react'

import { EmptyState, ErrorBanner, LandingBadge, PageHeader } from '@/components/common/Display'
import { TopView } from '@/components/common/FlightViews'
import { TimeSeriesChart } from '@/components/common/TimeSeriesChart'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/overlays'
import { Badge } from '@/components/ui/primitives'
import { Table, TBody, Td, Th, THead, Tr } from '@/components/ui/table'
import { ChartCard } from '@/features/simulation/SimulationPage'
import { comparisonMetricsToCsv } from '@/lib/export'
import { fmt } from '@/lib/format'
import { downloadBlob, slug } from '@/lib/utils'
import { useConfigStore } from '@/stores/configStore'
import { useResearchStore } from '@/stores/researchStore'
import { useUiStore } from '@/stores/uiStore'
import type { ComparisonReport, TelemetrySample } from '@/types/api'

import { ResultsView } from './ResultsView'

const RUN_COLORS = ['var(--series-1)', 'var(--series-2)', 'var(--series-3)', 'var(--series-4)']

export function ComparePage() {
  const config = useConfigStore((s) => s.config)
  const experiments = useConfigStore((s) => s.experiments)
  const setConfig = useConfigStore((s) => s.setConfig)
  const setPage = useUiStore((s) => s.setPage)
  const { report, title, running, error, compareControllers, runExperiment } = useResearchStore()
  const [focus, setFocus] = useState<string[] | null>(null)
  if (!config) return null

  return (
    <div className="space-y-3">
      <PageHeader
        title="Experiments and controller comparison"
        description="Predefined, repeatable experiments A–G and side-by-side comparison. Runs execute to completion in the Rust engine (batch mode) with identical initial conditions, parameters and disturbance seeds; the engine verifies that only the controller differs."
        actions={
          <Button size="sm" onClick={() => void compareControllers(config).then(() => setFocus(null))} disabled={running}>
            {running ? <Loader2 className="animate-spin" /> : <GitCompare />} Compare A vs B on current config
          </Button>
        }
      />
      {error ? <ErrorBanner message={error} /> : null}
      <div className="grid gap-2 sm:grid-cols-2 xl:grid-cols-4">
        {experiments.map((e) => (
          <Card key={e.id} className="flex flex-col">
            <CardHeader className="py-2">
              <div>
                <CardTitle>
                  {e.id}. {e.title}
                </CardTitle>
                <CardDescription className="mt-0.5">{e.objective}</CardDescription>
              </div>
            </CardHeader>
            <CardContent className="flex flex-1 flex-col justify-between gap-2 py-2.5">
              <p className="text-[11px] text-muted-foreground">{e.description}</p>
              <div className="flex flex-wrap gap-1.5">
                <Button
                  size="sm"
                  disabled={running}
                  onClick={() => {
                    setFocus(e.focus_metrics)
                    void runExperiment(e.id, `Experiment ${e.id}: ${e.title}`)
                  }}
                >
                  <Play /> Run {e.runs.length > 1 ? `(${e.runs.length} runs)` : ''}
                </Button>
                <Button
                  size="sm"
                  variant="secondary"
                  onClick={() => {
                    const first = e.runs[0]
                    if (first) {
                      setConfig(structuredClone(first.config))
                      setPage('simulation')
                    }
                  }}
                >
                  <Upload /> Load into workspace
                </Button>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>

      {running ? (
        <div className="flex items-center gap-2 rounded-lg border border-border bg-surface p-4 text-sm text-muted-foreground">
          <Loader2 className="size-4 animate-spin" /> Running {title} in the Rust engine…
        </div>
      ) : report ? (
        <ComparisonView report={report} title={title ?? 'Comparison'} focus={focus} />
      ) : (
        <EmptyState title="No experiment results yet" icon={<FlaskConical />}>
          Run an experiment or compare both controllers on the current configuration.
        </EmptyState>
      )}
    </div>
  )
}

function ComparisonView({ report, title, focus }: { report: ComparisonReport; title: string; focus: string[] | null }) {
  const config = useConfigStore((s) => s.config)
  const validation = useConfigStore((s) => s.validation)
  const [tab, setTab] = useState('overview')
  const merged = useMemo(() => mergeByTime(report), [report])
  const first = report.runs[0]
  if (!first || !config) return null
  const metrics = first.results.metrics
  const ordered = focus ? [...metrics.filter((m) => focus.includes(m.key)), ...metrics.filter((m) => !focus.includes(m.key))] : metrics
  const series = (key: string) => report.runs.map((r, i) => ({ key: `${key}__${i}`, label: r.label, color: RUN_COLORS[i] ?? 'var(--series-5)' }))

  return (
    <div className="space-y-3">
      <Card>
        <CardHeader>
          <div>
            <CardTitle className="text-sm">{title}</CardTitle>
            <CardDescription>
              {report.runs.length} run(s) · {report.identical_conditions ? 'identical initial conditions, parameters and seeds (only the controller differs)' : 'configurations differ beyond the controller'}
            </CardDescription>
          </div>
          <div className="flex flex-wrap gap-1.5">
            {report.identical_conditions ? (
              <Badge variant="good">
                <CheckCircle2 /> identical conditions
              </Badge>
            ) : (
              <Badge variant="warning">
                <AlertTriangle /> {report.differences.length} differing field(s)
              </Badge>
            )}
            <Button size="sm" variant="secondary" onClick={() => downloadBlob(new Blob([comparisonMetricsToCsv(report)], { type: 'text/csv' }), `${slug(title)}-metrics.csv`)}>
              <Download /> Metrics CSV
            </Button>
            <Button
              size="sm"
              variant="secondary"
              onClick={() => downloadBlob(new Blob([JSON.stringify({ format: 'robotic-airship-comparison/v1', title, ...report }, null, 1)], { type: 'application/json' }), `${slug(title)}.json`)}
            >
              <FileJson /> Comparison JSON
            </Button>
          </div>
        </CardHeader>
        {!report.identical_conditions ? (
          <CardContent className="text-xs text-muted-foreground">
            Differing fields: <span className="font-mono">{report.differences.slice(0, 12).join(', ')}</span>
          </CardContent>
        ) : null}
      </Card>

      <Tabs value={tab} onValueChange={setTab}>
        <TabsList>
          <TabsTrigger value="overview">Comparison</TabsTrigger>
          {report.runs.map((r, i) => (
            <TabsTrigger key={i} value={`run-${i}`}>
              {r.label}
            </TabsTrigger>
          ))}
        </TabsList>
        <TabsContent value="overview" className="space-y-3">
          <Card>
            <CardHeader>
              <div>
                <CardTitle>Metrics</CardTitle>
                <CardDescription>Values as computed; no ranking is implied. Experiment focus metrics are listed first.</CardDescription>
              </div>
            </CardHeader>
            <CardContent className="p-0">
              <Table>
                <THead>
                  <Tr>
                    <Th>Metric</Th>
                    <Th>Unit</Th>
                    {report.runs.map((r, i) => (
                      <Th key={i} className="text-right">
                        <span className="mr-1 inline-block h-2 w-2 rounded-full" style={{ background: RUN_COLORS[i] }} />
                        {r.label}
                      </Th>
                    ))}
                    {report.runs.length === 2 ? <Th className="text-right">Δ (2 − 1)</Th> : null}
                  </Tr>
                </THead>
                <TBody>
                  <Tr>
                    <Td className="font-medium">Landing classification</Td>
                    <Td />
                    {report.runs.map((r, i) => (
                      <Td key={i} className="text-right">
                        <LandingBadge c={r.results.landing.classification} />
                      </Td>
                    ))}
                    {report.runs.length === 2 ? <Td /> : null}
                  </Tr>
                  {ordered.map((m) => {
                    const values = report.runs.map((r) => r.results.metrics.find((x) => x.key === m.key)?.value ?? null)
                    const [a, b] = values
                    return (
                      <Tr key={m.key} className={focus?.includes(m.key) ? 'bg-primary/5' : undefined}>
                        <Td title={m.definition}>{m.label}</Td>
                        <Td className="text-muted-foreground">{m.unit}</Td>
                        {values.map((v, i) => (
                          <Td key={i} className="text-right tabular">
                            {v === null ? '—' : fmt(v, Math.abs(v) >= 100 ? 1 : 3)}
                          </Td>
                        ))}
                        {report.runs.length === 2 ? (
                          <Td className="text-right tabular text-muted-foreground">{a != null && b != null ? fmt(b - a, 3) : '—'}</Td>
                        ) : null}
                      </Tr>
                    )
                  })}
                </TBody>
              </Table>
            </CardContent>
          </Card>
          <div className="grid gap-3 xl:grid-cols-2">
            <Card>
              <CardHeader>
                <CardTitle>Trajectories</CardTitle>
              </CardHeader>
              <CardContent>
                <TopView
                  config={config}
                  approach={validation?.approach}
                  overlays={report.runs.map((r, i) => ({ label: r.label, color: RUN_COLORS[i] ?? 'var(--series-5)', samples: r.telemetry }))}
                  height={380}
                />
                <p className="mt-1.5 text-[11px] text-muted-foreground">Geometry drawn from the current workspace configuration; trajectories from each run.</p>
              </CardContent>
            </Card>
            <div className="grid gap-3">
              <ChartCard title="Altitude above ground [m]">
                <TimeSeriesChart data={merged} unit="m" height={160} series={series('altitude_agl_m')} />
              </ChartCard>
              <ChartCard title="Cross-track error [m]">
                <TimeSeriesChart data={merged} unit="m" height={160} series={series('err_cross_track_m')} />
              </ChartCard>
            </div>
          </div>
          <div className="grid gap-3 lg:grid-cols-3">
            <ChartCard title="Vertical speed [m/s]">
              <TimeSeriesChart data={merged} unit="m/s" height={160} series={series('vz_m_s')} />
            </ChartCard>
            <ChartCard title="Heave usage (normalised)">
              <TimeSeriesChart data={merged} unit="" height={160} yDomain={[0, 1]} series={series('u_heave')} />
            </ChartCard>
            <ChartCard title="Yaw usage (normalised)">
              <TimeSeriesChart data={merged} unit="" height={160} yDomain={[0, 1]} series={series('u_yaw')} />
            </ChartCard>
          </div>
          <p className="text-[11px] text-subtle-foreground">{report.disclaimer}</p>
        </TabsContent>
        {report.runs.map((r, i) => (
          <TabsContent key={i} value={`run-${i}`}>
            <ResultsView report={r.results} telemetry={r.telemetry} />
          </TabsContent>
        ))}
      </Tabs>
    </div>
  )
}

/** Merge run telemetry on a common time axis keyed `<field>__<run>` for overlay charts. */
export function mergeByTime(report: ComparisonReport): Record<string, number | null>[] {
  const keys: (keyof TelemetrySample)[] = ['altitude_agl_m', 'err_cross_track_m', 'vz_m_s', 'u_heave', 'u_yaw']
  const rows = new Map<number, Record<string, number | null>>()
  report.runs.forEach((r, i) => {
    for (const s of r.telemetry) {
      const t = Math.round(s.t_s * 2) / 2
      const row = rows.get(t) ?? { t_s: t }
      for (const k of keys) row[`${k}__${i}`] = s[k] as number | null
      rows.set(t, row)
    }
  })
  return [...rows.values()].sort((a, b) => (a.t_s as number) - (b.t_s as number))
}
