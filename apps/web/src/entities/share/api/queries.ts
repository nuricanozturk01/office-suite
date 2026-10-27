import { useInfiniteQuery, useQuery } from '@tanstack/vue-query'
import { type MaybeRefOrGetter, toValue } from 'vue'

import { api, type Schemas, unwrap } from '@/shared/api'

import { shareKeys } from './keys'

export type Share = Schemas['ShareResponse']
export type SharedFile = Schemas['SharedItemResponse']
export type Role = Share['role']

const PAGE_SIZE = 50
const FIRST_PAGE = ''

/** People a file is shared with; only the owner may read this (others get 404). */
export function useSharesQuery(
  fileId: MaybeRefOrGetter<string>,
  enabled: MaybeRefOrGetter<boolean> = true,
) {
  return useQuery({
    queryKey: () => shareKeys.file(toValue(fileId)),
    queryFn: () =>
      unwrap(api.GET('/api/v1/files/{id}/shares', { params: { path: { id: toValue(fileId) } } })),
    enabled: () => toValue(enabled),
  })
}

/** Files other people shared with the signed-in user, newest first. */
export function useSharedWithMeQuery() {
  return useInfiniteQuery({
    queryKey: shareKeys.sharedWithMe(),
    initialPageParam: FIRST_PAGE,
    queryFn: ({ pageParam }) =>
      unwrap(
        api.GET('/api/v1/files/shared-with-me', {
          params: {
            query:
              pageParam === FIRST_PAGE
                ? { limit: PAGE_SIZE }
                : { cursor: pageParam, limit: PAGE_SIZE },
          },
        }),
      ),
    getNextPageParam: (page) => page.next_cursor ?? undefined,
  })
}
