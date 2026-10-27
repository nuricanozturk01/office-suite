import { useMutation, useQueryClient } from '@tanstack/vue-query'

import { documentKeys } from '@/entities/document'
import { pdfKeys } from '@/entities/pdf'
import { presentationKeys } from '@/entities/presentation'
import { spreadsheetKeys } from '@/entities/spreadsheet'
import { api, expectOk } from '@/shared/api'

import type { FileRef } from './file-kind'

function remove({ kind, id }: FileRef): Promise<void> {
  const params = { path: { id } }
  switch (kind) {
    case 'document':
      return expectOk(api.DELETE('/api/v1/documents/{id}', { params }))
    case 'spreadsheet':
      return expectOk(api.DELETE('/api/v1/spreadsheets/{id}', { params }))
    case 'presentation':
      return expectOk(api.DELETE('/api/v1/presentations/{id}', { params }))
    case 'pdf':
      return expectOk(api.DELETE('/api/v1/pdfs/{id}', { params }))
  }
}

const LIST_KEYS = {
  document: () => documentKeys.lists(),
  spreadsheet: () => spreadsheetKeys.lists(),
  presentation: () => presentationKeys.lists(),
  pdf: () => pdfKeys.lists(),
} as const

/** Deletes a file of any kind; the caller asks for confirmation first. */
export function useDeleteFile() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: remove,
    onSuccess: async (_, { kind }) => {
      await queryClient.invalidateQueries({ queryKey: LIST_KEYS[kind]() })
    },
  })
}
