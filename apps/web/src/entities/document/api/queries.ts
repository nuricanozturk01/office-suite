import { useInfiniteQuery, useQuery } from '@tanstack/vue-query'
import { type MaybeRefOrGetter, toValue } from 'vue'

import { api, type Schemas, unwrap } from '@/shared/api'

import { documentKeys } from './keys'

export type DocumentSummary = Schemas['DocumentResponse']

const PAGE_SIZE = 50
const FIRST_PAGE = ''

export function useDocumentsQuery() {
  return useInfiniteQuery({
    queryKey: documentKeys.lists(),
    // The file list shows what was edited last; saves happen in the editors, so refetch on return.
    refetchOnMount: 'always',
    initialPageParam: FIRST_PAGE,
    queryFn: ({ pageParam }) =>
      unwrap(
        api.GET('/api/v1/documents', {
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

export function useDocumentQuery(id: MaybeRefOrGetter<string>) {
  return useQuery({
    queryKey: () => documentKeys.detail(toValue(id)),
    queryFn: () =>
      unwrap(api.GET('/api/v1/documents/{id}', { params: { path: { id: toValue(id) } } })),
  })
}
