import { BarChart3, Download, FileJson, RefreshCw } from 'lucide-react'

import { EmptyState, PageHeader } from '@/components/common/Display'
import { Button } from '@/components/ui/button'
import { api } from '@/services/api'
import { useSimulationStore } from '@/stores/simulationStore'
import { useUiStore } from '@/stores/uiStore'

import { ResultsView } from './ResultsView'

export function ResultsPage() {
  const session = useSimulationStore((s) => s.session)
  const results = useSimulationStore((s) => s.results)
  const telemetry = useSimulationStore((s) => s.telemetry)
  const exportCsv = useSimulationStore((s) => s.exportCsv)
  const exportJson = useSimulationStore((s) => s.exportJson)
  const setPage = useUiStore((s) => s.setPage)

  const loadPartial = async () => {
    if (!session) return
    try {
      useSimulationStore.setState({ results: await api.results(session.id, true) })
    } catch (e) {
      useSimulationStore.setState({ error: e instanceof Error ? e.message : String(e) })
    }
  }

  return (
    <div>
      <PageHeader
        title="Результати досліджень"
        description="Показники, класифікація посадки, часова діаграма фаз і часові ряди поточного сеансу моделювання. Усі значення є змодельованими даними."
        actions={
          session ? (
            <>
              {session.state !== 'finished' ? (
                <Button size="sm" variant="secondary" onClick={() => void loadPartial()}>
                  <RefreshCw /> Проміжні результати
                </Button>
              ) : null}
              <Button size="sm" variant="secondary" onClick={() => void exportCsv()}>
                <Download /> Телеметрія CSV
              </Button>
              <Button size="sm" variant="secondary" onClick={() => void exportJson()}>
                <FileJson /> Експеримент JSON
              </Button>
            </>
          ) : null
        }
      />
      {results ? (
        <ResultsView report={results} telemetry={telemetry} />
      ) : (
        <EmptyState title={session ? 'Моделювання ще триває' : 'Результатів ще немає'} icon={<BarChart3 />}>
          {session ? 'Рушій обчислює результати після завершення прогону; проміжні результати можна запросити будь-коли.' : 'Запустіть моделювання в робочій зоні або виконайте готовий експеримент.'}
          <div className="mt-3">
            <Button size="sm" onClick={() => setPage(session ? 'simulation' : 'compare')}>
              {session ? 'Відкрити робочу зону' : 'Відкрити експерименти'}
            </Button>
          </div>
        </EmptyState>
      )}
    </div>
  )
}
