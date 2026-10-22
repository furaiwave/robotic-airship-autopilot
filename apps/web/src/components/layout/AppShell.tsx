import { Activity, BarChart3, Cloud, Cpu, GitCompare, Gauge, Moon, Plane, Route, Sun } from 'lucide-react'
import type * as React from 'react'
import { useEffect, useState } from 'react'

import { PhaseBadge } from '@/components/common/Display'
import { Button } from '@/components/ui/button'
import { Tooltip } from '@/components/ui/overlays'
import { cn } from '@/lib/utils'
import { api } from '@/services/api'
import { useConfigStore } from '@/stores/configStore'
import { useSimulationStore } from '@/stores/simulationStore'
import { type PageId, useUiStore } from '@/stores/uiStore'

const NAV: { id: PageId; label: string; icon: React.ReactNode; group: string }[] = [
  { id: 'dashboard', label: 'Flight operations', icon: <Gauge />, group: 'Operate' },
  { id: 'simulation', label: 'Simulation workspace', icon: <Activity />, group: 'Operate' },
  { id: 'airship', label: 'Airship', icon: <Plane />, group: 'Configure' },
  { id: 'environment', label: 'Environment', icon: <Cloud />, group: 'Configure' },
  { id: 'flight-path', label: 'Flight path', icon: <Route />, group: 'Configure' },
  { id: 'autopilot', label: 'Autopilot', icon: <Cpu />, group: 'Configure' },
  { id: 'results', label: 'Results', icon: <BarChart3 />, group: 'Research' },
  { id: 'compare', label: 'Experiments & comparison', icon: <GitCompare />, group: 'Research' },
]

function useBackendHealth() {
  const [ok, setOk] = useState<boolean | null>(null)
  useEffect(() => {
    let alive = true
    const check = () =>
      api
        .health()
        .then(() => alive && setOk(true))
        .catch(() => alive && setOk(false))
    void check()
    const t = setInterval(check, 5000)
    return () => {
      alive = false
      clearInterval(t)
    }
  }, [])
  return ok
}

export function AppShell({ children }: { children: React.ReactNode }) {
  const page = useUiStore((s) => s.page)
  const setPage = useUiStore((s) => s.setPage)
  const theme = useUiStore((s) => s.theme)
  const toggleTheme = useUiStore((s) => s.toggleTheme)
  const session = useSimulationStore((s) => s.session)
  const validation = useConfigStore((s) => s.validation)
  const backend = useBackendHealth()
  const groups = [...new Set(NAV.map((n) => n.group))]
  const errors = validation?.issues.filter((i) => i.severity === 'error').length ?? 0

  return (
    <div className="flex h-full min-h-0">
      <aside className="hidden w-52 shrink-0 flex-col border-r border-border bg-surface md:flex">
        <div className="flex h-11 items-center gap-2 border-b border-border px-3">
          <div className="grid size-6 place-items-center rounded bg-primary/15 text-primary">
            <Plane className="size-3.5" />
          </div>
          <div className="leading-tight">
            <div className="text-[13px] font-semibold">Airship Autopilot Lab</div>
            <div className="text-[10px] text-subtle-foreground">take-off & landing research</div>
          </div>
        </div>
        <nav className="flex-1 overflow-y-auto p-2" aria-label="Main">
          {groups.map((g) => (
            <div key={g} className="mb-3">
              <div className="px-2 pb-1 text-[10px] font-medium tracking-wider text-subtle-foreground uppercase">{g}</div>
              {NAV.filter((n) => n.group === g).map((n) => (
                <button
                  key={n.id}
                  onClick={() => setPage(n.id)}
                  aria-current={page === n.id ? 'page' : undefined}
                  className={cn(
                    'flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-[13px] text-muted-foreground transition-colors [&_svg]:size-3.5',
                    'hover:bg-accent hover:text-foreground',
                    page === n.id && 'bg-accent text-foreground',
                  )}
                >
                  {n.icon}
                  {n.label}
                </button>
              ))}
            </div>
          ))}
        </nav>
        <div className="border-t border-border p-3 text-[10px] leading-snug text-subtle-foreground">
          Simulation software for academic research. Not certified and not suitable for controlling a real aircraft.
        </div>
      </aside>

      <div className="flex min-w-0 flex-1 flex-col">
        <header className="flex h-11 shrink-0 items-center gap-3 border-b border-border bg-surface px-4">
          <select
            aria-label="Page"
            className="h-7 rounded-md border border-input bg-background px-2 text-xs md:hidden"
            value={page}
            onChange={(e) => setPage(e.target.value as PageId)}
          >
            {NAV.map((n) => (
              <option key={n.id} value={n.id}>
                {n.label}
              </option>
            ))}
          </select>
          <div className="flex items-center gap-2 text-xs">
            <span
              className={cn('inline-block size-2 rounded-full', backend === null ? 'bg-subtle-foreground' : backend ? 'bg-good' : 'bg-critical')}
              aria-hidden
            />
            <span className="text-muted-foreground">{backend === null ? 'Connecting…' : backend ? 'Rust engine online' : 'Engine offline'}</span>
          </div>
          {errors > 0 ? (
            <button onClick={() => setPage('airship')} className="text-xs text-critical hover:underline">
              {errors} configuration error{errors === 1 ? '' : 's'}
            </button>
          ) : null}
          <div className="ml-auto flex items-center gap-3">
            {session ? (
              <div className="hidden items-center gap-2 text-xs sm:flex">
                <span className="max-w-48 truncate text-muted-foreground">{session.name}</span>
                <PhaseBadge phase={session.phase} />
                <span className="tabular text-muted-foreground">{session.state}</span>
              </div>
            ) : null}
            <Tooltip content={theme === 'dark' ? 'Light theme' : 'Dark theme'}>
              <Button variant="ghost" size="icon-sm" onClick={toggleTheme} aria-label="Toggle theme">
                {theme === 'dark' ? <Sun /> : <Moon />}
              </Button>
            </Tooltip>
          </div>
        </header>
        <main className="min-h-0 flex-1 overflow-y-auto p-4 lg:p-5">{children}</main>
      </div>
    </div>
  )
}
