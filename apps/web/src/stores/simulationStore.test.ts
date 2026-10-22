import { beforeEach, describe, expect, it, vi } from 'vitest'

import { api, ApiError } from '@/services/api'
import { comparisonReport, defaultConfig, sessionStatus } from '@/test/fixtures'

import { useSimulationStore } from './simulationStore'

vi.mock('@/services/api', async (orig) => {
  const mod = await orig<typeof import('@/services/api')>()
  return {
    ...mod,
    api: {
      createSimulation: vi.fn(),
      start: vi.fn(),
      pause: vi.fn(),
      stop: vi.fn(),
      status: vi.fn(),
      telemetry: vi.fn(),
      results: vi.fn(),
      deleteSimulation: vi.fn(async () => undefined),
      setSpeed: vi.fn(),
    },
  }
})

const m = api as unknown as Record<string, ReturnType<typeof vi.fn>>

describe('simulation store', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    useSimulationStore.getState().stopPolling()
    useSimulationStore.setState({ session: null, telemetry: [], nextIndex: 0, results: null, error: null, issues: [] })
  })

  it('creates and starts a session, then accumulates polled telemetry', async () => {
    const tel = comparisonReport().runs[0]!.telemetry
    m.createSimulation!.mockResolvedValue(sessionStatus({ state: 'created' }))
    m.start!.mockResolvedValue(sessionStatus({ state: 'running' }))
    m.status!.mockResolvedValue(sessionStatus({ state: 'running' }))
    m.telemetry!.mockResolvedValueOnce({ total: 10, next_index: 10, samples: tel.slice(0, 10) }).mockResolvedValueOnce({ total: 15, next_index: 15, samples: tel.slice(10, 15) })

    const ok = await useSimulationStore.getState().create(defaultConfig())
    expect(ok).toBe(true)
    useSimulationStore.getState().stopPolling()
    await useSimulationStore.getState().poll()
    await useSimulationStore.getState().poll()
    const s = useSimulationStore.getState()
    expect(s.telemetry).toHaveLength(15)
    expect(s.nextIndex).toBe(15)
    expect(m.telemetry!.mock.calls[1]![1]).toBe(10)
  })

  it('fetches results once the run finishes and stops polling', async () => {
    const report = comparisonReport().runs[0]!.results
    useSimulationStore.setState({ session: sessionStatus() })
    m.status!.mockResolvedValue(sessionStatus({ state: 'finished', finished: 'landed' }))
    m.telemetry!.mockResolvedValue({ total: 0, next_index: 0, samples: [] })
    m.results!.mockResolvedValue(report)
    useSimulationStore.getState().startPolling()
    await vi.waitFor(() => expect(useSimulationStore.getState().results).not.toBeNull())
    expect(useSimulationStore.getState().polling).toBe(false)
  })

  it('surfaces backend validation errors', async () => {
    m.createSimulation!.mockRejectedValue(new ApiError(422, 'validation_failed', 'configuration is invalid', [{ field: 'airship.length_m', message: 'bad', severity: 'error' }]))
    const ok = await useSimulationStore.getState().create(defaultConfig())
    expect(ok).toBe(false)
    expect(useSimulationStore.getState().error).toBe('configuration is invalid')
    expect(useSimulationStore.getState().issues).toHaveLength(1)
  })
})
