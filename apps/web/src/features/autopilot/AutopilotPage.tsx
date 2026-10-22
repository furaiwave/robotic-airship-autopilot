import { useCallback } from 'react'

import { PageHeader } from '@/components/common/Display'
import { SectionForm } from '@/components/common/SectionForm'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/overlays'
import { Badge, Input, Label, Switch } from '@/components/ui/primitives'
import { Table, TBody, Td, Th, THead, Tr } from '@/components/ui/table'
import { AUTOPILOT_GROUPS, autopilotSchema, SIMULATION_FIELDS, simulationSchema } from '@/lib/schemas'
import { cn } from '@/lib/utils'
import { issuesFor, useConfigStore } from '@/stores/configStore'
import type { AutopilotConfig, ControllerKind, PhaseClass, SimulationSettings } from '@/types/api'

const LOOPS: { key: 'altitude' | 'vertical_speed' | 'heading' | 'yaw_rate' | 'airspeed'; label: string; units: string }[] = [
  { key: 'altitude', label: 'Altitude → vertical-speed ref.', units: '1/s, 1/s², s' },
  { key: 'vertical_speed', label: 'Vertical speed → heave force', units: 'N·s/m, N/m, N·s²/m' },
  { key: 'heading', label: 'Heading → yaw-rate ref.', units: '1/s, 1/s², s' },
  { key: 'yaw_rate', label: 'Yaw rate → yaw moment', units: 'N·m·s, N·m, N·m·s²' },
  { key: 'airspeed', label: 'Airspeed → surge force', units: 'N·s/m, N/m, N·s²/m' },
]

const PHASE_CLASSES: PhaseClass[] = ['ground', 'takeoff', 'climb', 'cruise', 'approach', 'landing']

const STATE_MACHINE: { phase: string; loops: string; exit: string; failure: string }[] = [
  { phase: 'Preflight', loops: '—', exit: 'all checks pass (sensors, vehicle physics, lift margin, descent authority, wind, battery, path)', failure: 'any check fails → Aborted' },
  { phase: 'Take-off preparation', loops: 'open-loop heave ramp; yaw into wind at the mast', exit: 'mooring load ≥ release threshold, wind ≤ limit, hull ≤ 20° off the wind', failure: 'timeout → Aborted (cause reported)' },
  { phase: 'Take-off', loops: 'vertical speed, position hold, heading', exit: 'gear height ≥ clearance and vz > 0.1 m/s', failure: 'timeout → Aborted' },
  { phase: 'Initial climb', loops: 'vertical speed, position hold, heading', exit: 'gear height ≥ initial-climb height', failure: '—' },
  { phase: 'Climb', loops: 'altitude (rate-limited ref.), LOS path, heading, airspeed', exit: '|h − h_target| ≤ tol and |vz| ≤ tol', failure: 'unplanned ground contact → Touchdown' },
  { phase: 'En-route', loops: 'altitude, LOS path, heading, airspeed', exit: 'all waypoints captured', failure: 'battery critical → Approach' },
  { phase: 'Approach', loops: 'altitude, join point / final line, heading, airspeed', exit: 'at the gate: inside corridor, altitude, track error, speed, wind', failure: 'not established at the gate → Go-around' },
  { phase: 'Descent', loops: 'glide-path altitude + vz ff, line, decelerating speed', exit: 'low, within flare radius, slow', failure: 'cross-track, glide-path, wind, overshoot (above DH) → Go-around' },
  { phase: 'Flare', loops: 'vertical speed (sink), position hold', exit: 'weight on gear (contact model)', failure: 'drift > 2× tolerance or wind (above DH) → Go-around' },
  { phase: 'Touchdown', loops: 'open-loop hold-down thrust', exit: 'on ground, slow, |vz| ≤ 0.2 m/s', failure: 'bounce → Flare' },
  { phase: 'Post-landing', loops: 'open-loop hold-down thrust', exit: 'settled for the settle time → mooring engaged', failure: 'lift-off → Flare' },
  { phase: 'Go-around', loops: 'altitude to approach alt., straight ahead, heading, airspeed', exit: 'approach altitude reached → Approach', failure: 'attempts exhausted → Aborted' },
]

export function AutopilotPage() {
  const config = useConfigStore((s) => s.config)
  const update = useConfigStore((s) => s.update)
  const validation = useConfigStore((s) => s.validation)
  const onAutopilot = useCallback((a: AutopilotConfig) => update('autopilot', a), [update])
  const onSim = useCallback((s: SimulationSettings) => update('simulation', s), [update])
  if (!config) return null
  const ctl = config.controller

  const gain = (path: string, value: number, label: string) => (
    <Input
      type="number"
      step="any"
      aria-label={label}
      className="w-24"
      value={value}
      onChange={(e) => {
        const v = Number(e.target.value)
        if (Number.isFinite(v) && v >= 0) update(path, v)
      }}
    />
  )

  return (
    <div>
      <PageHeader
        title="Autopilot configuration"
        description="Both controllers share the same cascaded loop structure, feed-forward terms, wind-triangle guidance and control allocation; they differ only in how the loop gains are obtained."
      />
      <Tabs defaultValue="controller">
        <TabsList>
          <TabsTrigger value="controller">Controller</TabsTrigger>
          <TabsTrigger value="thresholds">Phases & thresholds</TabsTrigger>
          <TabsTrigger value="statemachine">State machine</TabsTrigger>
          <TabsTrigger value="numerics">Numerics</TabsTrigger>
        </TabsList>

        <TabsContent value="controller" className="space-y-3">
          <div className="grid gap-3 lg:grid-cols-2">
            {(['cascaded_pid', 'gain_scheduled'] as ControllerKind[]).map((k) => (
              <button
                key={k}
                onClick={() => update('controller.kind', k)}
                aria-pressed={ctl.kind === k}
                className={cn('rounded-lg border bg-surface p-3 text-left transition-colors', ctl.kind === k ? 'border-primary ring-1 ring-primary' : 'border-border hover:border-border-strong')}
              >
                <div className="flex items-center justify-between">
                  <span className="text-sm font-semibold">{k === 'cascaded_pid' ? 'Controller A — cascaded PID' : 'Controller B — gain-scheduled PID'}</span>
                  {ctl.kind === k ? <Badge variant="info">selected</Badge> : null}
                </div>
                <p className="mt-1 text-xs text-muted-foreground">
                  {k === 'cascaded_pid'
                    ? 'Fixed gains for all phases and speeds. Defaults come from pole placement at one nominal cruise point (4 m/s airspeed, cruise bandwidths) and are then held constant.'
                    : 'Gains recomputed every control step: Kp = max(2ζωM − d(σ), f·2ζωM), Ki = ω²M, where d(σ) is the linearised aerodynamic damping at the measured airspeed / vertical speed / yaw rate, ω comes from the phase table and the position gain is scheduled on distance to the landing point.'}
                </p>
              </button>
            ))}
          </div>

          <div className="grid gap-3 xl:grid-cols-2">
            <Card>
              <CardHeader>
                <div>
                  <CardTitle>Controller A gains</CardTitle>
                  <CardDescription>PID with anti-windup (conditional integration), filtered derivative on measurement, saturation and rate limits.</CardDescription>
                </div>
              </CardHeader>
              <CardContent className="p-0">
                <Table>
                  <THead>
                    <Tr>
                      <Th>Loop</Th>
                      <Th>Kp</Th>
                      <Th>Ki</Th>
                      <Th>Kd</Th>
                    </Tr>
                  </THead>
                  <TBody>
                    {LOOPS.map((l) => (
                      <Tr key={l.key}>
                        <Td>
                          <div>{l.label}</div>
                          <div className="text-[10px] text-subtle-foreground">{l.units}</div>
                        </Td>
                        {(['kp', 'ki', 'kd'] as const).map((g) => (
                          <Td key={g}>{gain(`controller.cascaded_pid.${l.key}.${g}`, ctl.cascaded_pid[l.key][g], `${l.label} ${g}`)}</Td>
                        ))}
                      </Tr>
                    ))}
                    <Tr>
                      <Td>Position → ground velocity [1/s]</Td>
                      <Td>{gain('controller.cascaded_pid.position_gain_1_s', ctl.cascaded_pid.position_gain_1_s, 'Position gain')}</Td>
                      <Td />
                      <Td />
                    </Tr>
                  </TBody>
                </Table>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <div>
                  <CardTitle>Controller B schedule</CardTitle>
                  <CardDescription>Closed-loop bandwidth ω [rad/s] per phase class; outer loops use ω / n_outer.</CardDescription>
                </div>
              </CardHeader>
              <CardContent className="space-y-3 p-0">
                <Table>
                  <THead>
                    <Tr>
                      <Th>Phase class</Th>
                      <Th>ω vertical</Th>
                      <Th>ω yaw</Th>
                      <Th>ω speed</Th>
                    </Tr>
                  </THead>
                  <TBody>
                    {PHASE_CLASSES.map((p) => (
                      <Tr key={p}>
                        <Td className="capitalize">{p}</Td>
                        {(['vertical_rad_s', 'yaw_rad_s', 'speed_rad_s'] as const).map((b) => (
                          <Td key={b}>{gain(`controller.gain_scheduled.bandwidths.${p}.${b}`, ctl.gain_scheduled.bandwidths[p][b], `${p} ${b}`)}</Td>
                        ))}
                      </Tr>
                    ))}
                  </TBody>
                </Table>
                <div className="grid grid-cols-2 gap-3 px-4 pb-4">
                  {(
                    [
                      ['damping_ratio', 'Damping ratio ζ'],
                      ['outer_loop_ratio', 'Outer-loop ratio n'],
                      ['min_proportional_fraction', 'Min Kp fraction f'],
                      ['position_gain_far_1_s', 'Position gain far [1/s]'],
                      ['position_gain_near_1_s', 'Position gain near [1/s]'],
                      ['position_schedule_distance_m', 'Schedule distance [m]'],
                    ] as const
                  ).map(([k, label]) => (
                    <div key={k} className="space-y-1">
                      <Label>{label}</Label>
                      {gain(`controller.gain_scheduled.${k}`, ctl.gain_scheduled[k], label)}
                    </div>
                  ))}
                </div>
              </CardContent>
            </Card>
          </div>
          <Card>
            <CardHeader>
              <CardTitle>Shared settings</CardTitle>
            </CardHeader>
            <CardContent className="grid gap-3 sm:grid-cols-3">
              {(
                [
                  ['derivative_filter_tau_s', 'Derivative filter τ [s]'],
                  ['heave_rate_limit_n_s', 'Heave rate limit [N/s]'],
                  ['surge_rate_limit_n_s', 'Surge rate limit [N/s]'],
                  ['yaw_moment_rate_limit_nm_s', 'Yaw-moment rate limit [N·m/s]'],
                  ['max_reverse_airspeed_m_s', 'Max reverse airspeed [m/s]'],
                ] as const
              ).map(([k, label]) => (
                <div key={k} className="space-y-1">
                  <Label>{label}</Label>
                  {gain(`controller.common.${k}`, ctl.common[k], label)}
                </div>
              ))}
              <label className="flex items-center gap-2 self-end text-xs text-muted-foreground">
                <Switch checked={ctl.common.drag_feedforward} onCheckedChange={(v) => update('controller.common.drag_feedforward', v)} aria-label="Drag feed-forward" />
                Drag and static-heaviness feed-forward
              </label>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="thresholds">
          <SectionForm value={config.autopilot} groups={AUTOPILOT_GROUPS} schema={autopilotSchema} onChange={onAutopilot} issuePrefix="autopilot" serverIssues={issuesFor(validation, 'autopilot')} columns={3} />
        </TabsContent>

        <TabsContent value="statemachine">
          <Card>
            <CardHeader>
              <div>
                <CardTitle>Explicit flight-phase state machine</CardTitle>
                <CardDescription>
                  Every transition depends on measured state, targets and safety constraints; timers only bound failure handling. Each transition is logged with its time and the measured values that
                  satisfied the condition.
                </CardDescription>
              </div>
            </CardHeader>
            <CardContent className="p-0">
              <Table>
                <THead>
                  <Tr>
                    <Th>Phase</Th>
                    <Th>Active loops</Th>
                    <Th>Nominal exit condition</Th>
                    <Th>Failure / abort handling</Th>
                  </Tr>
                </THead>
                <TBody>
                  {STATE_MACHINE.map((r) => (
                    <Tr key={r.phase}>
                      <Td className="font-medium whitespace-nowrap">{r.phase}</Td>
                      <Td className="text-muted-foreground">{r.loops}</Td>
                      <Td>{r.exit}</Td>
                      <Td className="text-muted-foreground">{r.failure}</Td>
                    </Tr>
                  ))}
                </TBody>
              </Table>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="numerics">
          <SectionForm
            value={config.simulation}
            groups={[{ title: 'Fixed-step integration', description: 'Classical RK4 at dt; the controller runs every 1/control-rate seconds; the replay speed never changes dt.', fields: SIMULATION_FIELDS }]}
            schema={simulationSchema}
            onChange={onSim}
            issuePrefix="simulation"
            serverIssues={issuesFor(validation, 'simulation')}
          />
        </TabsContent>
      </Tabs>
    </div>
  )
}
