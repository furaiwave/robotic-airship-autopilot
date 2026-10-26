import { describe, expect, it } from 'vitest'

import { comparisonReport, defaultConfig } from '@/test/fixtures'

import { comparisonMetricsToCsv, metricValue, telemetryToCsv } from './export'
import { breakAngleWraps, fmt, fmtSigned, fmtTime } from './format'
import { airshipSchema, autopilotSchema, buildSchema, environmentSchema, flightPathSchema, simulationSchema } from './schemas'
import { getIn, setIn } from './utils'

describe('configuration validation (Zod)', () => {
  it('accepts the engine default configuration', () => {
    const c = defaultConfig()
    expect(airshipSchema.safeParse(c.airship).success).toBe(true)
    expect(environmentSchema.safeParse(c.environment).success).toBe(true)
    expect(autopilotSchema.safeParse(c.autopilot).success).toBe(true)
    expect(simulationSchema.safeParse(c.simulation).success).toBe(true)
    expect(flightPathSchema.safeParse(c.flight_path).success).toBe(true)
  })

  it('rejects out-of-range and physically inconsistent airship parameters', () => {
    const a = defaultConfig().airship
    expect(airshipSchema.safeParse({ ...a, length_m: -1 }).success).toBe(false)
    const r = airshipSchema.safeParse({ ...a, length_m: 2, max_diameter_m: 3 })
    expect(r.success).toBe(false)
    expect(r.error?.issues.some((i) => i.message.includes('меншою за діаметр'))).toBe(true)
    expect(airshipSchema.safeParse({ ...a, aero: { ...a.aero, cd_axial: Number.NaN } }).success).toBe(false)
  })

  it('allows the derived hull volume (null) but validates an override', () => {
    const a = defaultConfig().airship
    expect(airshipSchema.safeParse({ ...a, hull_volume_m3: null }).success).toBe(true)
    expect(airshipSchema.safeParse({ ...a, hull_volume_m3: -5 }).success).toBe(false)
  })

  it('enforces autopilot threshold ordering', () => {
    const ap = defaultConfig().autopilot
    const bad = { ...ap, landing: { ...ap.landing, decision_height_m: 5, flare_height_m: 3 } }
    const r = autopilotSchema.safeParse(bad)
    expect(r.success).toBe(false)
    expect(r.error?.issues[0]?.path).toEqual(['landing', 'decision_height_m'])
  })

  it('requires control and telemetry periods to be multiples of dt', () => {
    const s = defaultConfig().simulation
    expect(simulationSchema.safeParse({ ...s, dt_s: 0.03 }).success).toBe(false)
    expect(simulationSchema.safeParse({ ...s, dt_s: 0.005 }).success).toBe(true)
  })

  it('detects waypoints inside the previous acceptance radius', () => {
    const p = defaultConfig().flight_path
    const w = p.waypoints[0]!
    const r = flightPathSchema.safeParse({ ...p, waypoints: [w, { ...w, x_m: w.x_m + 1 }] })
    expect(r.success).toBe(false)
  })

  it('builds nested schemas from dotted paths and keeps unknown keys', () => {
    const s = buildSchema([{ name: 'a.b', label: 'B', min: 0, max: 1 }])
    const r = s.safeParse({ a: { b: 0.5, keep: 'x' }, other: true })
    expect(r.success).toBe(true)
    expect(r.data).toEqual({ a: { b: 0.5, keep: 'x' }, other: true })
    expect(s.safeParse({ a: { b: 2 } }).success).toBe(false)
  })
})

describe('utilities', () => {
  it('setIn updates immutably, getIn reads dotted paths', () => {
    const o = { a: { b: [1, 2], c: 3 } }
    const n = setIn(o, 'a.b.1', 9)
    expect(n.a.b).toEqual([1, 9])
    expect(o.a.b).toEqual([1, 2])
    expect(n.a).not.toBe(o.a)
    expect(getIn(n, 'a.c')).toBe(3)
    expect(getIn(n, 'x.y')).toBeUndefined()
  })

  it('formats numbers, signs and times', () => {
    expect(fmt(1.234, 2)).toBe('1.23')
    expect(fmt(null)).toBe('—')
    expect(fmt(Number.NaN)).toBe('—')
    expect(fmtSigned(2)).toBe('+2.0')
    expect(fmtTime(75.5)).toBe('01:15.5')
  })

  it('breaks heading series at the 0/360 wrap', () => {
    const out = breakAngleWraps([{ h: 350 }, { h: 355 }, { h: 2 }, { h: 5 }], ['h'])
    expect(out.map((x) => x.h)).toEqual([350, 355, null, 5])
  })
})

describe('exports', () => {
  it('writes telemetry CSV with a header and one row per sample', () => {
    const t = comparisonReport().runs[0]!.telemetry
    const csv = telemetryToCsv(t)
    const lines = csv.trim().split('\n')
    expect(lines).toHaveLength(t.length + 1)
    expect(lines[0]!.startsWith('t_s,phase,')).toBe(true)
    const n = lines[0]!.split(',').length
    expect(lines.every((l) => l.split(',').length === n)).toBe(true)
    expect(telemetryToCsv([])).toBe('')
  })

  it('writes a metric-by-run comparison CSV', () => {
    const report = comparisonReport()
    const csv = comparisonMetricsToCsv(report)
    expect(csv).toContain('Каскадний ПІД (A)')
    expect(csv).toContain('ПІД з плануванням коефіцієнтів (B)')
    expect(csv).toContain('Похибка точки посадки')
    expect(csv.startsWith('# Змодельовані дані')).toBe(true)
    expect(metricValue(report.runs[0]!.results.metrics, 'landing_attempts')).toBe(1)
  })
})
