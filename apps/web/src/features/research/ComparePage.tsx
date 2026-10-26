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
import { DISCLAIMER_UK, experimentUk, metricUk, runLabelUk } from '@/lib/i18n'
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
        title="Експерименти та порівняння регуляторів"
        description="Готові відтворювані експерименти A–G і порівняння пліч-о-пліч. Прогони виконуються до кінця в Rust-рушії (пакетний режим) з однаковими початковими умовами, параметрами та зернами збурень; рушій перевіряє, що відрізняється лише регулятор."
        actions={
          <Button size="sm" onClick={() => void compareControllers(config).then(() => setFocus(null))} disabled={running}>
            {running ? <Loader2 className="animate-spin" /> : <GitCompare />} Порівняти A і B на поточній конфігурації
          </Button>
        }
      />
      {error ? <ErrorBanner message={error} /> : null}
      <div className="grid gap-2 sm:grid-cols-2 xl:grid-cols-4">
        {experiments.map(experimentUk).map((e) => (
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
                    void runExperiment(e.id, `Експеримент ${e.id}: ${e.title}`)
                  }}
                >
                  <Play /> Запустити {e.runs.length > 1 ? `(${e.runs.length} прогони)` : ''}
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
                  <Upload /> Завантажити в робочу зону
                </Button>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>

      {running ? (
        <div className="flex items-center gap-2 rounded-lg border border-border bg-surface p-4 text-sm text-muted-foreground">
          <Loader2 className="size-4 animate-spin" /> Виконується «{title}» у Rust-рушії…
        </div>
      ) : report ? (
        <ComparisonView report={report} title={title ?? 'Порівняння'} focus={focus} />
      ) : (
        <EmptyState title="Результатів експериментів ще немає" icon={<FlaskConical />}>
          Запустіть експеримент або порівняйте обидва регулятори на поточній конфігурації.
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
  const metrics = first.results.metrics.map(metricUk)
  const ordered = focus ? [...metrics.filter((m) => focus.includes(m.key)), ...metrics.filter((m) => !focus.includes(m.key))] : metrics
  const series = (key: string) => report.runs.map((r, i) => ({ key: `${key}__${i}`, label: runLabelUk(r.label), color: RUN_COLORS[i] ?? 'var(--series-5)' }))

  return (
    <div className="space-y-3">
      <Card>
        <CardHeader>
          <div>
            <CardTitle className="text-sm">{title}</CardTitle>
            <CardDescription>
              прогонів: {report.runs.length} ·{' '}
              {report.identical_conditions ? 'однакові початкові умови, параметри та зерна (відрізняється лише регулятор)' : 'конфігурації відрізняються не лише регулятором'}
            </CardDescription>
          </div>
          <div className="flex flex-wrap gap-1.5">
            {report.identical_conditions ? (
              <Badge variant="good">
                <CheckCircle2 /> однакові умови
              </Badge>
            ) : (
              <Badge variant="warning">
                <AlertTriangle /> відмінних полів: {report.differences.length}
              </Badge>
            )}
            <Button size="sm" variant="secondary" onClick={() => downloadBlob(new Blob([comparisonMetricsToCsv(report)], { type: 'text/csv' }), `${slug(title)}-metrics.csv`)}>
              <Download /> Показники CSV
            </Button>
            <Button
              size="sm"
              variant="secondary"
              onClick={() => downloadBlob(new Blob([JSON.stringify({ format: 'robotic-airship-comparison/v1', title, ...report }, null, 1)], { type: 'application/json' }), `${slug(title)}.json`)}
            >
              <FileJson /> Порівняння JSON
            </Button>
          </div>
        </CardHeader>
        {!report.identical_conditions ? (
          <CardContent className="text-xs text-muted-foreground">
            Відмінні поля: <span className="font-mono">{report.differences.slice(0, 12).join(', ')}</span>
          </CardContent>
        ) : null}
      </Card>

      <Tabs value={tab} onValueChange={setTab}>
        <TabsList>
          <TabsTrigger value="overview">Порівняння</TabsTrigger>
          {report.runs.map((r, i) => (
            <TabsTrigger key={i} value={`run-${i}`}>
              {runLabelUk(r.label)}
            </TabsTrigger>
          ))}
        </TabsList>
        <TabsContent value="overview" className="space-y-3">
          <Card>
            <CardHeader>
              <div>
                <CardTitle>Показники</CardTitle>
                <CardDescription>Значення наведено як обчислено; ранжування не передбачається. Ключові показники експерименту — першими.</CardDescription>
              </div>
            </CardHeader>
            <CardContent className="p-0">
              <Table>
                <THead>
                  <Tr>
                    <Th>Показник</Th>
                    <Th>Одиниця</Th>
                    {report.runs.map((r, i) => (
                      <Th key={i} className="text-right">
                        <span className="mr-1 inline-block h-2 w-2 rounded-full" style={{ background: RUN_COLORS[i] }} />
                        {runLabelUk(r.label)}
                      </Th>
                    ))}
                    {report.runs.length === 2 ? <Th className="text-right">Δ (2 − 1)</Th> : null}
                  </Tr>
                </THead>
                <TBody>
                  <Tr>
                    <Td className="font-medium">Класифікація посадки</Td>
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
                <CardTitle>Траєкторії</CardTitle>
              </CardHeader>
              <CardContent>
                <TopView
                  config={config}
                  approach={validation?.approach}
                  overlays={report.runs.map((r, i) => ({ label: r.label, color: RUN_COLORS[i] ?? 'var(--series-5)', samples: r.telemetry }))}
                  height={380}
                />
                <p className="mt-1.5 text-[11px] text-muted-foreground">Геометрію побудовано з поточної конфігурації робочої зони; траєкторії — з кожного прогону.</p>
              </CardContent>
            </Card>
            <div className="grid gap-3">
              <ChartCard title="Висота над землею [м]">
                <TimeSeriesChart data={merged} unit="м" height={160} series={series('altitude_agl_m')} />
              </ChartCard>
              <ChartCard title="Бокове відхилення [м]">
                <TimeSeriesChart data={merged} unit="м" height={160} series={series('err_cross_track_m')} />
              </ChartCard>
            </div>
          </div>
          <div className="grid gap-3 lg:grid-cols-3">
            <ChartCard title="Вертикальна швидкість [м/с]">
              <TimeSeriesChart data={merged} unit="м/с" height={160} series={series('vz_m_s')} />
            </ChartCard>
            <ChartCard title="Використання вертикальної тяги (нормоване)">
              <TimeSeriesChart data={merged} unit="" height={160} yDomain={[0, 1]} series={series('u_heave')} />
            </ChartCard>
            <ChartCard title="Використання моменту рискання (нормоване)">
              <TimeSeriesChart data={merged} unit="" height={160} yDomain={[0, 1]} series={series('u_yaw')} />
            </ChartCard>
          </div>
          <p className="text-[11px] text-subtle-foreground">{DISCLAIMER_UK}</p>
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
