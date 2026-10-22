import { create } from 'zustand'

import { api, ApiError } from '@/services/api'
import type { ComparisonReport, ControllerKind, SimulationConfig } from '@/types/api'

interface ResearchState {
  report: ComparisonReport | null
  title: string | null
  running: boolean
  error: string | null
  /** Run the current configuration with both controllers under identical conditions. */
  compareControllers: (config: SimulationConfig) => Promise<void>
  runExperiment: (id: string, title: string) => Promise<void>
  compareCustom: (runs: { label: string; config: SimulationConfig }[], title: string) => Promise<void>
  clear: () => void
}

const withController = (c: SimulationConfig, kind: ControllerKind): SimulationConfig => ({ ...c, controller: { ...c.controller, kind } })

export const useResearchStore = create<ResearchState>((set) => {
  const run = async (title: string, fn: () => Promise<ComparisonReport>) => {
    set({ running: true, error: null, title })
    try {
      set({ report: await fn(), running: false })
    } catch (e) {
      set({ running: false, error: e instanceof ApiError ? e.message : String(e) })
    }
  }
  return {
    report: null,
    title: null,
    running: false,
    error: null,
    compareControllers: (config) =>
      run(`${config.name}: controller comparison`, () =>
        api.compare([
          { label: 'Cascaded PID (A)', config: withController(config, 'cascaded_pid') },
          { label: 'Gain-scheduled PID (B)', config: withController(config, 'gain_scheduled') },
        ]),
      ),
    runExperiment: (id, title) => run(title, () => api.runPreset(id)),
    compareCustom: (runs, title) => run(title, () => api.compare(runs)),
    clear: () => set({ report: null, title: null, error: null }),
  }
})
