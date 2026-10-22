import type { ComparisonReport, Metric, TelemetrySample } from '@/types/api'

function csvCell(v: unknown): string {
  if (v === null || v === undefined) return ''
  if (typeof v === 'number') return Number.isFinite(v) ? String(v) : ''
  if (typeof v === 'boolean') return v ? '1' : '0'
  const s = String(v)
  return /[",\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s
}

/** CSV of telemetry samples (column order of the first sample). */
export function telemetryToCsv(samples: TelemetrySample[]): string {
  const first = samples[0]
  if (!first) return ''
  const cols = Object.keys(first) as (keyof TelemetrySample)[]
  const lines = [cols.join(',')]
  for (const s of samples) lines.push(cols.map((c) => csvCell(s[c])).join(','))
  return `${lines.join('\n')}\n`
}

/** Metric-by-run comparison table as CSV (one row per metric). */
export function comparisonMetricsToCsv(report: ComparisonReport): string {
  const runs = report.runs
  const header = ['metric', 'unit', ...runs.map((r) => r.label), 'definition']
  const first = runs[0]
  if (!first) return `${header.join(',')}\n`
  const rows = first.results.metrics.map((m) => {
    const values = runs.map((r) => r.results.metrics.find((x) => x.key === m.key)?.value ?? null)
    return [m.label, m.unit, ...values, m.definition].map(csvCell).join(',')
  })
  return `# ${report.disclaimer}\n${[header.join(','), ...rows].join('\n')}\n`
}

export function metricValue(metrics: Metric[], key: string): number | null {
  return metrics.find((m) => m.key === key)?.value ?? null
}
