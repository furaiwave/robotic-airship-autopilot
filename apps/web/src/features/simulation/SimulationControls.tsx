import { Download, FileJson, Pause, Play, RotateCcw, Square, Upload } from 'lucide-react'
import { useEffect } from 'react'

import { ErrorBanner } from '@/components/common/Display'
import { Button } from '@/components/ui/button'
import { Select } from '@/components/ui/overlays'
import { fmtTime } from '@/lib/format'
import { useConfigStore } from '@/stores/configStore'
import { useSimulationStore } from '@/stores/simulationStore'

const SPEEDS = [
  { value: '1', label: '1× real time' },
  { value: '2', label: '2×' },
  { value: '5', label: '5×' },
  { value: '10', label: '10×' },
  { value: '25', label: '25×' },
  { value: '100', label: '100×' },
  { value: '0', label: 'As fast as possible' },
]

/** Lifecycle controls. Speed changes how many fixed steps run per wall-clock tick, never dt. */
export function SimulationControls() {
  const config = useConfigStore((s) => s.config)
  const validation = useConfigStore((s) => s.validation)
  const sim = useSimulationStore()
  const s = sim.session

  useEffect(() => {
    if (s?.state === 'running') sim.startPolling()
  }, [s?.id])

  const configInvalid = validation ? !validation.valid : false
  const loadAndStart = async () => {
    if (!config) return
    if (await sim.create(config)) await sim.start()
  }
  const running = s?.state === 'running'
  const paused = s?.state === 'paused'
  const finished = s?.state === 'finished'

  return (
    <div className="space-y-2">
      {sim.error ? <ErrorBanner message={sim.error} details={sim.issues} onDismiss={sim.clearError} /> : null}
      <div className="flex flex-wrap items-center gap-2 rounded-lg border border-border bg-surface px-3 py-2">
        <Button size="sm" onClick={() => void loadAndStart()} disabled={!config || configInvalid || sim.busy} title={configInvalid ? 'Fix configuration errors first' : undefined}>
          <Upload /> Load config & start
        </Button>
        <div className="mx-1 h-5 w-px bg-border" />
        <Button size="sm" variant="secondary" onClick={() => void sim.start()} disabled={!s || s.state !== 'created' || sim.busy}>
          <Play /> Start
        </Button>
        {paused ? (
          <Button size="sm" variant="secondary" onClick={() => void sim.resume()} disabled={sim.busy}>
            <Play /> Resume
          </Button>
        ) : (
          <Button size="sm" variant="secondary" onClick={() => void sim.pause()} disabled={!running || sim.busy}>
            <Pause /> Pause
          </Button>
        )}
        <Button size="sm" variant="secondary" onClick={() => void sim.stop()} disabled={!s || finished || sim.busy}>
          <Square /> Stop
        </Button>
        <Button size="sm" variant="secondary" onClick={() => void sim.reset()} disabled={!s || sim.busy}>
          <RotateCcw /> Reset
        </Button>
        <div className="w-44">
          <Select ariaLabel="Simulation speed" value={String(sim.speed)} onValueChange={(v) => void sim.setSpeed(Number(v))} options={SPEEDS} />
        </div>
        <div className="ml-auto flex items-center gap-3 text-xs">
          {s ? (
            <>
              <span className="tabular text-muted-foreground">
                t = <span className="text-foreground">{fmtTime(s.time_s)}</span> / {fmtTime(s.duration_s)}
              </span>
              <div className="h-1.5 w-28 overflow-hidden rounded-full bg-border-strong" role="progressbar" aria-valuenow={Math.round(s.progress * 100)} aria-valuemin={0} aria-valuemax={100}>
                <div className="h-full bg-primary" style={{ width: `${s.progress * 100}%` }} />
              </div>
            </>
          ) : (
            <span className="text-muted-foreground">No simulation loaded</span>
          )}
          <Button size="sm" variant="ghost" onClick={() => void sim.exportCsv()} disabled={!s || s.telemetry_count === 0}>
            <Download /> CSV
          </Button>
          <Button size="sm" variant="ghost" onClick={() => void sim.exportJson()} disabled={!s || s.telemetry_count === 0}>
            <FileJson /> JSON
          </Button>
        </div>
      </div>
      {s?.finish_message ? <p className="text-xs text-muted-foreground">Run ended: {s.finish_message}</p> : null}
    </div>
  )
}
