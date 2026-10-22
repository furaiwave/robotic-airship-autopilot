// SVG flight visualisations: top-down trajectory view (also used as the path editor
// canvas) and side view. All geometry comes from the configuration, the backend's
// approach geometry and recorded telemetry; nothing is simulated here.
import { useMemo, useRef, useState } from 'react'

import { fmt } from '@/lib/format'
import type { ApproachGeometry, SimulationConfig, TelemetrySample, Vec2, Waypoint } from '@/types/api'

const W = 800
const H = 500
const PAD = 36

interface Bounds {
  minX: number
  maxX: number
  minY: number
  maxY: number
}

function boundsOf(points: Vec2[]): Bounds {
  const xs = points.map((p) => p.x)
  const ys = points.map((p) => p.y)
  return { minX: Math.min(...xs), maxX: Math.max(...xs), minY: Math.min(...ys), maxY: Math.max(...ys) }
}

/** World (East, North) to SVG mapping with equal scale on both axes. */
function makeMap(b: Bounds) {
  const spanX = Math.max(b.maxX - b.minX, 40)
  const spanY = Math.max(b.maxY - b.minY, 40)
  const scale = Math.min((W - 2 * PAD) / spanX, (H - 2 * PAD) / spanY)
  const cx = (b.minX + b.maxX) / 2
  const cy = (b.minY + b.maxY) / 2
  return {
    scale,
    toSvg: (p: Vec2) => ({ x: W / 2 + (p.x - cx) * scale, y: H / 2 - (p.y - cy) * scale }),
    toWorld: (sx: number, sy: number): Vec2 => ({ x: cx + (sx - W / 2) / scale, y: cy - (sy - H / 2) / scale }),
  }
}

function niceStep(span: number): number {
  const raw = span / 6
  const p = 10 ** Math.floor(Math.log10(raw))
  const m = raw / p
  return (m < 1.5 ? 1 : m < 3.5 ? 2 : m < 7.5 ? 5 : 10) * p
}

function headingVec(deg: number): Vec2 {
  const r = (deg * Math.PI) / 180
  return { x: Math.sin(r), y: Math.cos(r) }
}

export function approachPoints(a: ApproachGeometry | null | undefined) {
  if (!a) return null
  const u = a.course_dir
  const at = (d: number): Vec2 => ({ x: a.landing.x - u.x * d, y: a.landing.y - u.y * d })
  return { gate: at(a.gate_distance), pregate: at(a.gate_distance + a.pregate_extension), join: at(a.gate_distance + a.pregate_extension + 40) }
}

function AirshipGlyph({ x, y, rotationDeg, size = 26, color = 'var(--series-1)' }: { x: number; y: number; rotationDeg: number; size?: number; color?: string }) {
  const l = size
  const d = size * 0.32
  return (
    <g transform={`translate(${x} ${y}) rotate(${rotationDeg})`} aria-label="airship">
      <path d={`M ${-l * 0.45} 0 L ${-l * 0.62} ${-d * 0.9} L ${-l * 0.5} 0 L ${-l * 0.62} ${d * 0.9} Z`} fill={color} opacity={0.85} />
      <ellipse cx={0} cy={0} rx={l / 2} ry={d / 2} fill={color} stroke="var(--surface)" strokeWidth={1.5} />
      <circle cx={l * 0.32} cy={0} r={1.6} fill="var(--surface)" />
    </g>
  )
}

export interface TopViewProps {
  config: SimulationConfig
  approach?: ApproachGeometry | null
  trajectory?: TelemetrySample[]
  current?: TelemetrySample | null
  /** Optional comparison trajectories (e.g. second controller). */
  overlays?: { label: string; color: string; samples: TelemetrySample[] }[]
  editable?: boolean
  selected?: number | null
  onSelect?: (i: number | null) => void
  onAdd?: (p: Vec2) => void
  onMove?: (i: number, p: Vec2) => void
  height?: number
}

export function TopView({ config, approach, trajectory = [], current, overlays = [], editable, selected, onSelect, onAdd, onMove, height = 420 }: TopViewProps) {
  const env = config.environment
  const wps = config.flight_path.waypoints
  const ap = approachPoints(approach)
  const svgRef = useRef<SVGSVGElement>(null)
  const [drag, setDrag] = useState<number | null>(null)

  // Bounds are fixed by the mission geometry so the view does not jump while flying;
  // the trajectory extends them only when it leaves that box.
  const geometry = useMemo(() => {
    const pts: Vec2[] = [env.takeoff_site, env.landing_site, env.initial_state.position, ...wps.map((w) => ({ x: w.x_m, y: w.y_m }))]
    if (ap) pts.push(ap.join, ap.pregate)
    return pts
  }, [env.takeoff_site, env.landing_site, env.initial_state.position, wps, ap])
  const trajBox = useMemo(() => (trajectory.length > 0 ? boundsOf(trajectory.map((s) => ({ x: s.x_m, y: s.y_m }))) : null), [trajectory])
  const b = useMemo(() => {
    const g = boundsOf(geometry)
    const m = trajBox ? { minX: Math.min(g.minX, trajBox.minX), maxX: Math.max(g.maxX, trajBox.maxX), minY: Math.min(g.minY, trajBox.minY), maxY: Math.max(g.maxY, trajBox.maxY) } : g
    const pad = Math.max(m.maxX - m.minX, m.maxY - m.minY) * 0.06 + 20
    return { minX: m.minX - pad, maxX: m.maxX + pad, minY: m.minY - pad, maxY: m.maxY + pad }
  }, [geometry, trajBox])
  const map = makeMap(b)
  const step = niceStep(Math.max(b.maxX - b.minX, b.maxY - b.minY))

  const toWorldEvent = (e: React.MouseEvent<SVGSVGElement>) => {
    const svg = svgRef.current
    if (!svg) return null
    const pt = svg.createSVGPoint()
    pt.x = e.clientX
    pt.y = e.clientY
    const ctm = svg.getScreenCTM()
    if (!ctm) return null
    const p = pt.matrixTransform(ctm.inverse())
    return map.toWorld(p.x, p.y)
  }

  const gridLines: React.ReactNode[] = []
  for (let x = Math.ceil(b.minX / step) * step; x <= b.maxX; x += step) {
    const s = map.toSvg({ x, y: 0 })
    gridLines.push(<line key={`gx${x}`} x1={s.x} x2={s.x} y1={0} y2={H} stroke="var(--grid)" strokeWidth={1} />)
  }
  for (let y = Math.ceil(b.minY / step) * step; y <= b.maxY; y += step) {
    const s = map.toSvg({ x: 0, y })
    gridLines.push(<line key={`gy${y}`} x1={0} x2={W} y1={s.y} y2={s.y} stroke="var(--grid)" strokeWidth={1} />)
  }

  // Landing area rectangle aligned with the landing-axis heading.
  const la = env.landing_area
  const u = headingVec(la.heading_deg)
  const left = { x: -u.y, y: u.x }
  const corner = (along: number, cross: number) => map.toSvg({ x: env.landing_site.x + u.x * along + left.x * cross, y: env.landing_site.y + u.y * along + left.y * cross })
  const area = [corner(-la.undershoot_m, la.half_width_m), corner(la.overrun_m, la.half_width_m), corner(la.overrun_m, -la.half_width_m), corner(-la.undershoot_m, -la.half_width_m)]

  const planned: Vec2[] = [env.initial_state.position, ...wps.map((w) => ({ x: w.x_m, y: w.y_m }))]
  if (ap) planned.push(ap.join)
  planned.push(env.landing_site)
  const line = (pts: Vec2[]) => pts.map((p) => map.toSvg(p)).map((p) => `${p.x.toFixed(1)},${p.y.toFixed(1)}`).join(' ')

  const land = map.toSvg(env.landing_site)
  const tol = config.autopilot.landing.landing_point_tolerance_m * map.scale
  const takeoff = map.toSvg(env.takeoff_site)

  const windSpeed = current?.wind_speed_m_s ?? env.wind.speed_m_s
  const windFrom = current ? current.wind_from_deg : env.wind.from_direction_deg
  const windTo = headingVec(windFrom + 180)

  return (
    <svg
      ref={svgRef}
      viewBox={`0 0 ${W} ${H}`}
      className="w-full rounded-md border border-border bg-surface select-none"
      style={{ height }}
      role="img"
      aria-label="Top-down view of the mission and trajectory"
      onClick={(e) => {
        if (!editable || drag !== null) return
        if ((e.target as Element).getAttribute('data-wp') !== null) return
        const p = toWorldEvent(e)
        if (p && onAdd) onAdd({ x: Math.round(p.x), y: Math.round(p.y) })
      }}
      onMouseMove={(e) => {
        if (drag === null || !onMove) return
        const p = toWorldEvent(e)
        if (p) onMove(drag, { x: Math.round(p.x), y: Math.round(p.y) })
      }}
      onMouseUp={() => setDrag(null)}
      onMouseLeave={() => setDrag(null)}
    >
      {gridLines}
      <polygon points={area.map((p) => `${p.x},${p.y}`).join(' ')} fill="var(--status-good)" fillOpacity={0.08} stroke="var(--status-good)" strokeOpacity={0.5} strokeDasharray="4 3" />
      <circle cx={land.x} cy={land.y} r={Math.max(tol, 3)} fill="none" stroke="var(--status-good)" strokeWidth={1.5} />
      <line x1={land.x - 6} x2={land.x + 6} y1={land.y} y2={land.y} stroke="var(--foreground)" strokeWidth={1.5} />
      <line x1={land.x} x2={land.x} y1={land.y - 6} y2={land.y + 6} stroke="var(--foreground)" strokeWidth={1.5} />

      {ap ? (
        <g>
          <polyline points={line([ap.join, env.landing_site])} fill="none" stroke="var(--phase-landing)" strokeWidth={1.5} strokeDasharray="6 4" />
          {(['gate', 'pregate'] as const).map((k) => {
            const p = map.toSvg(ap[k])
            return (
              <g key={k}>
                <line x1={p.x - 5} x2={p.x + 5} y1={p.y - 5} y2={p.y + 5} stroke="var(--phase-landing)" strokeWidth={1.5} />
                <line x1={p.x - 5} x2={p.x + 5} y1={p.y + 5} y2={p.y - 5} stroke="var(--phase-landing)" strokeWidth={1.5} />
                <text x={p.x + 7} y={p.y - 6} fontSize={13} fill="var(--ink-muted)">
                  {k === 'gate' ? 'gate' : 'pre-gate'}
                </text>
              </g>
            )
          })}
        </g>
      ) : null}

      <polyline points={line(planned)} fill="none" stroke="var(--ink-muted)" strokeWidth={1.5} strokeDasharray="5 4" />

      {overlays.map((o) => (
        <polyline key={o.label} points={line(o.samples.map((s) => ({ x: s.x_m, y: s.y_m })))} fill="none" stroke={o.color} strokeWidth={2} strokeLinejoin="round" />
      ))}
      {trajectory.length > 1 ? (
        <polyline points={line(trajectory.map((s) => ({ x: s.x_m, y: s.y_m })))} fill="none" stroke="var(--series-1)" strokeWidth={2} strokeLinejoin="round" />
      ) : null}

      <g>
        <path d={`M ${takeoff.x} ${takeoff.y - 9} L ${takeoff.x + 7} ${takeoff.y + 5} L ${takeoff.x - 7} ${takeoff.y + 5} Z`} fill="none" stroke="var(--foreground)" strokeWidth={1.5} />
        <text x={takeoff.x + 9} y={takeoff.y + 14} fontSize={13} fill="var(--ink-muted)">
          mast
        </text>
        <text x={land.x + 9} y={land.y + 14} fontSize={13} fill="var(--ink-muted)">
          landing point
        </text>
      </g>

      {wps.map((w: Waypoint, i: number) => {
        const p = map.toSvg({ x: w.x_m, y: w.y_m })
        const isSel = selected === i
        return (
          <g key={i}>
            <circle cx={p.x} cy={p.y} r={Math.max(w.tolerance_m * map.scale, 4)} fill="var(--series-3)" fillOpacity={0.08} stroke="var(--series-3)" strokeOpacity={0.5} />
            <circle
              data-wp={i}
              cx={p.x}
              cy={p.y}
              r={isSel ? 8 : 6.5}
              fill={isSel ? 'var(--series-3)' : 'var(--surface)'}
              stroke="var(--series-3)"
              strokeWidth={2}
              style={{ cursor: editable ? 'grab' : 'default' }}
              onMouseDown={(e) => {
                if (!editable) return
                e.stopPropagation()
                setDrag(i)
                onSelect?.(i)
              }}
              onClick={(e) => {
                e.stopPropagation()
                onSelect?.(i)
              }}
            />
            <text x={p.x + 9} y={p.y - 8} fontSize={14} fontWeight={600} fill="var(--foreground)">
              {`WP${i + 1}`}
            </text>
            <text x={p.x + 9} y={p.y + 4} fontSize={12.5} fill="var(--ink-muted)">
              {`${fmt(w.altitude_m, 0)} m${w.speed_m_s < 0.3 ? ' · hover' : ''}`}
            </text>
          </g>
        )
      })}

      {current ? (() => {
        const p = map.toSvg({ x: current.x_m, y: current.y_m })
        return <AirshipGlyph x={p.x} y={p.y} rotationDeg={current.heading_deg - 90} size={34} />
      })() : null}

      {/* Wind indicator */}
      <g transform={`translate(${W - 58} 48)`}>
        <circle r={26} fill="var(--surface-2)" stroke="var(--border)" />
        {windSpeed > 0.05 ? (
          <g>
            <line x1={-windTo.x * 16} y1={windTo.y * 16} x2={windTo.x * 16} y2={-windTo.y * 16} stroke="var(--foreground)" strokeWidth={2} />
            <circle cx={windTo.x * 16} cy={-windTo.y * 16} r={3} fill="var(--foreground)" />
          </g>
        ) : (
          <text textAnchor="middle" y={4} fontSize={13} fill="var(--ink-muted)">
            calm
          </text>
        )}
        <text textAnchor="middle" y={42} fontSize={13} fill="var(--muted-foreground)">
          {`wind ${fmt(windSpeed, 1)} m/s`}
        </text>
      </g>
      {/* North arrow and scale */}
      <g transform="translate(26 30)">
        <path d="M 0 -14 L 5 2 L 0 -2 L -5 2 Z" fill="var(--foreground)" />
        <text textAnchor="middle" y={14} fontSize={13} fill="var(--muted-foreground)">
          N
        </text>
      </g>
      <g transform={`translate(20 ${H - 16})`}>
        <line x1={0} x2={step * map.scale} y1={0} y2={0} stroke="var(--foreground)" strokeWidth={1.5} />
        <line x1={0} x2={0} y1={-4} y2={4} stroke="var(--foreground)" strokeWidth={1.5} />
        <line x1={step * map.scale} x2={step * map.scale} y1={-4} y2={4} stroke="var(--foreground)" strokeWidth={1.5} />
        <text x={(step * map.scale) / 2} y={-6} textAnchor="middle" fontSize={13} fill="var(--muted-foreground)">
          {`${step} m`}
        </text>
      </g>
    </svg>
  )
}

export interface SideViewProps {
  config: SimulationConfig
  approach?: ApproachGeometry | null
  trajectory?: TelemetrySample[]
  current?: TelemetrySample | null
  mode: 'final' | 'east' | 'north'
  height?: number
}

/** Side view: gear height above ground versus a horizontal coordinate. */
export function SideView({ config, approach, trajectory = [], current, mode, height = 240 }: SideViewProps) {
  const hx = (s: TelemetrySample) => (mode === 'final' ? -s.distance_to_go_m : mode === 'east' ? s.x_m : s.y_m)
  const land = config.environment.landing_site
  const takeoff = config.environment.takeoff_site
  const landX = mode === 'final' ? 0 : mode === 'east' ? land.x : land.y
  const takeoffX = mode === 'east' ? takeoff.x : mode === 'north' ? takeoff.y : null
  const xs = trajectory.map(hx)
  const alts = trajectory.map((s) => s.altitude_agl_m)
  const maxAlt = Math.max(config.autopilot.target_altitude_m, config.autopilot.approach.altitude_m, ...config.flight_path.waypoints.map((w) => w.altitude_m), ...alts, 10) * 1.15
  const gateD = approach ? approach.gate_distance + approach.pregate_extension : 250
  const minX = Math.min(...xs, landX - (mode === 'final' ? gateD + 40 : 50), takeoffX ?? Infinity)
  const maxX = Math.max(...xs, landX + 40, takeoffX ?? -Infinity)
  const pl = 56
  const pr = 14
  const pt = 14
  const pb = 34
  const sx = (x: number) => pl + ((x - minX) / Math.max(maxX - minX, 1)) * (W - pl - pr)
  const sy = (h: number) => H * 0.5 - pb - (h / maxAlt) * (H * 0.5 - pt - pb) + 0
  const vbH = H * 0.5
  const pts = trajectory.map((s) => `${sx(hx(s)).toFixed(1)},${sy(s.altitude_agl_m).toFixed(1)}`).join(' ')
  const refPts = trajectory
    .filter((s) => s.ref_altitude_agl_m !== null)
    .map((s) => `${sx(hx(s)).toFixed(1)},${sy(s.ref_altitude_agl_m as number).toFixed(1)}`)
    .join(' ')
  const altStep = niceStep(maxAlt)
  const ticks: number[] = []
  for (let a = 0; a <= maxAlt; a += altStep) ticks.push(a)

  let glide: string | null = null
  if (mode === 'final' && approach) {
    const flare = approach.flare_height
    glide = `${sx(-approach.gate_distance)},${sy(approach.approach_altitude)} ${sx(0)},${sy(flare)}`
  }
  const la = config.environment.landing_area
  return (
    <svg viewBox={`0 0 ${W} ${vbH}`} className="w-full rounded-md border border-border bg-surface" style={{ height }} role="img" aria-label="Side view: height above ground">
      {ticks.map((a) => (
        <g key={a}>
          <line x1={pl} x2={W - pr} y1={sy(a)} y2={sy(a)} stroke="var(--grid)" />
          <text x={pl - 6} y={sy(a) + 3} textAnchor="end" fontSize={15} fill="var(--ink-muted)" className="tabular">
            {a}
          </text>
        </g>
      ))}
      <text x={10} y={pt + 2} fontSize={15} fill="var(--ink-muted)">
        m AGL
      </text>
      <rect x={pl} y={sy(0)} width={W - pl - pr} height={6} fill="var(--axis)" />
      {mode === 'final' ? (
        <rect x={sx(-la.undershoot_m)} y={sy(0) - 1} width={Math.max(sx(la.overrun_m) - sx(-la.undershoot_m), 2)} height={4} fill="var(--status-good)" opacity={0.7} />
      ) : null}
      {glide ? <polyline points={glide} fill="none" stroke="var(--phase-landing)" strokeWidth={1.5} strokeDasharray="6 4" /> : null}
      {refPts ? <polyline points={refPts} fill="none" stroke="var(--ink-muted)" strokeWidth={1.5} strokeDasharray="5 4" /> : null}
      {pts ? <polyline points={pts} fill="none" stroke="var(--series-1)" strokeWidth={2} strokeLinejoin="round" /> : null}
      <line x1={sx(landX)} x2={sx(landX)} y1={sy(0)} y2={sy(0) - 10} stroke="var(--foreground)" strokeWidth={1.5} />
      <text x={sx(landX)} y={sy(0) + 18} textAnchor="middle" fontSize={15} fill="var(--ink-muted)">
        landing point
      </text>
      {takeoffX !== null ? (
        <text x={sx(takeoffX)} y={sy(0) + 18} textAnchor="middle" fontSize={15} fill="var(--ink-muted)">
          mast
        </text>
      ) : null}
      {current ? (
        <g transform={`translate(${sx(hx(current))} ${sy(current.altitude_agl_m) - 6})`}>
          <g transform={`rotate(${-current.pitch_deg})`}>
            <ellipse rx={14} ry={4.5} fill="var(--series-1)" stroke="var(--surface)" strokeWidth={1.2} />
            <rect x={-3} y={4} width={6} height={3} fill="var(--series-1)" />
          </g>
        </g>
      ) : null}
      <text x={W - pr} y={vbH - 6} textAnchor="end" fontSize={15} fill="var(--ink-muted)">
        {mode === 'final' ? 'along final course (m, landing point = 0)' : mode === 'east' ? 'East (m)' : 'North (m)'}
      </text>
    </svg>
  )
}
