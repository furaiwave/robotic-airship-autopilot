import { CartesianGrid, Legend, Line, LineChart, ReferenceLine, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts'

import { fmt } from '@/lib/format'

export interface SeriesSpec {
  key: string
  label: string
  /** CSS colour (use the categorical tokens, e.g. `var(--series-1)`). */
  color: string
  dashed?: boolean
}

interface Props<T> {
  data: T[]
  series: SeriesSpec[]
  unit: string
  height?: number
  xKey?: string
  markers?: { x: number; label: string }[]
  yDomain?: [number | 'auto' | 'dataMin' | 'dataMax', number | 'auto' | 'dataMin' | 'dataMax']
  digits?: number
}

/** Time-series line chart with crosshair tooltip; one y-axis only. */
export function TimeSeriesChart<T extends object>({ data, series, unit, height = 180, xKey = 't_s', markers = [], yDomain, digits = 2 }: Props<T>) {
  return (
    <div style={{ height }} className="w-full" role="img" aria-label={`${series.map((s) => s.label).join(', ')} [${unit}] залежно від часу`}>
      <ResponsiveContainer width="100%" height="100%">
        <LineChart data={data} margin={{ top: 6, right: 12, bottom: 0, left: 0 }}>
          <CartesianGrid vertical={false} strokeWidth={1} />
          <XAxis
            dataKey={xKey}
            type="number"
            domain={['dataMin', 'dataMax']}
            tickFormatter={(v: number) => `${Math.round(v)}`}
            tickLine={false}
            minTickGap={28}
            unit=" с"
          />
          <YAxis width={52} tickLine={false} axisLine={false} domain={yDomain ?? ['auto', 'auto']} tickFormatter={(v: number) => fmt(v, Math.abs(v) < 10 ? 1 : 0)} />
          <Tooltip
            isAnimationActive={false}
            cursor={{ stroke: 'var(--ink-muted)', strokeWidth: 1 }}
            contentStyle={{ background: 'var(--surface-2)', border: '1px solid var(--border)', borderRadius: 6, fontSize: 11, padding: '6px 8px' }}
            labelStyle={{ color: 'var(--muted-foreground)', marginBottom: 2 }}
            itemStyle={{ color: 'var(--foreground)', padding: 0 }}
            labelFormatter={(v) => `t = ${fmt(Number(v), 1)} с`}
            formatter={(v) => [`${fmt(Number(v), digits)} ${unit}`]}
          />
          {series.length > 1 ? <Legend iconType="plainline" height={18} wrapperStyle={{ fontSize: 11 }} /> : null}
          {markers.map((m) => (
            <ReferenceLine key={`${m.x}-${m.label}`} x={m.x} stroke="var(--axis)" strokeDasharray="2 3" ifOverflow="extendDomain" />
          ))}
          {series.map((s) => (
            <Line
              key={s.key}
              dataKey={s.key}
              name={s.label}
              type="linear"
              dot={false}
              stroke={s.color}
              strokeWidth={s.dashed ? 1.5 : 2}
              strokeDasharray={s.dashed ? '5 4' : undefined}
              isAnimationActive={false}
              connectNulls={false}
            />
          ))}
        </LineChart>
      </ResponsiveContainer>
    </div>
  )
}
