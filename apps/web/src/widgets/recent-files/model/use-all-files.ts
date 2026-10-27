import { computed } from 'vue'

import { type DocumentSummary, useDocumentsQuery } from '@/entities/document'
import { type PdfSummary, usePdfsQuery } from '@/entities/pdf'
import { type PresentationSummary, usePresentationsQuery } from '@/entities/presentation'
import { type SpreadsheetSummary, useSpreadsheetsQuery } from '@/entities/spreadsheet'

export type LibraryFile =
  | { kind: 'document'; item: DocumentSummary; updatedAt: string }
  | { kind: 'spreadsheet'; item: SpreadsheetSummary; updatedAt: string }
  | { kind: 'presentation'; item: PresentationSummary; updatedAt: string }
  | { kind: 'pdf'; item: PdfSummary; updatedAt: string }

/** The user's files of every kind as one list, newest edit first, with shared paging state. */
export function useAllFiles() {
  const documents = useDocumentsQuery()
  const spreadsheets = useSpreadsheetsQuery()
  const presentations = usePresentationsQuery()
  const pdfs = usePdfsQuery()
  const queries = [documents, spreadsheets, presentations, pdfs]

  const files = computed(() => {
    const docs: LibraryFile[] = (
      documents.data.value?.pages.flatMap((page) => page.items) ?? []
    ).map((item) => ({ kind: 'document', item, updatedAt: item.updated_at }))
    const sheets: LibraryFile[] = (
      spreadsheets.data.value?.pages.flatMap((page) => page.items) ?? []
    ).map((item) => ({ kind: 'spreadsheet', item, updatedAt: item.updated_at }))
    const decks: LibraryFile[] = (
      presentations.data.value?.pages.flatMap((page) => page.items) ?? []
    ).map((item) => ({ kind: 'presentation', item, updatedAt: item.updated_at }))
    const pdfFiles: LibraryFile[] = (
      pdfs.data.value?.pages.flatMap((page) => page.items) ?? []
    ).map((item) => ({ kind: 'pdf', item, updatedAt: item.updated_at }))
    return [...docs, ...sheets, ...decks, ...pdfFiles].sort((a, b) =>
      b.updatedAt.localeCompare(a.updatedAt),
    )
  })

  const error = computed(
    () =>
      documents.error.value ??
      spreadsheets.error.value ??
      presentations.error.value ??
      pdfs.error.value,
  )
  const failed = computed(() => queries.some((query) => query.isError.value))
  const ready = computed(() => queries.every((query) => query.isSuccess.value))
  const hasMore = computed(() => queries.some((query) => query.hasNextPage.value))
  const loadingMore = computed(() => queries.some((query) => query.isFetchingNextPage.value))

  function loadMore(): void {
    for (const query of queries) {
      if (query.hasNextPage.value) {
        void query.fetchNextPage()
      }
    }
  }

  return { files, error, failed, ready, hasMore, loadingMore, loadMore }
}
