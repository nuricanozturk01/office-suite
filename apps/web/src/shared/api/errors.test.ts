import { describe, expect, it } from 'vitest'

import { ApiError, errorMessageKey, networkError, toApiError } from './errors'

describe('toApiError', () => {
  it('reads RFC 9457 problem bodies', () => {
    const error = toApiError(
      { type: 'about:blank', title: 'Conflict', status: 409, code: 'email_taken' },
      new Response(null, { status: 409 }),
    )
    expect(error).toMatchObject({ status: 409, code: 'email_taken' })
  })

  it('falls back when the body is not a problem', () => {
    const error = toApiError('<html>', new Response(null, { status: 502 }))
    expect(error).toMatchObject({ status: 502, code: 'unexpected_response' })
  })
})

describe('errorMessageKey', () => {
  it('maps known codes and hides unknown ones', () => {
    expect(errorMessageKey(new ApiError(401, 'invalid_credentials'))).toBe(
      'errors.invalid_credentials',
    )
    expect(errorMessageKey(new ApiError(409, 'version_conflict'))).toBe('errors.version_conflict')
    expect(errorMessageKey(new ApiError(418, 'teapot'))).toBe('errors.unknown')
    expect(errorMessageKey(networkError())).toBe('errors.network')
    expect(errorMessageKey(new Error('x'))).toBe('errors.unknown')
  })
})
