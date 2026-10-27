import { useMutation, useQueryClient } from '@tanstack/vue-query'

import { documentKeys } from '@/entities/document'
import { pdfKeys } from '@/entities/pdf'
import { presentationKeys } from '@/entities/presentation'
import { spreadsheetKeys } from '@/entities/spreadsheet'
import { api, unwrap } from '@/shared/api'

import type { FileKind } from './file-kind'

interface RenameInput {
  kind: FileKind
  id: string
  title: string
}

function patchTitle({ kind, id, title }: RenameInput): Promise<unknown> {
  const params = { path: { id } }
  const body = { title }
  switch (kind) {
    case 'document':
      return unwrap(api.PATCH('/api/v1/documents/{id}', { params, body }))
    case 'spreadsheet':
      return unwrap(api.PATCH('/api/v1/spreadsheets/{id}', { params, body }))
    case 'presentation':
      return unwrap(api.PATCH('/api/v1/presentations/{id}', { params, body }))
    case 'pdf':
      return unwrap(api.PATCH('/api/v1/pdfs/{id}', { params, body }))
  }
}

const LIST_KEYS = {
  document: () => documentKeys.lists(),
  spreadsheet: () => spreadsheetKeys.lists(),
  presentation: () => presentationKeys.lists(),
  pdf: () => pdfKeys.lists(),
} as const

/** Renames a file from the file list, whatever its kind. */
export function useRenameFile() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: patchTitle,
    onSuccess: async (_, { kind }) => {
      await queryClient.invalidateQueries({ queryKey: LIST_KEYS[kind]() })
    },
  })
}
