import createClient, { type ClientOptions, type Middleware } from 'openapi-fetch'

import type { components, paths } from './schema'

export type { components, paths }
export type Schemas = components['schemas']
export type ApiClient = ReturnType<typeof createApiClient>
export type { Middleware }

export function createApiClient(options: ClientOptions = {}) {
  return createClient<paths>(options)
}
