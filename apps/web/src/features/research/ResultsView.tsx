import { CheckCircle2, Info, XCircle } from 'lucide-react'
import { useMemo } from 'react'

import { KeyValue, LandingBadge, SeverityBadge } from '@/components/common/Display'
import { TimeSeriesChart } from '@/components/common/TimeSeriesChart'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Tooltip } from '@/components/ui/overlays'
import { Badge } from '@/components/ui/primitives'
import { Table, TBody, Td, Th, THead, Tr } from '@/components/ui/table'
import { ChartCard, chartData } from '@/features/simulation/SimulationPage'
import { controllerLabel, fmt, PHASE_LABEL, phaseTone } from '@/lib/format'
import { criterionUk, DISCLAIMER_UK, finishUk, metricUk, preflightUk, responseUk } from '@/lib/i18n'
import type { Metric, ResultsReport, TelemetrySample } from '@/types/api'

export function ResultsView({ report, telemetry }: { report: ResultsReport; telemetry: TelemetrySample[] }) {
  const data = useMemo(() => chartData(telemetry, 1200), [telemetry])
  const markers = report.transitions.map((t) => ({ x: t.time_s, label: t.to }))
  const metrics = useMemo(() => report.metrics.map(metricUk), [report.metrics])
  const groups = [...new Set(metrics.map((m) => m.group))]
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
              {report.partial ? <Badge variant="warning">проміжні (прогін триває)</Badge> : null}
            </div>
            <CardDescription>
              {controllerLabel(report.controller)} · завершення: {finishUk(report.finish_reason)} · хеш конфігурації <span className="font-mono">{report.config_hash}</span>
              {report.turbulence_seed !== null ? ` · зерно турбулентності ${report.turbulence_seed}` : ''}
              {report.sensor_noise_seed !== null ? ` · зерно шуму ${report.sensor_noise_seed}` : ''}
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
                    <span className="font-medium">{criterionUk(c.name)}</span> — <span className="text-muted-foreground">{c.detail}</span>
                  </span>
                </li>
              ))}
            </ul>
          </div>
          <div>
            {td ? (
              <KeyValue
                items={[
                  ['Час торкання', `${fmt(td.time_s, 2)} с (${PHASE_LABEL[td.phase]})`],
                  ['Відстань до цілі', `${fmt(td.distance_to_target_m, 2)} м`],
                  ['Уздовж / поперек осі', `${fmt(td.along_axis_m, 2)} / ${fmt(td.cross_axis_m, 2)} м`],
                  ['Горизонтальна швидкість', `${fmt(td.horizontal_speed_m_s, 3)} м/с`],
                  ['Вертикальна швидкість', `${fmt(td.vertical_speed_m_s, 3)} м/с`],
                  ['У межах посадкової площадки', td.inside_landing_area ? 'так' : 'ні'],
                ]}
              />
            ) : (
              <p className="text-xs text-muted-foreground">Торкання не зафіксовано.</p>
            )}
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Часова діаграма фаз</CardTitle>
          <span className="text-xs text-muted-foreground">змодельовано {fmt(total, 1)} с</span>
        </CardHeader>
        <CardContent>
          <div className="flex h-6 w-full overflow-hidden rounded" role="img" aria-label="Часова діаграма фаз">
            {report.phase_timeline
              .filter((p) => p.duration_s > 0)
              .map((p, i) => {
                const tone = phaseTone(p.phase)
                const color = tone === 'warning' ? 'var(--status-warning)' : tone === 'danger' ? 'var(--status-critical)' : tone === 'success' ? 'var(--status-good)' : `var(--phase-${tone})`
                return (
                  <Tooltip key={i} content={`${PHASE_LABEL[p.phase]}: ${fmt(p.start_s, 1)}–${fmt(p.end_s, 1)} с (${fmt(p.duration_s, 1)} с)`}>
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
                  {PHASE_LABEL[p.phase]} {fmt(p.duration_s, 1)} с
                </span>
              ))}
          </div>
        </CardContent>
      </Card>

      <div className="grid gap-3 lg:grid-cols-2 2xl:grid-cols-3">
        <ChartCard title="Висота та завдання [м]">
          <TimeSeriesChart
            data={data}
            unit="м"
            markers={markers}
            series={[
              { key: 'altitude_agl_m', label: 'Модель', color: 'var(--series-1)' },
              { key: 'ref_altitude_agl_m', label: 'Завдання', color: 'var(--ink-muted)', dashed: true },
            ]}
          />
        </ChartCard>
        <ChartCard title="Положення: схід і північ [м]">
          <TimeSeriesChart
            data={data}
            unit="м"
            series={[
              { key: 'x_m', label: 'Схід', color: 'var(--series-1)' },
              { key: 'y_m', label: 'Північ', color: 'var(--series-2)' },
            ]}
          />
        </ChartCard>
        <ChartCard title="Горизонтальні швидкості [м/с]">
          <TimeSeriesChart
            data={data}
            unit="м/с"
            series={[
              { key: 'ground_speed_m_s', label: 'Шляхова', color: 'var(--series-1)' },
              { key: 'ref_airspeed_m_s', label: 'Завдання повітряної', color: 'var(--ink-muted)', dashed: true },
              { key: 'airspeed_forward_m_s', label: 'Повітряна (поздовжня)', color: 'var(--series-3)' },
            ]}
          />
        </ChartCard>
        <ChartCard title="Вертикальна швидкість [м/с]">
          <TimeSeriesChart
            data={data}
            unit="м/с"
            series={[
              { key: 'vz_m_s', label: 'Модель', color: 'var(--series-1)' },
              { key: 'ref_vz_m_s', label: 'Завдання', color: 'var(--ink-muted)', dashed: true },
            ]}
          />
        </ChartCard>
        <ChartCard title="Похибки відстеження: висота й бокове відхилення [м]">
          <TimeSeriesChart
            data={data}
            unit="м"
            series={[
              { key: 'err_altitude_m', label: 'Похибка висоти', color: 'var(--series-1)' },
              { key: 'err_cross_track_m', label: 'Бокове відхилення', color: 'var(--series-2)' },
            ]}
          />
        </ChartCard>
        <ChartCard title="Похибка курсу [°]">
          <TimeSeriesChart data={data} unit="°" digits={1} series={[{ key: 'err_heading_deg', label: 'Похибка курсу', color: 'var(--series-1)' }]} />
        </ChartCard>
        <ChartCard title="Вітер у точці апарата [м/с]">
          <TimeSeriesChart
            data={data}
            unit="м/с"
            series={[
              { key: 'wind_x_m_s', label: 'Схід', color: 'var(--series-1)' },
              { key: 'wind_y_m_s', label: 'Північ', color: 'var(--series-2)' },
              { key: 'wind_z_m_s', label: 'Угору', color: 'var(--series-3)' },
            ]}
          />
        </ChartCard>
        <ChartCard title="Керувальні сигнали [Н]">
          <TimeSeriesChart
            data={data}
            unit="Н"
            series={[
              { key: 'cmd_heave_n', label: 'Команда вертикальної сили', color: 'var(--series-2)' },
              { key: 'cmd_surge_n', label: 'Команда поздовжньої сили', color: 'var(--series-1)' },
            ]}
          />
        </ChartCard>
        <ChartCard title="Команда моменту рискання [Н·м]">
          <TimeSeriesChart data={data} unit="Н·м" series={[{ key: 'cmd_yaw_moment_nm', label: 'Команда моменту рискання', color: 'var(--series-1)' }]} />
        </ChartCard>
      </div>

      <Card>
        <CardHeader>
          <div>
            <CardTitle>Показники якості</CardTitle>
            <CardDescription>Обчислено рушієм із записаної телеметрії. Наведіть на значок «i», щоб побачити математичне визначення.</CardDescription>
          </div>
        </CardHeader>
        <CardContent className="grid gap-x-6 gap-y-3 p-0 lg:grid-cols-2">
          {groups.map((g) => (
            <MetricTable key={g} title={g} metrics={metrics.filter((m) => m.group === g)} />
          ))}
        </CardContent>
      </Card>

      <div className="grid gap-3 xl:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Переходи між фазами ({report.transitions.length})</CardTitle>
          </CardHeader>
          <CardContent className="max-h-96 overflow-y-auto p-0">
            <Table>
              <THead>
                <Tr>
                  <Th>t [с]</Th>
                  <Th>Перехід</Th>
                  <Th>Причина</Th>
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
            <CardTitle>Події безпеки ({report.safety_events.length})</CardTitle>
          </CardHeader>
          <CardContent className="max-h-96 overflow-y-auto p-0">
            {report.safety_events.length === 0 ? (
              <p className="p-4 text-xs text-muted-foreground">Порушень обмежень немає.</p>
            ) : (
              <Table>
                <THead>
                  <Tr>
                    <Th>t [с]</Th>
                    <Th>Рівень</Th>
                    <Th>Подія</Th>
                    <Th>Реакція</Th>
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
                          {PHASE_LABEL[e.phase]} · h {fmt(e.state.altitude_agl_m, 1)} м · v {fmt(e.state.ground_speed_m_s, 1)} м/с
                          {e.cleared_at_s !== null ? ` · знято на ${fmt(e.cleared_at_s, 1)} с` : ''}
                        </div>
                      </Td>
                      <Td className="text-muted-foreground">{responseUk(e.response)}</Td>
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
            <CardTitle>Передпольотні перевірки</CardTitle>
          </CardHeader>
          <CardContent className="space-y-1 text-xs">
            {report.preflight.map((c) => (
              <div key={c.name} className="flex items-start gap-2">
                {c.passed ? <CheckCircle2 className="mt-0.5 size-3.5 shrink-0 text-good" /> : <XCircle className="mt-0.5 size-3.5 shrink-0 text-critical" />}
                <span>
                  <span className="font-medium">{preflightUk(c.name)}</span> — <span className="text-muted-foreground">{c.detail}</span>
                </span>
              </div>
            ))}
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Активовані збурення</CardTitle>
          </CardHeader>
          <CardContent className="space-y-1 text-xs">
            {report.disturbances.length === 0 ? <p className="text-muted-foreground">Немає.</p> : null}
            {report.disturbances.map((d) => (
              <div key={`${d.time_s}${d.description}`}>
                <span className="tabular text-muted-foreground">{fmt(d.time_s, 1)} с</span> {d.description}
              </div>
            ))}
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Чисельні параметри</CardTitle>
          </CardHeader>
          <CardContent>
            <KeyValue
              items={[
                ['Інтегратор', 'класичний Рунге–Кутта 4-го порядку, сталий крок'],
                ['Крок інтегрування', `${report.numerical.dt_s} с`],
                ['Частота керування / телеметрії', `${report.numerical.control_rate_hz} / ${report.numerical.telemetry_rate_hz} Гц`],
                ['Кроків', report.numerical.steps.toLocaleString('uk-UA')],
                ['Час обчислення', `${fmt(report.numerical.wall_time_ms, 1)} мс`],
                ['Коефіцієнт прискорення', `${fmt(report.numerical.real_time_factor, 0)}×`],
              ]}
            />
          </CardContent>
        </Card>
      </div>
      <p className="text-[11px] text-subtle-foreground">{DISCLAIMER_UK}</p>
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
                    <button className="text-subtle-foreground hover:text-foreground" aria-label={`Визначення: ${m.label}`}>
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
