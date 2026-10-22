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
import type { AirshipParams, DerivedProperties } from '@/types/api'

import { AirshipSchematic } from './AirshipSchematic'

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
        title="Airship configuration"
        description="Non-rigid research blimp: ellipsoidal helium envelope, two reversible main thrusters on outriggers (surge and differential yaw), one reversible heave thruster through the CG, aerodynamic rudder on the tail fins. Pitch and roll are passively stable (CG below CV); no elevator is modelled."
        actions={
          <Button variant="secondary" size="sm" onClick={() => defaults && update('airship', structuredClone(defaults.airship))}>
            <RotateCcw /> Default vehicle
          </Button>
        }
      />
      <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_380px]">
        <div className="space-y-3">
          <Card>
            <CardContent className="grid gap-3 sm:grid-cols-2">
              <div className="space-y-1">
                <Label htmlFor="airship-name">Vehicle name</Label>
                <Input id="airship-name" value={a.name} onChange={(e) => update('airship.name', e.target.value)} />
              </div>
              <div className="space-y-1">
                <Label>Lifting gas</Label>
                <Select
                  ariaLabel="Lifting gas"
                  value={a.lifting_gas}
                  onValueChange={(v) => update('airship.lifting_gas', v)}
                  options={[
                    { value: 'helium', label: 'Helium (ρ_gas/ρ_air = 0.138)' },
                    { value: 'hydrogen', label: 'Hydrogen (ρ_gas/ρ_air = 0.070)' },
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
              <CardTitle>Schematic (to scale)</CardTitle>
            </CardHeader>
            <CardContent>
              <AirshipSchematic p={a} />
            </CardContent>
          </Card>
          {d ? <DerivedPanel d={d} /> : <Card><CardContent className="text-xs text-muted-foreground">Derived properties appear once the vehicle parameters are valid.</CardContent></Card>}
          {issues.length > 0 ? (
            <Card>
              <CardHeader>
                <CardTitle>Engine validation</CardTitle>
              </CardHeader>
              <CardContent className="space-y-1.5 text-xs">
                {issues.map((i) => (
                  <div key={i.field + i.message} className="flex gap-2">
                    <Badge variant={i.severity === 'error' ? 'critical' : 'warning'}>{i.severity}</Badge>
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
          <CardTitle>Derived quantities</CardTitle>
          <CardDescription>Computed by the Rust model at ground density {fmt(d.reference_density_kg_m3, 4)} kg/m³</CardDescription>
        </div>
        <Badge variant={stateVariant}>statically {d.buoyancy_state}</Badge>
      </CardHeader>
      <CardContent className="space-y-3">
        <KeyValue
          items={[
            [`Hull volume${d.hull_volume_is_estimate ? ' (ellipsoid est.)' : ''}`, `${fmt(d.hull_volume_m3, 2)} m³`],
            ['Displaced volume V', `${fmt(d.displaced_volume_m3, 2)} m³`],
            ['Buoyancy ρVg', `${fmt(d.buoyancy_n, 1)} N`],
            ['Weight mg', `${fmt(d.weight_n, 1)} N`],
            ['Static lift B − W', `${fmtSigned(d.static_lift_n, 1)} N`],
            ['Buoyancy ratio B/W', fmt(d.buoyancy_ratio, 4)],
            ['Lifting-gas mass (est.)', `${fmt(d.gas_mass_estimate_kg, 2)} kg`],
          ]}
        />
        <div className="border-t border-border pt-2">
          <KeyValue
            items={[
              ['Fineness L/D', fmt(d.fineness_ratio, 2)],
              ['Frontal / side area', `${fmt(d.frontal_area_m2, 2)} / ${fmt(d.lateral_area_m2, 2)} m²`],
              ['Lamb k1 / k2 / k′', `${fmt(d.lamb_k1, 3)} / ${fmt(d.lamb_k2, 3)} / ${fmt(d.lamb_k_rot, 3)}`],
              ['Eff. mass axial / transv.', `${fmt(d.effective_mass_axial_kg, 1)} / ${fmt(d.effective_mass_vertical_kg, 1)} kg`],
              ['Eff. yaw inertia (est.)', `${fmt(d.effective_inertia_pitch_yaw_kg_m2, 0)} kg·m²`],
              ['Pitch natural freq.', `${fmt(d.pitch_natural_frequency_rad_s, 2)} rad/s`],
            ]}
          />
        </div>
        <div className="border-t border-border pt-2">
          <KeyValue
            items={[
              ['Max forward / reverse thrust', `${fmt(d.max_forward_thrust_n, 0)} / ${fmt(d.max_reverse_thrust_n, 0)} N`],
              ['Heave up / down', `${fmt(d.max_heave_up_n, 0)} / ${fmt(d.max_heave_down_n, 0)} N`],
              ['Diff.-thrust yaw moment', `${fmt(d.max_differential_yaw_moment_nm, 0)} N·m`],
              ['Rudder = diff. thrust at', `${fmt(d.rudder_crossover_airspeed_m_s, 1)} m/s`],
              ['Max level speed (est.)', `${fmt(d.estimated_max_level_speed_m_s, 1)} m/s`],
              ['Max climb / descent (est.)', `${fmt(d.estimated_max_climb_rate_m_s, 2)} / ${fmt(d.estimated_max_descent_rate_m_s, 2)} m/s`],
              ['Hover heave thrust / power', `${fmtSigned(d.hover_heave_thrust_n, 1)} N / ${fmt(d.hover_power_w, 0)} W`],
            ]}
          />
        </div>
      </CardContent>
    </Card>
  )
}
