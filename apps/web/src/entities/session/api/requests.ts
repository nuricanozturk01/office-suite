import { api, ApiError, expectOk, type Schemas, unwrap } from '@/shared/api'

/** Cookie-authenticated endpoints require this header (CSRF guard, see the identity module). */
const csrfHeader = { 'x-requested-with': 'office' } as const

/**
 * Serializes refreshes across tabs: a rotated refresh cookie is shared by all tabs, so a second
 * tab that waits for the lock presents the already-rotated cookie instead of replaying the old one.
 */
async function exclusive<T>(task: () => Promise<T>): Promise<T> {
  return 'locks' in navigator ? navigator.locks.request('office-refresh', task) : task()
}

/** Exchanges the refresh cookie for a new session; `null` when there is none or it is invalid. */
export async function requestRefresh(): Promise<Schemas['LoginResponse'] | null> {
  return exclusive(async () => {
    try {
      return await unwrap(api.POST('/api/v1/auth/refresh', { params: { header: csrfHeader } }))
    } catch (error) {
      if (error instanceof ApiError) {
        return null
      }
      throw error
    }
  })
}

/** Revokes the server-side session; failures are irrelevant because the client forgets it anyway. */
export async function requestLogout(): Promise<void> {
  try {
    await expectOk(api.POST('/api/v1/auth/logout', { params: { header: csrfHeader } }))
  } catch (error) {
    if (!(error instanceof ApiError)) {
      throw error
    }
  }
}
