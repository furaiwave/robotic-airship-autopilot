import { create } from 'zustand'

import { setIn } from '@/lib/utils'
import { api, ApiError } from '@/services/api'
import type { Experiment, MetricDefinition, Scenario, SimulationConfig, ValidationResponse } from '@/types/api'

const STORAGE_KEY = 'airship.config.v1'

function loadStored(): SimulationConfig | null {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    return raw ? (JSON.parse(raw) as SimulationConfig) : null
  } catch {
    return null
  }
}

function store(c: SimulationConfig) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(c))
  } catch {
    /* draft is kept in memory only */
  }
}

interface ConfigState {
  config: SimulationConfig | null
  defaults: SimulationConfig | null
  validation: ValidationResponse | null
  validating: boolean
  loading: boolean
  error: string | null
  scenarios: Scenario[]
  experiments: Experiment[]
  metricDefinitions: MetricDefinition[]
  load: () => Promise<void>
  setConfig: (c: SimulationConfig) => void
  update: (path: string, value: unknown) => void
  resetToDefault: () => void
  applyScenario: (id: string) => void
  validateNow: () => Promise<ValidationResponse | null>
}

let validateTimer: ReturnType<typeof setTimeout> | undefined
let validationSeq = 0

export const useConfigStore = create<ConfigState>((set, get) => {
  const scheduleValidation = () => {
    if (validateTimer) clearTimeout(validateTimer)
    validateTimer = setTimeout(() => void get().validateNow(), 300)
  }
  return {
    config: null,
    defaults: null,
    validation: null,
    validating: false,
    loading: false,
    error: null,
    scenarios: [],
    experiments: [],
    metricDefinitions: [],

    load: async () => {
      set({ loading: true, error: null })
      try {
        const [defaults, scenarios, experiments, metricDefinitions] = await Promise.all([
          api.defaultConfig(),
          api.scenarios(),
          api.experiments(),
          api.metricDefinitions(),
        ])
        const stored = loadStored()
        set({ defaults, config: stored ?? defaults, scenarios, experiments, metricDefinitions, loading: false })
        await get().validateNow()
      } catch (e) {
        set({ loading: false, error: e instanceof Error ? e.message : String(e) })
      }
    },

    setConfig: (config) => {
      store(config)
      set({ config })
      scheduleValidation()
    },

    update: (path, value) => {
      const c = get().config
      if (!c) return
      get().setConfig(setIn(c, path, value))
    },

    resetToDefault: () => {
      const d = get().defaults
      if (d) get().setConfig(structuredClone(d))
    },

    applyScenario: (id) => {
      const c = get().config
      const s = get().scenarios.find((x) => x.id === id)
      if (!c || !s) return
      // Keep the current mission geometry (sites, initial state); take the disturbances.
      const environment = {
        ...c.environment,
        atmosphere: s.environment.atmosphere,
        wind: s.environment.wind,
        sensor_noise: s.environment.sensor_noise,
        landing_area: s.environment.landing_area,
      }
      get().setConfig({ ...c, environment })
    },

    validateNow: async () => {
      const c = get().config
      if (!c) return null
      const seq = ++validationSeq
      set({ validating: true })
      try {
        const v = await api.validate(c)
        if (seq === validationSeq) set({ validation: v, validating: false, error: null })
        return v
      } catch (e) {
        if (seq === validationSeq) set({ validating: false, error: e instanceof ApiError ? e.message : String(e) })
        return null
      }
    },
  }
})

/** Validation issues whose field path starts with `prefix`. */
export function issuesFor(v: ValidationResponse | null, prefix: string) {
  return (v?.issues ?? []).filter((i) => i.field === prefix || i.field.startsWith(`${prefix}.`) || i.field.startsWith(`${prefix}[`))
}
