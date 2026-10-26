import { Loader2 } from 'lucide-react'
import { useEffect } from 'react'

import { ErrorBanner } from '@/components/common/Display'
import { AppShell } from '@/components/layout/AppShell'
import { Button } from '@/components/ui/button'
import { TooltipProvider } from '@/components/ui/overlays'
import { AirshipConfigPage } from '@/features/airship-config/AirshipConfigPage'
import { AutopilotPage } from '@/features/autopilot/AutopilotPage'
import { DashboardPage } from '@/features/dashboard/DashboardPage'
import { EnvironmentPage } from '@/features/environment/EnvironmentPage'
import { FlightPathPage } from '@/features/flight-path/FlightPathPage'
import { ComparePage } from '@/features/research/ComparePage'
import { ResultsPage } from '@/features/research/ResultsPage'
import { SimulationPage } from '@/features/simulation/SimulationPage'
import { useConfigStore } from '@/stores/configStore'
import { useUiStore } from '@/stores/uiStore'

function Page() {
  const page = useUiStore((s) => s.page)
  switch (page) {
    case 'dashboard':
      return <DashboardPage />
    case 'airship':
      return <AirshipConfigPage />
    case 'environment':
      return <EnvironmentPage />
    case 'flight-path':
      return <FlightPathPage />
    case 'autopilot':
      return <AutopilotPage />
    case 'simulation':
      return <SimulationPage />
    case 'results':
      return <ResultsPage />
    case 'compare':
      return <ComparePage />
  }
}

export default function App() {
  const config = useConfigStore((s) => s.config)
  const loading = useConfigStore((s) => s.loading)
  const error = useConfigStore((s) => s.error)
  const load = useConfigStore((s) => s.load)

  useEffect(() => {
    void load()
  }, [load])

  return (
    <TooltipProvider>
      <AppShell>
        {config ? (
          <Page />
        ) : (
          <div className="mx-auto mt-16 max-w-lg">
            {error ? (
              <>
                <ErrorBanner message={`${error}. Запустіть Rust API командою «cargo run -p api-server --release» і повторіть спробу.`} />
                <Button onClick={() => void load()}>Повторити</Button>
              </>
            ) : (
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <Loader2 className="size-4 animate-spin" /> {loading ? 'Завантаження конфігурації з рушія моделювання…' : 'Запуск…'}
              </div>
            )}
          </div>
        )}
      </AppShell>
    </TooltipProvider>
  )
}
