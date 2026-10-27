import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { useSessionStore } from './session-store'

const user = { id: '0192f2a0-0000-7000-8000-000000000001', email: 'a@b.co', display_name: 'Ada' }

function login(token: string, expiresIn = 100) {
  return { access_token: token, token_type: 'Bearer', expires_in: expiresIn, user }
}

function json(status: number, body: unknown): Response {
  const type = status >= 400 ? 'application/problem+json' : 'application/json'
  return new Response(JSON.stringify(body), { status, headers: { 'Content-Type': type } })
}

describe('session store', () => {
  const fetchMock = vi.fn<(request: Request) => Promise<Response>>()

  beforeEach(() => {
    vi.useFakeTimers()
    vi.stubGlobal('fetch', fetchMock)
    setActivePinia(createPinia())
  })

  afterEach(() => {
    vi.useRealTimers()
    vi.unstubAllGlobals()
    fetchMock.mockReset()
  })

  function paths(): string[] {
    return fetchMock.mock.calls.map(([request]) => new URL(request.url).pathname)
  }

  it('restores the session from the refresh cookie after a reload', async () => {
    fetchMock.mockResolvedValue(json(200, login('restored')))
    const session = useSessionStore()
    await session.restore()

    expect(session.token()).toBe('restored')
    expect(session.user?.display_name).toBe('Ada')
    const request = fetchMock.mock.calls[0]?.[0]
    expect(request?.headers.get('x-requested-with')).toBe('office')
  })

  it('stays signed out when there is no valid refresh cookie', async () => {
    fetchMock.mockResolvedValue(
      json(401, { status: 401, code: 'invalid_refresh_token', title: 'Invalid refresh token' }),
    )
    const session = useSessionStore()
    await session.restore()
    await session.whenReady()

    expect(session.isAuthenticated).toBe(false)
    expect(session.token()).toBeNull()
  })

  it('renews the access token before it expires', async () => {
    fetchMock.mockResolvedValue(json(200, login('second')))
    const session = useSessionStore()
    session.signIn(login('first', 100))
    expect(session.token()).toBe('first')

    await vi.advanceTimersByTimeAsync(79_000)
    expect(fetchMock).not.toHaveBeenCalled()
    await vi.advanceTimersByTimeAsync(2_000)
    expect(paths()).toEqual(['/api/v1/auth/refresh'])
    expect(session.token()).toBe('second')
  })

  it('signs out when renewing fails', async () => {
    fetchMock.mockResolvedValue(
      json(401, { status: 401, code: 'invalid_refresh_token', title: 'Invalid refresh token' }),
    )
    const session = useSessionStore()
    session.signIn(login('first', 100))
    await vi.advanceTimersByTimeAsync(81_000)

    expect(session.isAuthenticated).toBe(false)
  })

  it('logs out locally and on the server', async () => {
    fetchMock.mockResolvedValue(new Response(null, { status: 204 }))
    const session = useSessionStore()
    session.signIn(login('first'))
    await session.logout()

    expect(session.isAuthenticated).toBe(false)
    expect(paths()).toEqual(['/api/v1/auth/logout'])
    // The renewal timer was cancelled with the session.
    await vi.advanceTimersByTimeAsync(200_000)
    expect(paths()).toEqual(['/api/v1/auth/logout'])
  })
})
