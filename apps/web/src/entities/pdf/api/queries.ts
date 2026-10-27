import { useInfiniteQuery, useQuery } from '@tanstack/vue-query'
import { type MaybeRefOrGetter, toValue } from 'vue'

import { api, type Schemas, unwrap } from '@/shared/api'

import { pdfKeys } from './keys'

export type PdfSummary = Schemas['PdfResponse']

const PAGE_SIZE = 50
const FIRST_PAGE = ''

export function usePdfsQuery() {
  return useInfiniteQuery({
    queryKey: pdfKeys.lists(),
    // The file list shows what was edited last; saves happen in the editors, so refetch on return.
    refetchOnMount: 'always',
    initialPageParam: FIRST_PAGE,
    queryFn: ({ pageParam }) =>
      unwrap(
        api.GET('/api/v1/pdfs', {
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

export function usePdfQuery(id: MaybeRefOrGetter<string>) {
  return useQuery({
    queryKey: () => pdfKeys.detail(toValue(id)),
    queryFn: () => unwrap(api.GET('/api/v1/pdfs/{id}', { params: { path: { id: toValue(id) } } })),
  })
}
