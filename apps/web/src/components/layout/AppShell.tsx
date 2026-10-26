import { Activity, BarChart3, Cloud, Cpu, GitCompare, Gauge, Moon, Plane, Route, Sun } from 'lucide-react'
import type * as React from 'react'
import { useEffect, useState } from 'react'

import { PhaseBadge } from '@/components/common/Display'
import { Button } from '@/components/ui/button'
import { Tooltip } from '@/components/ui/overlays'
import { runStateUk } from '@/lib/i18n'
import { cn } from '@/lib/utils'
import { api } from '@/services/api'
import { useConfigStore } from '@/stores/configStore'
import { useSimulationStore } from '@/stores/simulationStore'
import { type PageId, useUiStore } from '@/stores/uiStore'

const NAV: { id: PageId; label: string; icon: React.ReactNode; group: string }[] = [
  { id: 'dashboard', label: 'Керування польотом', icon: <Gauge />, group: 'Робота' },
  { id: 'simulation', label: 'Робоча зона моделювання', icon: <Activity />, group: 'Робота' },
  { id: 'airship', label: 'Дирижабль', icon: <Plane />, group: 'Налаштування' },
  { id: 'environment', label: 'Середовище', icon: <Cloud />, group: 'Налаштування' },
  { id: 'flight-path', label: 'Маршрут польоту', icon: <Route />, group: 'Налаштування' },
  { id: 'autopilot', label: 'Автопілот', icon: <Cpu />, group: 'Налаштування' },
  { id: 'results', label: 'Результати', icon: <BarChart3 />, group: 'Дослідження' },
  { id: 'compare', label: 'Експерименти й порівняння', icon: <GitCompare />, group: 'Дослідження' },
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
            <div className="text-[13px] font-semibold">Автопілот дирижабля</div>
            <div className="text-[10px] text-subtle-foreground">лабораторія зльоту й посадки</div>
          </div>
        </div>
        <nav className="flex-1 overflow-y-auto p-2" aria-label="Головне меню">
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
          Програмне забезпечення для академічного моделювання. Не сертифіковане й не придатне для керування реальним літальним апаратом.
        </div>
      </aside>

      <div className="flex min-w-0 flex-1 flex-col">
        <header className="flex h-11 shrink-0 items-center gap-3 border-b border-border bg-surface px-4">
          <select
            aria-label="Сторінка"
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
            <span className="text-muted-foreground">{backend === null ? 'Підключення…' : backend ? 'Rust-рушій на зв’язку' : 'Рушій недоступний'}</span>
          </div>
          {errors > 0 ? (
            <button onClick={() => setPage('airship')} className="text-xs text-critical hover:underline">
              помилок конфігурації: {errors}
            </button>
          ) : null}
          <div className="ml-auto flex items-center gap-3">
            {session ? (
              <div className="hidden items-center gap-2 text-xs sm:flex">
                <span className="max-w-48 truncate text-muted-foreground">{session.name}</span>
                <PhaseBadge phase={session.phase} />
                <span className="tabular text-muted-foreground">{runStateUk(session.state)}</span>
              </div>
            ) : null}
            <Tooltip content={theme === 'dark' ? 'Світла тема' : 'Темна тема'}>
              <Button variant="ghost" size="icon-sm" onClick={toggleTheme} aria-label="Перемкнути тему">
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
