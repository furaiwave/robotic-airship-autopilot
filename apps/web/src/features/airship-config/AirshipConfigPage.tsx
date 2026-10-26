import { RotateCcw } from 'lucide-react'
import { useCallback } from 'react'

import { KeyValue, PageHeader } from '@/components/common/Display'
import { SectionForm } from '@/components/common/SectionForm'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Select } from '@/components/ui/overlays'
import { Badge, Input, Label } from '@/components/ui/primitives'
import { fmt, fmtSigned } from '@/lib/format'
import { AIRSHIP_GROUPS, airshipSchema } from '@/lib/schemas'
import { issuesFor, useConfigStore } from '@/stores/configStore'
import type { AirshipParams, BuoyancyState, DerivedProperties } from '@/types/api'

import { AirshipSchematic } from './AirshipSchematic'

const BUOYANCY_LABEL: Record<BuoyancyState, string> = {
  light: 'статично легкий',
  neutral: 'статично нейтральний',
  heavy: 'статично важкий',
}

export function AirshipConfigPage() {
  const config = useConfigStore((s) => s.config)
  const update = useConfigStore((s) => s.update)
  const validation = useConfigStore((s) => s.validation)
  const defaults = useConfigStore((s) => s.defaults)
  const onChange = useCallback((a: AirshipParams) => update('airship', a), [update])
  if (!config) return null
  const a = config.airship
  const d = validation?.derived ?? null
  const issues = issuesFor(validation, 'airship')

  return (
    <div>
      <PageHeader
        title="Конфігурація дирижабля"
        description="М’який дослідницький дирижабль: еліпсоїдна гелієва оболонка, два реверсивні маршові рушії на виносних консолях (поздовжня тяга та диференційне рискання), один реверсивний вертикальний рушій через центр мас, аеродинамічне кермо напрямку на хвостовому оперенні. Тангаж і крен пасивно стійкі (ЦМ нижче ЦО); руль висоти не моделюється."
        actions={
          <Button variant="secondary" size="sm" onClick={() => defaults && update('airship', structuredClone(defaults.airship))}>
            <RotateCcw /> Типовий апарат
          </Button>
        }
      />
      <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_380px]">
        <div className="space-y-3">
          <Card>
            <CardContent className="grid gap-3 sm:grid-cols-2">
              <div className="space-y-1">
                <Label htmlFor="airship-name">Назва апарата</Label>
                <Input id="airship-name" value={a.name} onChange={(e) => update('airship.name', e.target.value)} />
              </div>
              <div className="space-y-1">
                <Label>Несучий газ</Label>
                <Select
                  ariaLabel="Несучий газ"
                  value={a.lifting_gas}
                  onValueChange={(v) => update('airship.lifting_gas', v)}
                  options={[
                    { value: 'helium', label: 'Гелій (ρ_газу/ρ_повітря = 0,138)' },
                    { value: 'hydrogen', label: 'Водень (ρ_газу/ρ_повітря = 0,070)' },
                  ]}
                />
              </div>
            </CardContent>
          </Card>
          <SectionForm value={a} groups={AIRSHIP_GROUPS} schema={airshipSchema} onChange={onChange} issuePrefix="airship" serverIssues={issues} />
        </div>
        <div className="space-y-3">
          <Card>
            <CardHeader>
              <CardTitle>Схема (у масштабі)</CardTitle>
            </CardHeader>
            <CardContent>
              <AirshipSchematic p={a} />
            </CardContent>
          </Card>
          {d ? (
            <DerivedPanel d={d} />
          ) : (
            <Card>
              <CardContent className="text-xs text-muted-foreground">Похідні величини з’являться, коли параметри апарата стануть коректними.</CardContent>
            </Card>
          )}
          {issues.length > 0 ? (
            <Card>
              <CardHeader>
                <CardTitle>Перевірка рушієм моделювання</CardTitle>
              </CardHeader>
              <CardContent className="space-y-1.5 text-xs">
                {issues.map((i) => (
                  <div key={i.field + i.message} className="flex gap-2">
                    <Badge variant={i.severity === 'error' ? 'critical' : 'warning'}>{i.severity === 'error' ? 'помилка' : 'попередження'}</Badge>
                    <span>
                      <span className="font-mono text-muted-foreground">{i.field.replace('airship.', '')}</span> {i.message}
                    </span>
                  </div>
                ))}
              </CardContent>
            </Card>
          ) : null}
        </div>
      </div>
    </div>
  )
}

function DerivedPanel({ d }: { d: DerivedProperties }) {
  const stateVariant = d.buoyancy_state === 'neutral' ? 'info' : d.buoyancy_state === 'light' ? 'good' : 'warning'
  return (
    <Card>
      <CardHeader>
        <div>
          <CardTitle>Похідні величини</CardTitle>
          <CardDescription>Обчислено моделлю на Rust за густини біля землі {fmt(d.reference_density_kg_m3, 4)} кг/м³</CardDescription>
        </div>
        <Badge variant={stateVariant}>{BUOYANCY_LABEL[d.buoyancy_state]}</Badge>
      </CardHeader>
      <CardContent className="space-y-3">
        <KeyValue
          items={[
            [`Об’єм оболонки${d.hull_volume_is_estimate ? ' (оцінка еліпсоїдом)' : ''}`, `${fmt(d.hull_volume_m3, 2)} м³`],
            ['Витіснений об’єм V', `${fmt(d.displaced_volume_m3, 2)} м³`],
            ['Архімедова сила ρVg', `${fmt(d.buoyancy_n, 1)} Н`],
            ['Вага mg', `${fmt(d.weight_n, 1)} Н`],
            ['Статична підйомна сила B − W', `${fmtSigned(d.static_lift_n, 1)} Н`],
            ['Відношення B/W', fmt(d.buoyancy_ratio, 4)],
            ['Маса несучого газу (оцінка)', `${fmt(d.gas_mass_estimate_kg, 2)} кг`],
          ]}
        />
        <div className="border-t border-border pt-2">
          <KeyValue
            items={[
              ['Подовження L/D', fmt(d.fineness_ratio, 2)],
              ['Лобова / бічна площа', `${fmt(d.frontal_area_m2, 2)} / ${fmt(d.lateral_area_m2, 2)} м²`],
              ['Коефіцієнти Лемба k1 / k2 / k′', `${fmt(d.lamb_k1, 3)} / ${fmt(d.lamb_k2, 3)} / ${fmt(d.lamb_k_rot, 3)}`],
              ['Ефект. маса поздовжня / поперечна', `${fmt(d.effective_mass_axial_kg, 1)} / ${fmt(d.effective_mass_vertical_kg, 1)} кг`],
              ['Ефект. момент інерції рискання (оцінка)', `${fmt(d.effective_inertia_pitch_yaw_kg_m2, 0)} кг·м²`],
              ['Власна частота тангажу', `${fmt(d.pitch_natural_frequency_rad_s, 2)} рад/с`],
            ]}
          />
        </div>
        <div className="border-t border-border pt-2">
          <KeyValue
            items={[
              ['Макс. тяга вперед / реверс', `${fmt(d.max_forward_thrust_n, 0)} / ${fmt(d.max_reverse_thrust_n, 0)} Н`],
              ['Вертикальна тяга вгору / вниз', `${fmt(d.max_heave_up_n, 0)} / ${fmt(d.max_heave_down_n, 0)} Н`],
              ['Момент рискання від диф. тяги', `${fmt(d.max_differential_yaw_moment_nm, 0)} Н·м`],
              ['Кермо = диф. тязі при', `${fmt(d.rudder_crossover_airspeed_m_s, 1)} м/с`],
              ['Макс. швидкість горизонт. польоту (оцінка)', `${fmt(d.estimated_max_level_speed_m_s, 1)} м/с`],
              ['Макс. набір / зниження (оцінка)', `${fmt(d.estimated_max_climb_rate_m_s, 2)} / ${fmt(d.estimated_max_descent_rate_m_s, 2)} м/с`],
              ['Тяга / потужність зависання', `${fmtSigned(d.hover_heave_thrust_n, 1)} Н / ${fmt(d.hover_power_w, 0)} Вт`],
            ]}
          />
        </div>
      </CardContent>
    </Card>
  )
}
