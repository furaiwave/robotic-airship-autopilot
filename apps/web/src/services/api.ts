import type {
  ApiErrorBody,
  ComparisonReport,
  Experiment,
  Health,
  MetricDefinition,
  ResultsReport,
  Scenario,
  SessionStatus,
  SimulationConfig,
  TelemetryResponse,
  TemplateResponse,
  ValidationIssue,
  ValidationResponse,
} from '@/types/api'

/** Error raised for non-2xx responses and network failures. */
export class ApiError extends Error {
  readonly status: number
  readonly code: string
  readonly details: ValidationIssue[]

  constructor(status: number, code: string, message: string, details: ValidationIssue[] = []) {
    super(message)
    this.name = 'ApiError'
    this.status = status
    this.code = code
    this.details = details
  }
}

export interface ApiClientOptions {
  baseUrl?: string
  fetchImpl?: typeof fetch
}

export function createApiClient({ baseUrl = '', fetchImpl }: ApiClientOptions = {}) {
  const doFetch = (input: string, init?: RequestInit) => (fetchImpl ?? globalThis.fetch)(input, init)

  async function request<T>(method: string, path: string, body?: unknown): Promise<T> {
    let response: Response
    try {
      response = await doFetch(`${baseUrl}/api${path}`, {
        method,
        headers: body === undefined ? undefined : { 'content-type': 'application/json' },
        body: body === undefined ? undefined : JSON.stringify(body),
      })
    } catch (e) {
      throw new ApiError(0, 'network', `Cannot reach the simulation backend (${e instanceof Error ? e.message : String(e)})`)
    }
    if (response.status === 204) return undefined as T
    const text = await response.text()
    let data: unknown = undefined
    if (text.length > 0) {
      try {
        data = JSON.parse(text)
      } catch {
        data = text
      }
    }
    if (!response.ok) {
      const err = (data as ApiErrorBody | undefined)?.error
      throw new ApiError(response.status, err?.code ?? 'http_error', err?.message ?? `HTTP ${response.status}`, err?.details ?? [])
    }
    return data as T
  }

  async function download(path: string): Promise<Blob> {
    let response: Response
    try {
      response = await doFetch(`${baseUrl}/api${path}`)
    } catch (e) {
      throw new ApiError(0, 'network', `Cannot reach the simulation backend (${e instanceof Error ? e.message : String(e)})`)
    }
    if (!response.ok) throw new ApiError(response.status, 'http_error', `Download failed: HTTP ${response.status}`)
    return response.blob()
  }

  return {
    health: () => request<Health>('GET', '/health'),
    defaultConfig: () => request<SimulationConfig>('GET', '/config/default'),
    validate: (config: SimulationConfig) => request<ValidationResponse>('POST', '/config/validate', config),
    metricDefinitions: () => request<MetricDefinition[]>('GET', '/metrics/definitions'),
    scenarios: () => request<Scenario[]>('GET', '/presets/scenarios'),
    experiments: () => request<Experiment[]>('GET', '/presets/experiments'),
    pathTemplates: (config: SimulationConfig) => request<TemplateResponse[]>('POST', '/flight-path/templates', config),
    listSimulations: () => request<SessionStatus[]>('GET', '/simulations'),
    createSimulation: (config: SimulationConfig, speed = 1) => request<SessionStatus>('POST', '/simulations', { config, speed }),
    status: (id: string) => request<SessionStatus>('GET', `/simulations/${id}/status`),
    start: (id: string, speed?: number) => request<SessionStatus>('POST', `/simulations/${id}/start`, speed === undefined ? {} : { speed }),
    pause: (id: string) => request<SessionStatus>('POST', `/simulations/${id}/pause`),
    resume: (id: string) => request<SessionStatus>('POST', `/simulations/${id}/resume`),
    stop: (id: string) => request<SessionStatus>('POST', `/simulations/${id}/stop`),
    reset: (id: string) => request<SessionStatus>('POST', `/simulations/${id}/reset`),
    setSpeed: (id: string, speed: number) => request<SessionStatus>('POST', `/simulations/${id}/speed`, { speed }),
    deleteSimulation: (id: string) => request<void>('DELETE', `/simulations/${id}`),
    telemetry: (id: string, since = 0, maxPoints?: number) =>
      request<TelemetryResponse>('GET', `/simulations/${id}/telemetry?since=${since}${maxPoints ? `&max_points=${maxPoints}` : ''}`),
    results: (id: string, partial = false) => request<ResultsReport>('GET', `/simulations/${id}/results${partial ? '?partial=true' : ''}`),
    exportCsv: (id: string) => download(`/simulations/${id}/export.csv`),
    exportJson: (id: string) => download(`/simulations/${id}/export.json`),
    runBatch: (config: SimulationConfig, maxPoints = 1500) =>
      request<ComparisonReport>('POST', '/experiments/run', { config, max_points: maxPoints }),
    compare: (runs: { label: string; config: SimulationConfig }[], maxPoints = 1500) =>
      request<ComparisonReport>('POST', '/experiments/compare', { runs, max_points: maxPoints }),
    runPreset: (id: string) => request<ComparisonReport>('POST', `/experiments/presets/${id}/run`),
  }
}

export type ApiClient = ReturnType<typeof createApiClient>

export const api: ApiClient = createApiClient()
