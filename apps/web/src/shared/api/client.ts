import { createApiClient } from '@office/api-client'

import { ApiError, networkError, toApiError } from './errors'
import { apiOrigin } from './origin'

interface AuthHooks {
  /** Current access token, if signed in. */
  getToken: () => string | null
  /** Called when an authenticated request is rejected with 401. */
  onUnauthorized: () => void
}

let auth: AuthHooks = { getToken: () => null, onUnauthorized: () => undefined }

/** Wires the session into the HTTP client; called once by the app layer. */
export function configureAuth(hooks: AuthHooks): void {
  auth = hooks
}

export const api = createApiClient({
  baseUrl: apiOrigin(),
  // Sends the refresh cookie when the API is on another origin of the same site.
  credentials: 'include',
  // Resolved per call so tests (and devtools) can replace the global.
  fetch: (request) => globalThis.fetch(request),
})

api.use({
  onRequest({ request }) {
    const token = auth.getToken()
    if (token !== null) {
      request.headers.set('Authorization', `Bearer ${token}`)
    }
    return request
  },
  onResponse({ request, response }) {
    if (response.status === 401 && request.headers.has('Authorization')) {
      auth.onUnauthorized()
    }
    return response
  },
})

interface FetchResult<T> {
  data?: T
  error?: unknown
  response: Response
}

async function settle<T>(request: Promise<FetchResult<T>>): Promise<FetchResult<T>> {
  try {
    return await request
  } catch (error) {
    if (error instanceof ApiError) {
      throw error
    }
    throw networkError()
  }
}

/** Resolves to the response body or throws an {@link ApiError}. */
export async function unwrap<T>(request: Promise<FetchResult<T>>): Promise<T> {
  const { data, error, response } = await settle(request)
  if (error !== undefined || !response.ok || data === undefined) {
    throw toApiError(error, response)
  }
  return data
}

/** For endpoints without a response body (204). */
export async function expectOk(request: Promise<FetchResult<unknown>>): Promise<void> {
  const { error, response } = await settle(request)
  if (error !== undefined || !response.ok) {
    throw toApiError(error, response)
  }
}

/** Like {@link unwrap}, but also returns the response so callers can read its headers. */
export async function unwrapWithResponse<T>(
  request: Promise<FetchResult<T>>,
): Promise<{ data: T; response: Response }> {
  const { data, error, response } = await settle(request)
  if (error !== undefined || !response.ok || data === undefined) {
    throw toApiError(error, response)
  }
  return { data, response }
}
