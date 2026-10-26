import { zodResolver } from '@hookform/resolvers/zod'
import { ArrowDown, ArrowUp, Loader2, Trash2, Wand2 } from 'lucide-react'
import { useEffect, useMemo, useState } from 'react'
import { useForm } from 'react-hook-form'

import { PageHeader } from '@/components/common/Display'
import { TopView } from '@/components/common/FlightViews'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge, Input, Label } from '@/components/ui/primitives'
import { Table, TBody, Td, Th, THead, Tr } from '@/components/ui/table'
import { fmt } from '@/lib/format'
import { TEMPLATE_UK } from '@/lib/i18n'
import { flightPathSchema, type WaypointInput, waypointSchema } from '@/lib/schemas'
import { api } from '@/services/api'
import { issuesFor, useConfigStore } from '@/stores/configStore'
import type { TemplateResponse, Waypoint } from '@/types/api'

export function FlightPathPage() {
  const config = useConfigStore((s) => s.config)
  const update = useConfigStore((s) => s.update)
  const validation = useConfigStore((s) => s.validation)
  const [selected, setSelected] = useState<number | null>(null)
  const [templates, setTemplates] = useState<TemplateResponse[] | null>(null)
  const [loadingTemplates, setLoadingTemplates] = useState(false)
  const [templateError, setTemplateError] = useState<string | null>(null)

  const wps = useMemo(() => config?.flight_path.waypoints ?? [], [config])
  const clientCheck = useMemo(() => (config ? flightPathSchema.safeParse(config.flight_path) : null), [config])
  if (!config) return null
  const serverIssues = issuesFor(validation, 'flight_path')

  const setWaypoints = (w: Waypoint[]) => update('flight_path.waypoints', w)
  const loadTemplates = async () => {
    setLoadingTemplates(true)
    setTemplateError(null)
    try {
      setTemplates(await api.pathTemplates(config))
    } catch (e) {
      setTemplateError(e instanceof Error ? e.message : String(e))
    } finally {
      setLoadingTemplates(false)
    }
  }
  const move = (i: number, d: -1 | 1) => {
    const j = i + d
    if (j < 0 || j >= wps.length) return
    const next = [...wps]
    ;[next[i], next[j]] = [next[j] as Waypoint, next[i] as Waypoint]
    setWaypoints(next)
    setSelected(j)
  }

  return (
    <div>
      <PageHeader
        title="Редактор маршруту польоту"
        description="Шляхові точки (ШТ) задають маршрутну ділянку між початковим набором висоти та заходом на посадку. Зліт, захід, глісаду й посадку формує автомат станів автопілота, а не цей список. Клацніть на карті, щоб додати точку; перетягніть точку, щоб перемістити її. Точка зі швидкістю 0 — точка зависання (зупинка, захоплення висоти та, за потреби, курсу)."
        actions={
          <>
            <Button variant="secondary" size="sm" onClick={() => void loadTemplates()} disabled={loadingTemplates}>
              {loadingTemplates ? <Loader2 className="animate-spin" /> : <Wand2 />} Готові маршрути
            </Button>
            <Button variant="ghost" size="sm" onClick={() => setWaypoints([])} disabled={wps.length === 0}>
              <Trash2 /> Очистити
            </Button>
          </>
        }
      />
      {templateError ? <p className="mb-2 text-xs text-critical">{templateError}</p> : null}
      {templates ? (
        <Card className="mb-3">
          <CardHeader>
            <div>
              <CardTitle>Готові маршрути</CardTitle>
              <CardDescription>Згенеровано сервером для поточного місця зльоту, точки посадки та геометрії заходу.</CardDescription>
            </div>
            <Button size="sm" variant="ghost" onClick={() => setTemplates(null)}>
              Закрити
            </Button>
          </CardHeader>
          <CardContent className="grid gap-2 sm:grid-cols-2 xl:grid-cols-3">
            {templates.map((t) => (
              <button
                key={t.template}
                className="rounded-md border border-border bg-background p-2.5 text-left hover:border-primary hover:bg-accent"
                onClick={() => {
                  update('flight_path', { name: t.path.name, waypoints: t.path.waypoints })
                  setSelected(null)
                  setTemplates(null)
                }}
              >
                <div className="text-xs font-medium">{TEMPLATE_UK[t.template].name}</div>
                <div className="mt-0.5 text-[11px] text-muted-foreground">{TEMPLATE_UK[t.template].description}</div>
                <div className="mt-1 text-[11px] text-subtle-foreground">шляхових точок: {t.path.waypoints.length}</div>
              </button>
            ))}
          </CardContent>
        </Card>
      ) : null}

      <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_400px]">
        <div className="space-y-3">
          <TopView
            config={config}
            approach={validation?.approach}
            editable
            selected={selected}
            onSelect={setSelected}
            onAdd={(p) => {
              const prev = wps[wps.length - 1]
              setWaypoints([
                ...wps,
                { x_m: p.x, y_m: p.y, altitude_m: prev?.altitude_m ?? config.autopilot.target_altitude_m, heading_deg: null, speed_m_s: prev?.speed_m_s || config.autopilot.cruise_speed_m_s, tolerance_m: 15 },
              ])
              setSelected(wps.length)
            }}
            onMove={(i, p) => setWaypoints(wps.map((w, k) => (k === i ? { ...w, x_m: p.x, y_m: p.y } : w)))}
            height={460}
          />
          <Card>
            <CardHeader>
              <CardTitle>Шляхові точки ({wps.length})</CardTitle>
            </CardHeader>
            <CardContent className="p-0">
              {wps.length === 0 ? (
                <p className="p-4 text-xs text-muted-foreground">Маршрутних точок немає: після набору висоти автопілот переходить безпосередньо до заходу на посадку.</p>
              ) : (
                <Table>
                  <THead>
                    <Tr>
                      <Th>#</Th>
                      <Th>Схід [м]</Th>
                      <Th>Північ [м]</Th>
                      <Th>Висота [м]</Th>
                      <Th>Швидкість [м/с]</Th>
                      <Th>Курс</Th>
                      <Th>Допуск [м]</Th>
                      <Th />
                    </Tr>
                  </THead>
                  <TBody>
                    {wps.map((w, i) => (
                      <Tr key={i} className={selected === i ? 'bg-accent' : 'cursor-pointer'} onClick={() => setSelected(i)}>
                        <Td className="font-medium">ШТ{i + 1}</Td>
                        <Td>{fmt(w.x_m, 0)}</Td>
                        <Td>{fmt(w.y_m, 0)}</Td>
                        <Td>{fmt(w.altitude_m, 1)}</Td>
                        <Td>{w.speed_m_s < 0.3 ? <Badge variant="outline">зависання</Badge> : fmt(w.speed_m_s, 1)}</Td>
                        <Td>{w.heading_deg === null ? '—' : `${fmt(w.heading_deg, 0)}°`}</Td>
                        <Td>{fmt(w.tolerance_m, 0)}</Td>
                        <Td className="text-right whitespace-nowrap">
                          <Button size="icon-sm" variant="ghost" aria-label="Перемістити вгору" onClick={(e) => (e.stopPropagation(), move(i, -1))}>
                            <ArrowUp />
                          </Button>
                          <Button size="icon-sm" variant="ghost" aria-label="Перемістити вниз" onClick={(e) => (e.stopPropagation(), move(i, 1))}>
                            <ArrowDown />
                          </Button>
                          <Button
                            size="icon-sm"
                            variant="ghost"
                            aria-label={`Видалити ШТ${i + 1}`}
                            onClick={(e) => {
                              e.stopPropagation()
                              setWaypoints(wps.filter((_, k) => k !== i))
                              setSelected(null)
                            }}
                          >
                            <Trash2 />
                          </Button>
                        </Td>
                      </Tr>
                    ))}
                  </TBody>
                </Table>
              )}
            </CardContent>
          </Card>
        </div>
        <div className="space-y-3">
          {selected !== null && wps[selected] ? (
            <WaypointForm
              key={selected}
              index={selected}
              waypoint={wps[selected]}
              onChange={(w) => setWaypoints(wps.map((x, k) => (k === selected ? w : x)))}
            />
          ) : (
            <Card>
              <CardContent className="text-xs text-muted-foreground">Виберіть шляхову точку для редагування.</CardContent>
            </Card>
          )}
          <Card>
            <CardHeader>
              <CardTitle>Перевірки маршруту</CardTitle>
            </CardHeader>
            <CardContent className="space-y-1.5 text-xs">
              {clientCheck && !clientCheck.success
                ? clientCheck.error.issues.map((i) => (
                    <div key={i.message} className="flex gap-2">
                      <Badge variant="critical">помилка</Badge>
                      <span>{i.message}</span>
                    </div>
                  ))
                : null}
              {serverIssues.map((i) => (
                <div key={i.field + i.message} className="flex gap-2">
                  <Badge variant={i.severity === 'error' ? 'critical' : 'warning'}>{i.severity === 'error' ? 'помилка' : 'попередження'}</Badge>
                  <span>
                    <span className="font-mono text-muted-foreground">{i.field.replace('flight_path.', '')}</span> {i.message}
                  </span>
                </div>
              ))}
              {serverIssues.length === 0 && (clientCheck?.success ?? true) ? (
                <p className="text-muted-foreground">Порядок точок, обмеження висоти, радіуси захоплення, градієнти набору та радіуси розворотів здійсненні для заданих обмежень апарата.</p>
              ) : null}
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  )
}

function WaypointForm({ index, waypoint, onChange }: { index: number; waypoint: Waypoint; onChange: (w: Waypoint) => void }) {
  const form = useForm<WaypointInput>({ resolver: zodResolver(waypointSchema), mode: 'onChange', defaultValues: waypoint })
  useEffect(() => {
    form.reset(waypoint)
  }, [waypoint.x_m, waypoint.y_m])
  useEffect(() => {
    const sub = form.watch((v) => {
      const r = waypointSchema.safeParse(v)
      if (r.success) onChange(r.data)
    })
    return () => sub.unsubscribe()
  }, [form, onChange])
  const field = (name: keyof WaypointInput, label: string, unit: string, nullable = false) => {
    const err = form.formState.errors[name]?.message
    return (
      <div className="space-y-1">
        <Label htmlFor={`wp-${name}`}>
          {label} <span className="text-subtle-foreground">[{unit}]</span>
        </Label>
        <Input
          id={`wp-${name}`}
          type="number"
          step="any"
          placeholder={nullable ? 'немає' : undefined}
          aria-invalid={Boolean(err)}
          {...form.register(name, { setValueAs: (v: unknown) => (v === '' || v === null ? (nullable ? null : Number.NaN) : Number(v)) })}
        />
        {err ? <p className="text-[11px] text-critical">{err}</p> : null}
      </div>
    )
  }
  return (
    <Card>
      <CardHeader>
        <CardTitle>Шляхова точка ШТ{index + 1}</CardTitle>
      </CardHeader>
      <CardContent className="grid grid-cols-2 gap-3">
        {field('x_m', 'Схід', 'м')}
        {field('y_m', 'Північ', 'м')}
        {field('altitude_m', 'Цільова висота', 'м над землею')}
        {field('speed_m_s', 'Задана швидкість', 'м/с')}
        {field('heading_deg', 'Цільовий курс', '°', true)}
        {field('tolerance_m', 'Допуск', 'м')}
      </CardContent>
    </Card>
  )
}
