// Fixtures generated from the real Rust engine (see README, "Testing").
import type { ComparisonReport, Experiment, SessionStatus, SimulationConfig, ValidationResponse } from '@/types/api'

import comparison from './fixtures/comparison-d.json'
import defaults from './fixtures/default-config.json'

export function defaultConfig(): SimulationConfig {
  return structuredClone(defaults) as unknown as SimulationConfig
}

export function comparisonReport(): ComparisonReport {
  return structuredClone(comparison) as unknown as ComparisonReport
}

export function validResponse(): ValidationResponse {
  return { valid: true, issues: [], derived: comparisonReport().runs[0]!.results.derived, ground_density_kg_m3: 1.2109, approach: null }
}

export function experimentsList(): Experiment[] {
  const c = defaultConfig()
  return [
    { id: 'D', title: 'Approach and landing in calm air', objective: 'Evaluate landing.', description: 'Approach-only start.', focus_metrics: ['landing_point_error_m'], runs: [{ label: 'Cascaded PID (A)', config: c }] },
  ]
}

export function sessionStatus(over: Partial<SessionStatus> = {}): SessionStatus {
  const t = comparisonReport().runs[0]!.telemetry
  return {
    id: '11111111-2222-3333-4444-555555555555',
    name: 'Reference mission',
    controller: 'cascaded_pid',
    state: 'running',
    speed: 5,
    created_at_ms: 0,
    progress: 0.3,
    time_s: 50,
    duration_s: 900,
    steps: 5000,
    phase: 'approach',
    guidance_mode: 'approach: final-approach line',
    finished: null,
    finish_message: null,
    telemetry_count: t.length,
    transitions: 2,
    safety_events: 0,
    wall_time_ms: 10,
    latest: t[10] ?? null,
    last_transition: null,
    recent_events: [],
    ...over,
  }
}
