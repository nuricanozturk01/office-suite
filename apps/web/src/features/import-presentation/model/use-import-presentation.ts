import { useMutation, useQueryClient } from '@tanstack/vue-query'

import { presentationKeys, type PresentationDetail } from '@/entities/presentation'
import { storeOriginalFile } from '@/entities/package'
import { api, unwrap } from '@/shared/api'

const EXTENSION = /\.pptx$/i

/** Uploads an `.pptx` file as a new presentation and primes the cache with the result. */
export function useImportPresentation(onImported: (presentation: PresentationDetail) => void) {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (file: File) => {
      const imported = await unwrap(
        api.POST('/api/v1/presentations/import', {
          params: { query: { name: file.name.replace(EXTENSION, '') } },
          // The endpoint takes the raw file; the generated body type only documents it.
          body: '',
          bodySerializer: () => file,
          headers: { 'Content-Type': 'application/octet-stream' },
        }),
      )
      // Keep the file exactly as uploaded; the editor opens this, not the converted copy.
      await storeOriginalFile(imported.presentation.id, file)
      return imported
    },
    onSuccess: async ({ presentation }) => {
      queryClient.setQueryData(presentationKeys.detail(presentation.id), presentation)
      await queryClient.invalidateQueries({ queryKey: presentationKeys.lists() })
      onImported(presentation)
    },
  })
}
