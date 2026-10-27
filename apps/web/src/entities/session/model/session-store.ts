import { defineStore } from 'pinia'
import { computed, shallowRef } from 'vue'

import type { Schemas } from '@/shared/api'

import { requestLogout, requestRefresh } from '../api/requests'

export type SessionUser = Schemas['UserResponse']

interface Session {
  token: string
  expiresAt: number
  user: SessionUser
}

/** The access token is renewed after this share of its lifetime has passed. */
const RENEW_AT = 0.8
const MIN_RENEW_DELAY_MS = 5_000
const EXPIRING_SOON_MS = 30_000

/**
 * The access token lives in memory only (CLAUDE.md §6.3). A rotating refresh token in an HttpOnly
 * cookie restores the session after a reload and renews the access token before it expires.
 */
export const useSessionStore = defineStore('session', () => {
  const session = shallowRef<Session | null>(null)
  let timer: ReturnType<typeof setTimeout> | undefined
  let restoring: Promise<void> | undefined

  const user = computed(() => session.value?.user ?? null)
  const isAuthenticated = computed(() => session.value !== null)

  function token(): string | null {
    const current = session.value
    if (current === null) {
      return null
    }
    if (Date.now() >= current.expiresAt) {
      session.value = null
      return null
    }
    return current.token
  }

  function expiresSoon(): boolean {
    const current = session.value
    return current !== null && current.expiresAt - Date.now() < EXPIRING_SOON_MS
  }

  function signIn(login: Schemas['LoginResponse']): void {
    session.value = {
      token: login.access_token,
      expiresAt: Date.now() + login.expires_in * 1000,
      user: login.user,
    }
    clearTimeout(timer)
    timer = setTimeout(
      () => void refresh(),
      Math.max(login.expires_in * 1000 * RENEW_AT, MIN_RENEW_DELAY_MS),
    )
  }

  function signOut(): void {
    clearTimeout(timer)
    session.value = null
  }

  /** Renews the session from the refresh cookie; signs out when the cookie is gone or invalid. */
  async function refresh(): Promise<boolean> {
    const login = await requestRefresh()
    if (login === null) {
      signOut()
      return false
    }
    signIn(login)
    return true
  }

  /** Called once at startup; navigation waits for it via [`whenReady`]. */
  function restore(): Promise<void> {
    restoring ??= refresh().then(() => undefined)
    return restoring
  }

  function whenReady(): Promise<void> {
    return restoring ?? Promise.resolve()
  }

  /** Forgets the session locally and revokes it on the server. */
  async function logout(): Promise<void> {
    signOut()
    await requestLogout()
  }

  return {
    user,
    isAuthenticated,
    token,
    expiresSoon,
    signIn,
    signOut,
    refresh,
    restore,
    whenReady,
    logout,
  }
})
