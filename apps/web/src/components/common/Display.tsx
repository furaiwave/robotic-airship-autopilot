// Compact display components shared by the pages.
import { AlertOctagon, AlertTriangle, CheckCircle2, Info, XCircle } from 'lucide-react'
import type * as React from 'react'

import { Badge } from '@/components/ui/primitives'
import { PHASE_LABEL, phaseTone } from '@/lib/format'
import { cn } from '@/lib/utils'
import type { FlightPhase, LandingClass, SafetySeverity } from '@/types/api'

/** Telemetry value card. `role` distinguishes measured/simulated state, targets and commands. */
export function StatCard({
  label,
  value,
  unit,
  sub,
  role = 'state',
  className,
}: {
  label: string
  value: string
  unit?: string
  sub?: React.ReactNode
  role?: 'state' | 'target' | 'command'
  className?: string
}) {
  const roleMark = { state: 'bg-series-1', target: 'bg-subtle-foreground', command: 'bg-series-2' }[role]
  return (
    <div className={cn('rounded-md border border-border bg-surface px-3 py-2', className)}>
      <div className="flex items-center gap-1.5 text-[11px] text-muted-foreground">
        <span className={cn('inline-block h-2 w-0.5 rounded-full', roleMark)} aria-hidden />
        <span className="truncate">{label}</span>
      </div>
      <div className="mt-0.5 flex items-baseline gap-1">
        <span className="text-lg font-semibold tabular leading-tight">{value}</span>
        {unit ? <span className="text-[11px] text-subtle-foreground">{unit}</span> : null}
      </div>
      {sub ? <div className="mt-0.5 truncate text-[11px] text-muted-foreground tabular">{sub}</div> : null}
    </div>
  )
}

export function RoleLegend() {
  return (
    <div className="flex flex-wrap items-center gap-3 text-[11px] text-muted-foreground">
      <span className="flex items-center gap-1">
        <span className="inline-block h-2 w-0.5 rounded-full bg-series-1" /> модельований стан
      </span>
      <span className="flex items-center gap-1">
        <span className="inline-block h-2 w-0.5 rounded-full bg-subtle-foreground" /> завдання / еталон
      </span>
      <span className="flex items-center gap-1">
        <span className="inline-block h-2 w-0.5 rounded-full bg-series-2" /> керувальна команда
      </span>
    </div>
  )
}

const TONE_CLASS: Record<ReturnType<typeof phaseTone>, string> = {
  ground: 'border-border-strong text-muted-foreground',
  climb: 'border-[var(--phase-climb)] text-foreground',
  cruise: 'border-[var(--phase-cruise)] text-foreground',
  landing: 'border-[var(--phase-landing)] text-foreground',
  warning: 'border-warning text-foreground',
  danger: 'border-critical text-foreground',
  success: 'border-good text-foreground',
}

export function PhaseBadge({ phase, className }: { phase: FlightPhase; className?: string }) {
  const tone = phaseTone(phase)
  return (
    <span className={cn('inline-flex items-center gap-1.5 rounded border px-1.5 py-0.5 text-[11px] font-medium', TONE_CLASS[tone], className)}>
      <span className="inline-block size-1.5 rounded-full" style={{ background: `var(--phase-${tone === 'warning' ? 'climb' : tone})` }} aria-hidden />
      {PHASE_LABEL[phase]}
    </span>
  )
}

export function SeverityBadge({ severity }: { severity: SafetySeverity }) {
  const map = {
    info: { v: 'info' as const, icon: <Info />, label: 'Інфо' },
    warning: { v: 'warning' as const, icon: <AlertTriangle />, label: 'Попередження' },
    critical: { v: 'serious' as const, icon: <AlertOctagon />, label: 'Критична' },
    fatal: { v: 'critical' as const, icon: <XCircle />, label: 'Фатальна' },
  }[severity]
  return (
    <Badge variant={map.v}>
      {map.icon}
      {map.label}
    </Badge>
  )
}

export function LandingBadge({ c }: { c: LandingClass }) {
  if (c === 'successful')
    return (
      <Badge variant="good">
        <CheckCircle2 /> Успішна посадка
      </Badge>
    )
  if (c === 'unsuccessful')
    return (
      <Badge variant="critical">
        <XCircle /> Неуспішна посадка
      </Badge>
    )
  return (
    <Badge variant="outline">
      <Info /> Посадку не виконували
    </Badge>
  )
}

export function EmptyState({ title, children, icon }: { title: string; children?: React.ReactNode; icon?: React.ReactNode }) {
  return (
    <div className="flex flex-col items-center justify-center gap-2 rounded-lg border border-dashed border-border-strong px-6 py-10 text-center">
      {icon ? <div className="text-subtle-foreground [&_svg]:size-6">{icon}</div> : null}
      <p className="text-sm font-medium">{title}</p>
      {children ? <div className="max-w-md text-xs text-muted-foreground">{children}</div> : null}
    </div>
  )
}

export function PageHeader({ title, description, actions }: { title: string; description?: string; actions?: React.ReactNode }) {
  return (
    <div className="mb-4 flex flex-wrap items-end justify-between gap-3">
      <div>
        <h1 className="text-base font-semibold tracking-tight">{title}</h1>
        {description ? <p className="mt-0.5 max-w-3xl text-xs text-muted-foreground">{description}</p> : null}
      </div>
      {actions ? <div className="flex flex-wrap items-center gap-2">{actions}</div> : null}
    </div>
  )
}

export function ErrorBanner({ message, onDismiss, details }: { message: string; onDismiss?: () => void; details?: { field: string; message: string }[] }) {
  return (
    <div role="alert" className="mb-3 rounded-md border border-critical/50 bg-critical/10 px-3 py-2 text-xs">
      <div className="flex items-start justify-between gap-3">
        <div className="flex items-start gap-2">
          <AlertOctagon className="mt-0.5 size-3.5 shrink-0 text-critical" />
          <span>{message}</span>
        </div>
        {onDismiss ? (
          <button className="text-muted-foreground hover:text-foreground" onClick={onDismiss}>
            Закрити
          </button>
        ) : null}
      </div>
      {details && details.length > 0 ? (
        <ul className="mt-1.5 ml-5 list-disc space-y-0.5 text-muted-foreground">
          {details.slice(0, 8).map((d) => (
            <li key={d.field + d.message}>
              <span className="font-mono">{d.field}</span>: {d.message}
            </li>
          ))}
        </ul>
      ) : null}
    </div>
  )
}

export function KeyValue({ items }: { items: [string, React.ReactNode][] }) {
  return (
    <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-xs">
      {items.map(([k, v]) => (
        <div key={k} className="contents">
          <dt className="text-muted-foreground">{k}</dt>
          <dd className="text-right tabular">{v}</dd>
        </div>
      ))}
    </dl>
  )
}
