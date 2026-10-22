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
        title="Research results"
        description="Metrics, landing classification, phase timeline and time series of the current simulation session. All values are simulated data."
        actions={
          session ? (
            <>
              {session.state !== 'finished' ? (
                <Button size="sm" variant="secondary" onClick={() => void loadPartial()}>
                  <RefreshCw /> Interim results
                </Button>
              ) : null}
              <Button size="sm" variant="secondary" onClick={() => void exportCsv()}>
                <Download /> Telemetry CSV
              </Button>
              <Button size="sm" variant="secondary" onClick={() => void exportJson()}>
                <FileJson /> Experiment JSON
              </Button>
            </>
          ) : null
        }
      />
      {results ? (
        <ResultsView report={results} telemetry={telemetry} />
      ) : (
        <EmptyState title={session ? 'Simulation still running' : 'No results yet'} icon={<BarChart3 />}>
          {session ? 'Results are computed by the engine when the run finishes; interim results can be requested at any time.' : 'Run a simulation from the workspace, or run a predefined experiment.'}
          <div className="mt-3">
            <Button size="sm" onClick={() => setPage(session ? 'simulation' : 'compare')}>
              {session ? 'Open workspace' : 'Open experiments'}
            </Button>
          </div>
        </EmptyState>
      )}
    </div>
  )
}
