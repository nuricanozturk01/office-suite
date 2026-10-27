import { afterEach, describe, expect, it, vi } from 'vitest'

import { apiOrigin, apiWebSocketOrigin } from './origin'

describe('apiOrigin', () => {
  afterEach(() => {
    vi.unstubAllEnvs()
  })

  it('falls back to the page origin when VITE_API_URL is unset', () => {
    vi.stubEnv('VITE_API_URL', '')
    expect(apiOrigin()).toBe(window.location.origin)
  })

  it('uses VITE_API_URL without a trailing slash', () => {
    vi.stubEnv('VITE_API_URL', 'https://myoffice-api.example.com/')
    expect(apiOrigin()).toBe('https://myoffice-api.example.com')
  })

  it('maps https to wss for WebSocket URLs', () => {
    vi.stubEnv('VITE_API_URL', 'https://myoffice-api.example.com')
    expect(apiWebSocketOrigin()).toBe('wss://myoffice-api.example.com')
  })
})
