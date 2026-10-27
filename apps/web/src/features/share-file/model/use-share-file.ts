import { useMutation, useQueryClient } from '@tanstack/vue-query'
import { type MaybeRefOrGetter, toValue } from 'vue'

import { shareKeys } from '@/entities/share'
import { api, ApiError, expectOk, type Schemas, unwrap } from '@/shared/api'

export type GrantableRole = Schemas['ShareBody']['role']

/** Roles an owner can hand out, in the order the UI lists them. */
export const grantableRoles: readonly GrantableRole[] = ['viewer', 'commenter', 'editor']

export function isGrantable(value: string): value is GrantableRole {
  return grantableRoles.some((role) => role === value)
}

const MAX_FRESH_FILE_RETRIES = 3
const RETRY_DELAY_MS = 700

/**
 * A file created a moment ago may not be known to the sharing module yet (it learns about files
 * from events), so a 404 right after creation is retried a few times before it is reported.
 */
export function shouldRetryShare(failureCount: number, error: unknown): boolean {
  return (
    failureCount < MAX_FRESH_FILE_RETRIES && error instanceof ApiError && error.code === 'not_found'
  )
}

/** Share and revoke mutations for one file; both refresh the file's people list. */
export function useShareFile(fileId: MaybeRefOrGetter<string>) {
  const queryClient = useQueryClient()
  const refresh = () => queryClient.invalidateQueries({ queryKey: shareKeys.file(toValue(fileId)) })

  const share = useMutation({
    mutationFn: (input: { email: string; role: GrantableRole }) =>
      unwrap(
        api.POST('/api/v1/files/{id}/shares', {
          params: { path: { id: toValue(fileId) } },
          body: input,
        }),
      ),
    retry: shouldRetryShare,
    retryDelay: RETRY_DELAY_MS,
    onSuccess: refresh,
  })

  const revoke = useMutation({
    mutationFn: (userId: string) =>
      expectOk(
        api.DELETE('/api/v1/files/{id}/shares/{user_id}', {
          params: { path: { id: toValue(fileId), user_id: userId } },
        }),
      ),
    onSuccess: refresh,
  })

  return { share, revoke }
}
