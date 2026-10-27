import { useMutation, useQueryClient } from '@tanstack/vue-query'

import { documentKeys, type DocumentSummary, setPendingImport } from '@/entities/document'
import { storeOriginalFile } from '@/entities/package'
import { api, unwrap } from '@/shared/api'

const EXTENSION = /\.docx$/i

/** Converts a `.docx` file, creates a document for it and hands the content to its editor. */
export function useImportDocument(onImported: (document: DocumentSummary) => void) {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (file: File) => {
      const { content } = await unwrap(
        api.POST('/api/v1/documents/import-docx', {
          // The endpoint takes the raw file; the generated body type only documents it.
          body: '',
          bodySerializer: () => file,
          headers: { 'Content-Type': 'application/octet-stream' },
        }),
      )
      const document = await unwrap(
        api.POST('/api/v1/documents', { body: { title: file.name.replace(EXTENSION, '') } }),
      )
      setPendingImport(document.id, content)
      await storeOriginalFile(document.id, file)
      return document
    },
    onSuccess: async (document) => {
      queryClient.setQueryData(documentKeys.detail(document.id), document)
      await queryClient.invalidateQueries({ queryKey: documentKeys.lists() })
      onImported(document)
    },
  })
}
