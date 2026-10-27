import { describe, expect, it } from 'vitest'

import { ApiError } from '@/shared/api'

import { isGrantable, shouldRetryShare } from './use-share-file'

describe('shouldRetryShare', () => {
  it('retries a 404 a few times because a fresh file may not be known yet', () => {
    const notFound = new ApiError(404, 'not_found')
    expect(shouldRetryShare(0, notFound)).toBe(true)
    expect(shouldRetryShare(2, notFound)).toBe(true)
    expect(shouldRetryShare(3, notFound)).toBe(false)
  })

  it('never retries other failures', () => {
    expect(shouldRetryShare(0, new ApiError(404, 'user_not_found'))).toBe(false)
    expect(shouldRetryShare(0, new ApiError(422, 'cannot_share_with_owner'))).toBe(false)
    expect(shouldRetryShare(0, new ApiError(500, 'internal'))).toBe(false)
    expect(shouldRetryShare(0, new Error('boom'))).toBe(false)
  })
})

describe('isGrantable', () => {
  it('accepts the three grantable roles only', () => {
    expect(['viewer', 'commenter', 'editor'].every((role) => isGrantable(role))).toBe(true)
    expect(isGrantable('owner')).toBe(false)
    expect(isGrantable('')).toBe(false)
  })
})
