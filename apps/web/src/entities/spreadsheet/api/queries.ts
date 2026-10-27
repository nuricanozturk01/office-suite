import { useInfiniteQuery, useQuery } from '@tanstack/vue-query'
import { type MaybeRefOrGetter, toValue } from 'vue'

import { api, type Schemas, unwrap } from '@/shared/api'

import { spreadsheetKeys } from './keys'

export type SpreadsheetSummary = Schemas['SpreadsheetResponse']
export type SpreadsheetDetail = Schemas['SpreadsheetDetailResponse']

const PAGE_SIZE = 50
const FIRST_PAGE = ''

export function useSpreadsheetsQuery() {
  return useInfiniteQuery({
    queryKey: spreadsheetKeys.lists(),
    // The file list shows what was edited last; saves happen in the editors, so refetch on return.
    refetchOnMount: 'always',
    initialPageParam: FIRST_PAGE,
    queryFn: ({ pageParam }) =>
      unwrap(
        api.GET('/api/v1/spreadsheets', {
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

export function useSpreadsheetQuery(id: MaybeRefOrGetter<string>) {
  return useQuery({
    queryKey: () => spreadsheetKeys.detail(toValue(id)),
    queryFn: () =>
      unwrap(api.GET('/api/v1/spreadsheets/{id}', { params: { path: { id: toValue(id) } } })),
  })
}
