import { Plus, Trash2 } from 'lucide-react'
import { useCallback } from 'react'

import { PageHeader } from '@/components/common/Display'
import { TopView } from '@/components/common/FlightViews'
import { SectionForm } from '@/components/common/SectionForm'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Select } from '@/components/ui/overlays'
import { Input, Label, Switch } from '@/components/ui/primitives'
import { Table, TBody, Td, Th, THead, Tr } from '@/components/ui/table'
import { fmt } from '@/lib/format'
import { ENVIRONMENT_GROUPS, environmentSchema } from '@/lib/schemas'
import { issuesFor, useConfigStore } from '@/stores/configStore'
import type { DisturbanceTrigger, EnvironmentConfig } from '@/types/api'

export function EnvironmentPage() {
  const config = useConfigStore((s) => s.config)
  const update = useConfigStore((s) => s.update)
  const scenarios = useConfigStore((s) => s.scenarios)
  const applyScenario = useConfigStore((s) => s.applyScenario)
  const validation = useConfigStore((s) => s.validation)
  const onChange = useCallback((e: EnvironmentConfig) => update('environment', e), [update])
  if (!config) return null
  const env = config.environment
  const issues = issuesFor(validation, 'environment')

  return (
    <div>
      <PageHeader
        title="Environment configuration"
        description="Flat terrain, ISA or constant atmosphere, and a deterministic wind model (mean wind with power-law shear, scheduled wind changes, 1-cosine gusts, seeded Gauss–Markov turbulence). This is a test-harness disturbance model, not a validated meteorological model."
      />
      <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_420px]">
        <div className="space-y-3">
          <Card>
            <CardHeader>
              <div>
                <CardTitle>Scenario presets</CardTitle>
                <CardDescription>Applies wind, turbulence, noise and landing area; keeps sites and initial state.</CardDescription>
              </div>
            </CardHeader>
            <CardContent className="grid gap-2 sm:grid-cols-2 xl:grid-cols-3">
              {scenarios.map((s) => (
                <button
                  key={s.id}
                  onClick={() => applyScenario(s.id)}
                  className="rounded-md border border-border bg-background p-2.5 text-left transition-colors hover:border-primary hover:bg-accent"
                >
                  <div className="text-xs font-medium">{s.name}</div>
                  <div className="mt-0.5 line-clamp-3 text-[11px] text-muted-foreground">{s.description}</div>
                </button>
              ))}
            </CardContent>
          </Card>

          <Card>
            <CardContent className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
              <div className="space-y-1">
                <Label>Atmosphere model</Label>
                <Select
                  ariaLabel="Atmosphere model"
                  value={env.atmosphere.model}
                  onValueChange={(v) => update('environment.atmosphere.model', v)}
                  options={[
                    { value: 'isa', label: 'ISA troposphere' },
                    { value: 'constant', label: 'Constant density' },
                  ]}
                />
              </div>
              <Toggle label="Start moored at mast" checked={env.initial_state.moored} onChange={(v) => update('environment.initial_state.moored', v)} />
              <Toggle label="Turbulence" checked={env.wind.turbulence.enabled} onChange={(v) => update('environment.wind.turbulence.enabled', v)} />
              <Toggle label="Sensor noise" checked={env.sensor_noise.enabled} onChange={(v) => update('environment.sensor_noise.enabled', v)} />
            </CardContent>
          </Card>

          <SectionForm value={env} groups={ENVIRONMENT_GROUPS} schema={environmentSchema} onChange={onChange} issuePrefix="environment" serverIssues={issues} columns={2} />
          <DisturbanceEditor env={env} update={update} />
        </div>
        <div className="space-y-3 xl:sticky xl:top-0 xl:self-start">
          <Card>
            <CardHeader>
              <CardTitle>Mission geometry</CardTitle>
            </CardHeader>
            <CardContent>
              <TopView config={config} approach={validation?.approach} height={360} />
              <p className="mt-2 text-[11px] text-muted-foreground">
                Ground density {fmt(validation?.ground_density_kg_m3 ?? null, 4)} kg/m³ at {fmt(env.ground_elevation_m, 0)} m MSL. Dashed: planned route and final approach; green: landing area and
                landing-point tolerance.
              </p>
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  )
}

function Toggle({ label, checked, onChange }: { label: string; checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <label className="flex items-center justify-between gap-2 rounded-md border border-border px-2.5 py-1.5 text-xs">
      <span className="text-muted-foreground">{label}</span>
      <Switch checked={checked} onCheckedChange={onChange} aria-label={label} />
    </label>
  )
}

function TriggerCell({ trigger, onChange }: { trigger: DisturbanceTrigger; onChange: (t: DisturbanceTrigger) => void }) {
  const value = trigger.kind === 'at_time' ? trigger.time_s : trigger.distance_m
  return (
    <div className="flex items-center gap-1">
      <Select
        ariaLabel="Trigger"
        className="w-32"
        value={trigger.kind}
        onValueChange={(k) => onChange(k === 'at_time' ? { kind: 'at_time', time_s: value } : { kind: 'distance_to_landing', distance_m: value })}
        options={[
          { value: 'at_time', label: 'at time [s]' },
          { value: 'distance_to_landing', label: 'within [m] of LP' },
        ]}
      />
      <Input
        type="number"
        className="w-20"
        aria-label="Trigger value"
        value={value}
        onChange={(e) => {
          const v = Number(e.target.value)
          onChange(trigger.kind === 'at_time' ? { kind: 'at_time', time_s: v } : { kind: 'distance_to_landing', distance_m: v })
        }}
      />
    </div>
  )
}

function DisturbanceEditor({ env, update }: { env: EnvironmentConfig; update: (p: string, v: unknown) => void }) {
  const num = (path: string, v: number, label: string) => (
    <Input type="number" aria-label={label} className="w-20" value={v} onChange={(e) => update(path, Number(e.target.value))} />
  )
  const changes = env.wind.changes
  const gusts = env.wind.gusts
  return (
    <Card>
      <CardHeader>
        <div>
          <CardTitle>Wind changes and discrete gusts</CardTitle>
          <CardDescription>
            Triggers fire once, at a simulation time or when the vehicle first comes within a distance of the landing point (LP). Distance triggers place the disturbance at the same point of the
            manoeuvre for every controller.
          </CardDescription>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        <div>
          <div className="mb-1 flex items-center justify-between">
            <span className="text-xs font-medium">Wind changes (linear ramp)</span>
            <Button
              size="sm"
              variant="secondary"
              onClick={() =>
                update('environment.wind.changes', [
                  ...changes,
                  { trigger: { kind: 'distance_to_landing', distance_m: 250 }, ramp_duration_s: 20, speed_m_s: 5, from_direction_deg: 220, vertical_m_s: 0 },
                ])
              }
            >
              <Plus /> Add change
            </Button>
          </div>
          {changes.length === 0 ? (
            <p className="text-[11px] text-muted-foreground">No wind changes.</p>
          ) : (
            <Table>
              <THead>
                <Tr>
                  <Th>Trigger</Th>
                  <Th>Ramp [s]</Th>
                  <Th>Speed [m/s]</Th>
                  <Th>From [deg]</Th>
                  <Th>Vertical [m/s]</Th>
                  <Th />
                </Tr>
              </THead>
              <TBody>
                {changes.map((c, i) => (
                  <Tr key={i}>
                    <Td>
                      <TriggerCell trigger={c.trigger} onChange={(t) => update(`environment.wind.changes.${i}.trigger`, t)} />
                    </Td>
                    <Td>{num(`environment.wind.changes.${i}.ramp_duration_s`, c.ramp_duration_s, 'Ramp duration')}</Td>
                    <Td>{num(`environment.wind.changes.${i}.speed_m_s`, c.speed_m_s, 'Speed')}</Td>
                    <Td>{num(`environment.wind.changes.${i}.from_direction_deg`, c.from_direction_deg, 'Direction')}</Td>
                    <Td>{num(`environment.wind.changes.${i}.vertical_m_s`, c.vertical_m_s, 'Vertical')}</Td>
                    <Td>
                      <Button size="icon-sm" variant="ghost" aria-label="Remove change" onClick={() => update('environment.wind.changes', changes.filter((_, k) => k !== i))}>
                        <Trash2 />
                      </Button>
                    </Td>
                  </Tr>
                ))}
              </TBody>
            </Table>
          )}
        </div>
        <div>
          <div className="mb-1 flex items-center justify-between">
            <span className="text-xs font-medium">1-cosine gusts</span>
            <Button
              size="sm"
              variant="secondary"
              onClick={() =>
                update('environment.wind.gusts', [...gusts, { trigger: { kind: 'at_time', time_s: 60 }, duration_s: 10, speed_m_s: 4, from_direction_deg: 180, vertical_m_s: 0 }])
              }
            >
              <Plus /> Add gust
            </Button>
          </div>
          {gusts.length === 0 ? (
            <p className="text-[11px] text-muted-foreground">No discrete gusts.</p>
          ) : (
            <Table>
              <THead>
                <Tr>
                  <Th>Trigger</Th>
                  <Th>Duration [s]</Th>
                  <Th>Peak [m/s]</Th>
                  <Th>From [deg]</Th>
                  <Th>Vertical [m/s]</Th>
                  <Th />
                </Tr>
              </THead>
              <TBody>
                {gusts.map((g, i) => (
                  <Tr key={i}>
                    <Td>
                      <TriggerCell trigger={g.trigger} onChange={(t) => update(`environment.wind.gusts.${i}.trigger`, t)} />
                    </Td>
                    <Td>{num(`environment.wind.gusts.${i}.duration_s`, g.duration_s, 'Duration')}</Td>
                    <Td>{num(`environment.wind.gusts.${i}.speed_m_s`, g.speed_m_s, 'Peak speed')}</Td>
                    <Td>{num(`environment.wind.gusts.${i}.from_direction_deg`, g.from_direction_deg, 'Direction')}</Td>
                    <Td>{num(`environment.wind.gusts.${i}.vertical_m_s`, g.vertical_m_s, 'Vertical')}</Td>
                    <Td>
                      <Button size="icon-sm" variant="ghost" aria-label="Remove gust" onClick={() => update('environment.wind.gusts', gusts.filter((_, k) => k !== i))}>
                        <Trash2 />
                      </Button>
                    </Td>
                  </Tr>
                ))}
              </TBody>
            </Table>
          )}
        </div>
      </CardContent>
    </Card>
  )
}
