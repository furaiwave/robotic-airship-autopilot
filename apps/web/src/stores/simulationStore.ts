import { create } from 'zustand'

import { downloadBlob, slug } from '@/lib/utils'
import { api, ApiError } from '@/services/api'
import type { ResultsReport, SessionStatus, SimulationConfig, TelemetrySample, ValidationIssue } from '@/types/api'

export const POLL_INTERVAL_MS = 250
/** Client-side display buffer limit; older samples are thinned, never invented. */
export const MAX_BUFFER = 12000

interface SimulationState {
  session: SessionStatus | null
  config: SimulationConfig | null
  telemetry: TelemetrySample[]
  nextIndex: number
  results: ResultsReport | null
  error: string | null
  issues: ValidationIssue[]
  busy: boolean
  speed: number
  polling: boolean
  create: (config: SimulationConfig) => Promise<boolean>
  start: () => Promise<void>
  pause: () => Promise<void>
  resume: () => Promise<void>
  stop: () => Promise<void>
  reset: () => Promise<void>
  setSpeed: (speed: number) => Promise<void>
  poll: () => Promise<void>
  startPolling: () => void
  stopPolling: () => void
  exportCsv: () => Promise<void>
  exportJson: () => Promise<void>
  clearError: () => void
}

let timer: ReturnType<typeof setInterval> | undefined
let inFlight = false

function thin(samples: TelemetrySample[]): TelemetrySample[] {
  if (samples.length <= MAX_BUFFER) return samples
  // Keep every second sample of the older half.
  const half = Math.floor(samples.length / 2)
  return [...samples.slice(0, half).filter((_, i) => i % 2 === 0), ...samples.slice(half)]
}

export const useSimulationStore = create<SimulationState>((set, get) => {
  const fail = (e: unknown) => {
    if (e instanceof ApiError) set({ error: e.message, issues: e.details, busy: false })
    else set({ error: e instanceof Error ? e.message : String(e), issues: [], busy: false })
  }
  const control = async (fn: (id: string) => Promise<SessionStatus>) => {
    const s = get().session
    if (!s) return
    set({ busy: true, error: null })
    try {
      const session = await fn(s.id)
      set({ session, busy: false })
      get().startPolling()
    } catch (e) {
      fail(e)
    }
  }
  return {
    session: null,
    config: null,
    telemetry: [],
    nextIndex: 0,
    results: null,
    error: null,
    issues: [],
    busy: false,
    speed: 5,
    polling: false,

    create: async (config) => {
      set({ busy: true, error: null, issues: [] })
      try {
        const previous = get().session
        if (previous) void api.deleteSimulation(previous.id).catch(() => undefined)
        const session = await api.createSimulation(config, get().speed)
        set({ session, config, telemetry: [], nextIndex: 0, results: null, busy: false })
        return true
      } catch (e) {
        fail(e)
        return false
      }
    },
    start: () => control((id) => api.start(id, get().speed)),
    pause: () => control(api.pause),
    resume: () => control(api.resume),
    stop: async () => {
      await control(api.stop)
      await get().poll()
    },
    reset: async () => {
      await control(api.reset)
      set({ telemetry: [], nextIndex: 0, results: null })
    },
    setSpeed: async (speed) => {
      set({ speed })
      const s = get().session
      if (!s) return
      try {
        set({ session: await api.setSpeed(s.id, speed) })
      } catch (e) {
        fail(e)
      }
    },

    poll: async () => {
      const s = get().session
      if (!s || inFlight) return
      inFlight = true
      try {
        const [status, tel] = await Promise.all([api.status(s.id), api.telemetry(s.id, get().nextIndex)])
        if (get().session?.id !== s.id) return
        const telemetry = tel.samples.length > 0 ? thin([...get().telemetry, ...tel.samples]) : get().telemetry
        set({ session: status, telemetry, nextIndex: tel.next_index })
        if (status.state === 'finished' && !get().results) {
          set({ results: await api.results(s.id) })
        }
        if (status.state !== 'running') get().stopPolling()
      } catch (e) {
        fail(e)
        get().stopPolling()
      } finally {
        inFlight = false
      }
    },
    startPolling: () => {
      if (timer) return
      set({ polling: true })
      timer = setInterval(() => void get().poll(), POLL_INTERVAL_MS)
      void get().poll()
    },
    stopPolling: () => {
      if (timer) clearInterval(timer)
      timer = undefined
      set({ polling: false })
    },
    exportCsv: async () => {
      const s = get().session
      if (!s) return
      try {
        downloadBlob(await api.exportCsv(s.id), `${slug(s.name)}-telemetry.csv`)
      } catch (e) {
        fail(e)
      }
    },
    exportJson: async () => {
      const s = get().session
      if (!s) return
      try {
        downloadBlob(await api.exportJson(s.id), `${slug(s.name)}-experiment.json`)
      } catch (e) {
        fail(e)
      }
    },
    clearError: () => set({ error: null, issues: [] }),
  }
})
