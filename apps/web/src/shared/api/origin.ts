/**
 * Origin of the backend API, from `VITE_API_URL` (build time, e.g. `https://myoffice-api.example.com`).
 * Unset means same origin, which is the Vite dev proxy and a reverse-proxied build.
 */
export function apiOrigin(): string {
  const configured = (import.meta.env.VITE_API_URL ?? '').trim().replace(/\/+$/, '')
  return configured === '' ? window.location.origin : configured
}

/** {@link apiOrigin} with a WebSocket scheme (`ws:` or `wss:`). */
export function apiWebSocketOrigin(): string {
  return apiOrigin().replace(/^http/, 'ws')
}
