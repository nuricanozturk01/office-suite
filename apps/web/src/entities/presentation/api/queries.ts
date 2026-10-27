import { useInfiniteQuery, useQuery } from '@tanstack/vue-query'
import { type MaybeRefOrGetter, toValue } from 'vue'

import { api, type Schemas, unwrap } from '@/shared/api'

import { presentationKeys } from './keys'

export type PresentationSummary = Schemas['PresentationResponse']
export type PresentationDetail = Schemas['PresentationDetailResponse']

const PAGE_SIZE = 50
const FIRST_PAGE = ''

export function usePresentationsQuery() {
  return useInfiniteQuery({
    queryKey: presentationKeys.lists(),
    // The file list shows what was edited last; saves happen in the editors, so refetch on return.
    refetchOnMount: 'always',
    initialPageParam: FIRST_PAGE,
    queryFn: ({ pageParam }) =>
      unwrap(
        api.GET('/api/v1/presentations', {
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

export function usePresentationQuery(id: MaybeRefOrGetter<string>) {
  return useQuery({
    queryKey: () => presentationKeys.detail(toValue(id)),
    queryFn: () =>
      unwrap(api.GET('/api/v1/presentations/{id}', { params: { path: { id: toValue(id) } } })),
  })
}
