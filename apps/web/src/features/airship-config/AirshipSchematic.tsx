import { fmt } from '@/lib/format'
import type { AirshipParams } from '@/types/api'

/** Side-view schematic drawn to scale from the configured dimensions. */
export function AirshipSchematic({ p }: { p: AirshipParams }) {
  const W = 520
  const H = 260
  const L = p.length_m
  const D = p.max_diameter_m
  const scale = Math.min((W - 120) / L, (H - 90) / (D + p.gear_height_m - D / 2 + 0.5))
  const cx = W / 2
  const cy = 40 + (D / 2) * scale
  const rx = (L / 2) * scale
  const ry = (D / 2) * scale
  const cgY = cy + p.cg_below_cv_m * scale
  const gearY = cy + p.gear_height_m * scale
  const finX = cx - p.aero.fin_arm_m * scale
  const thrustY = cgY + p.propulsion.main_vertical_offset_m * scale
  const dim = 'var(--ink-muted)'
  return (
    <svg viewBox={`0 0 ${W} ${H}`} className="w-full" role="img" aria-label="Airship schematic with dimensions">
      <defs>
        <marker id="arr" viewBox="0 0 6 6" refX="3" refY="3" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
          <path d="M0,0 L6,3 L0,6 z" fill={dim} />
        </marker>
      </defs>
      {/* fins */}
      <path d={`M ${finX + 6} ${cy - ry * 0.35} L ${finX - 16} ${cy - ry * 1.15} L ${finX - 22} ${cy - ry * 0.2} Z`} fill="var(--series-1)" opacity={0.55} />
      <path d={`M ${finX + 6} ${cy + ry * 0.35} L ${finX - 16} ${cy + ry * 1.15} L ${finX - 22} ${cy + ry * 0.2} Z`} fill="var(--series-1)" opacity={0.55} />
      <ellipse cx={cx} cy={cy} rx={rx} ry={ry} fill="var(--series-1)" fillOpacity={0.18} stroke="var(--series-1)" strokeWidth={2} />
      {/* gondola and gear */}
      <rect x={cx - 26} y={cy + ry - 2} width={52} height={Math.max(gearY - cy - ry - 4, 6)} rx={4} fill="var(--surface-2)" stroke="var(--border-strong)" />
      <line x1={cx - 20} x2={cx + 20} y1={gearY} y2={gearY} stroke="var(--foreground)" strokeWidth={2} />
      <line x1={20} x2={W - 20} y1={gearY + 1} y2={gearY + 1} stroke="var(--axis)" strokeDasharray="3 3" />
      {/* thrusters */}
      <circle cx={cx + 30} cy={thrustY} r={5} fill="var(--series-2)" />
      <path d={`M ${cx + 36} ${thrustY} l 14 0`} stroke="var(--series-2)" strokeWidth={2} markerEnd="url(#arr)" />
      <path d={`M ${cx - 34} ${cgY + 2} l 0 -16`} stroke="var(--series-2)" strokeWidth={2} markerEnd="url(#arr)" />
      {/* CV and CG */}
      <circle cx={cx} cy={cy} r={3.5} fill="var(--foreground)" />
      <text x={cx + 6} y={cy - 4} fontSize={12} fill="var(--foreground)">
        CV
      </text>
      <circle cx={cx} cy={cgY} r={3.5} fill="none" stroke="var(--foreground)" strokeWidth={1.5} />
      <text x={cx + 6} y={cgY + 4} fontSize={12} fill="var(--foreground)">
        CG
      </text>
      {/* dimensions */}
      <line x1={cx - rx} x2={cx + rx} y1={22} y2={22} stroke={dim} markerStart="url(#arr)" markerEnd="url(#arr)" />
      <text x={cx} y={15} textAnchor="middle" fontSize={13} fill="var(--muted-foreground)">{`L = ${fmt(L, 1)} m`}</text>
      <line x1={cx + rx + 14} x2={cx + rx + 14} y1={cy - ry} y2={cy + ry} stroke={dim} markerStart="url(#arr)" markerEnd="url(#arr)" />
      <text x={cx + rx + 18} y={cy + 3} fontSize={13} fill="var(--muted-foreground)">{`D = ${fmt(D, 1)} m`}</text>
      <line x1={cx - rx - 12} x2={cx - rx - 12} y1={cy} y2={gearY} stroke={dim} markerStart="url(#arr)" markerEnd="url(#arr)" />
      <text x={cx - rx - 16} y={(cy + gearY) / 2 + 3} textAnchor="end" fontSize={13} fill="var(--muted-foreground)">{`gear ${fmt(p.gear_height_m, 1)} m`}</text>
      <text x={finX - 20} y={cy + ry * 1.15 + 14} textAnchor="middle" fontSize={12} fill="var(--muted-foreground)">{`fin arm ${fmt(p.aero.fin_arm_m, 1)} m`}</text>
      <text x={cx + 56} y={thrustY + 4} fontSize={12} fill="var(--muted-foreground)">main thrusters ×2</text>
      <text x={cx - 40} y={cgY - 16} textAnchor="end" fontSize={12} fill="var(--muted-foreground)">heave</text>
      <text x={cx - 6} y={cgY + 16} textAnchor="end" fontSize={12} fill="var(--muted-foreground)">{`BG ${fmt(p.cg_below_cv_m, 2)} m`}</text>
    </svg>
  )
}
