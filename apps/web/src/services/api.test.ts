import { describe, expect, it, vi } from 'vitest'

import { ApiError, createApiClient } from './api'

function jsonResponse(status: number, body: unknown) {
  return new Response(body === undefined ? null : JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } })
}

describe('API client', () => {
  it('sends JSON and parses responses', async () => {
    const fetchImpl = vi.fn(async () => jsonResponse(200, { status: 'ok', version: '0.1.0', uptime_s: 1, sessions: 0 }))
    const api = createApiClient({ fetchImpl })
    await expect(api.health()).resolves.toMatchObject({ status: 'ok' })
    await api.createSimulation({ name: 'x' } as never, 5)
    const [url, init] = fetchImpl.mock.calls[1] as unknown as [string, RequestInit]
    expect(url).toBe('/api/simulations')
    expect(init.method).toBe('POST')
    expect(JSON.parse(init.body as string)).toEqual({ config: { name: 'x' }, speed: 5 })
  })

  it('maps structured backend errors to ApiError with validation details', async () => {
    const fetchImpl = vi.fn(async () =>
      jsonResponse(422, { error: { code: 'validation_failed', message: 'configuration is invalid', details: [{ field: 'airship.length_m', message: 'too small', severity: 'error' }] } }),
    )
    const api = createApiClient({ fetchImpl })
    const err = await api.validate({} as never).catch((e: unknown) => e)
    expect(err).toBeInstanceOf(ApiError)
    expect((err as ApiError).status).toBe(422)
    expect((err as ApiError).code).toBe('validation_failed')
    expect((err as ApiError).details[0]?.field).toBe('airship.length_m')
  })

  it('reports network failures distinctly', async () => {
    const api = createApiClient({ fetchImpl: vi.fn(async () => Promise.reject(new TypeError('Failed to fetch'))) })
    const err = (await api.health().catch((e: unknown) => e)) as ApiError
    expect(err.code).toBe('network')
    expect(err.message).toContain('Cannot reach the simulation backend')
  })

  it('handles 204 and non-JSON error bodies', async () => {
    const api = createApiClient({ fetchImpl: vi.fn(async () => new Response(null, { status: 204 })) })
    await expect(api.deleteSimulation('abc')).resolves.toBeUndefined()
    const api2 = createApiClient({ fetchImpl: vi.fn(async () => new Response('gateway down', { status: 502 })) })
    const err = (await api2.health().catch((e: unknown) => e)) as ApiError
    expect(err.status).toBe(502)
  })
})
