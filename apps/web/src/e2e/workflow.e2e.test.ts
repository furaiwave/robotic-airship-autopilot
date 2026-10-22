// End-to-end workflow against the running Rust API, using the frontend's own client:
// configure airship -> configure environment -> flight path -> select autopilot ->
// run take-off, path following, approach and landing -> evaluate -> compare -> export.
import { describe, expect, it } from 'vitest'

import { createApiClient } from '@/services/api'
import type { SimulationConfig } from '@/types/api'

const api = createApiClient({ baseUrl: process.env.E2E_API_URL ?? 'http://127.0.0.1:8080' })

async function waitFinished(id: string) {
  for (let i = 0; i < 600; i++) {
    const s = await api.status(id)
    if (s.state === 'finished') return s
    await new Promise((r) => setTimeout(r, 100))
  }
  throw new Error('simulation did not finish')
}

describe('frontend-to-Rust workflow', () => {
  it('runs the complete research workflow', async () => {
    expect((await api.health()).status).toBe('ok')
    const config: SimulationConfig = await api.defaultConfig()

    // Configure the airship (slightly heavier) and the environment (constant wind scenario).
    config.airship.total_mass_kg = 70.5
    const scenario = (await api.scenarios()).find((s) => s.id === 'constant_wind')
    expect(scenario).toBeDefined()
    config.environment.wind = scenario!.environment.wind
    // Flight path from a backend template; autopilot: controller B.
    const templates = await api.pathTemplates(config)
    config.flight_path = templates.find((t) => t.template === 'climb_and_level')!.path
    config.controller.kind = 'gain_scheduled'

    const v = await api.validate(config)
    expect(v.valid).toBe(true)
    expect(v.derived?.buoyancy_state).toBe('heavy')

    // Invalid configurations are rejected by the backend independently of the client.
    const bad = structuredClone(config)
    bad.simulation.dt_s = 0
    await expect(api.createSimulation(bad)).rejects.toMatchObject({ status: 422 })

    const session = await api.createSimulation(config, 0)
    await api.start(session.id)
    const done = await waitFinished(session.id)
    expect(done.finished).toBe('landed')

    const results = await api.results(session.id)
    expect(results.landing.classification).toBe('successful')
    const phases = results.transitions.map((t) => t.to)
    for (const p of ['takeoff', 'climb', 'cruise', 'approach', 'descent', 'flare', 'touchdown', 'landed'] as const) expect(phases).toContain(p)
    expect(results.metrics.find((m) => m.key === 'landing_point_error_m')?.value).toBeLessThanOrEqual(config.autopilot.landing.landing_point_tolerance_m)

    const tel = await api.telemetry(session.id, 0, 200)
    expect(tel.samples.length).toBeGreaterThan(50)
    const csv = await (await api.exportCsv(session.id)).text()
    expect(csv.split('\n')[1]).toMatch(/^t_s,phase,/)
    const json = JSON.parse(await (await api.exportJson(session.id)).text())
    expect(json.results.config_hash).toBe(results.config_hash)

    // Repeatability: a reset run with the same configuration reproduces the metrics.
    await api.reset(session.id)
    await api.start(session.id, 0)
    await waitFinished(session.id)
    const again = await api.results(session.id)
    expect(again.metrics.find((m) => m.key === 'mission_duration_s')?.value).toBe(results.metrics.find((m) => m.key === 'mission_duration_s')?.value)

    // Controller comparison under identical conditions.
    const a = structuredClone(config)
    a.controller.kind = 'cascaded_pid'
    const cmp = await api.compare([
      { label: 'A', config: a },
      { label: 'B', config },
    ], 300)
    expect(cmp.identical_conditions).toBe(true)
    expect(cmp.runs).toHaveLength(2)
    await api.deleteSimulation(session.id)
  })
})
