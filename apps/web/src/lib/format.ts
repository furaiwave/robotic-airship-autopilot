import type { FlightPhase, LandingClass, SafetySeverity } from '@/types/api'

export function fmt(v: number | null | undefined, digits = 1): string {
  if (v === null || v === undefined || !Number.isFinite(v)) return '—'
  return v.toFixed(digits)
}

export function fmtSigned(v: number | null | undefined, digits = 1): string {
  if (v === null || v === undefined || !Number.isFinite(v)) return '—'
  return `${v >= 0 ? '+' : ''}${v.toFixed(digits)}`
}

export function fmtTime(s: number | null | undefined): string {
  if (s === null || s === undefined || !Number.isFinite(s)) return '—'
  const m = Math.floor(s / 60)
  const sec = s - m * 60
  return `${String(m).padStart(2, '0')}:${sec.toFixed(1).padStart(4, '0')}`
}

export const PHASE_LABEL: Record<FlightPhase, string> = {
  preflight: 'Preflight checks',
  takeoff_preparation: 'Take-off preparation',
  takeoff: 'Take-off',
  initial_climb: 'Initial climb',
  climb: 'Climb',
  cruise: 'En-route',
  approach: 'Approach',
  descent: 'Descent',
  flare: 'Flare',
  touchdown: 'Touchdown',
  post_landing: 'Post-landing',
  landed: 'Landed',
  go_around: 'Go-around',
  aborted: 'Aborted',
}

export const PHASE_ORDER: FlightPhase[] = [
  'preflight',
  'takeoff_preparation',
  'takeoff',
  'initial_climb',
  'climb',
  'cruise',
  'approach',
  'descent',
  'flare',
  'touchdown',
  'post_landing',
  'landed',
  'go_around',
  'aborted',
]

/** Categorical colour (CSS variable) per phase group. */
export function phaseTone(p: FlightPhase): 'ground' | 'climb' | 'cruise' | 'landing' | 'warning' | 'danger' | 'success' {
  switch (p) {
    case 'preflight':
    case 'takeoff_preparation':
    case 'touchdown':
    case 'post_landing':
      return 'ground'
    case 'takeoff':
    case 'initial_climb':
    case 'climb':
      return 'climb'
    case 'cruise':
      return 'cruise'
    case 'approach':
    case 'descent':
    case 'flare':
      return 'landing'
    case 'go_around':
      return 'warning'
    case 'aborted':
      return 'danger'
    case 'landed':
      return 'success'
  }
}

export const LANDING_LABEL: Record<LandingClass, string> = {
  successful: 'Successful landing',
  unsuccessful: 'Unsuccessful landing',
  not_attempted: 'No landing attempted',
}

export const SEVERITY_ORDER: Record<SafetySeverity, number> = { info: 0, warning: 1, critical: 2, fatal: 3 }

export function controllerLabel(k: string): string {
  return k === 'gain_scheduled' ? 'Gain-scheduled PID (B)' : 'Cascaded PID (A)'
}

/**
 * Break angle series at the 0/360 deg wrap so line charts do not draw a vertical
 * jump: the sample right after a wrap gets `null` (a gap) for that key.
 */
export function breakAngleWraps<T extends object>(samples: T[], keys: (keyof T)[]): T[] {
  const prev: Partial<Record<keyof T, number>> = {}
  return samples.map((s) => {
    const out = { ...s }
    for (const k of keys) {
      const v = s[k] as unknown as number | null
      const p = prev[k]
      if (typeof v === 'number') {
        if (p !== undefined && Math.abs(v - p) > 180) (out as Record<keyof T, unknown>)[k] = null
        prev[k] = v
      }
    }
    return out
  })
}
