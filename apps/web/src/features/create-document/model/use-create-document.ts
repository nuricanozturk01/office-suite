import { useMutation, useQueryClient } from '@tanstack/vue-query'
import { useI18n } from 'vue-i18n'

import { documentKeys, type DocumentSummary } from '@/entities/document'
import { api, unwrap } from '@/shared/api'

/** Creates an untitled document and primes the cache so the editor opens without a refetch. */
export function useCreateDocument(onCreated: (document: DocumentSummary) => void) {
  const { t } = useI18n()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: () =>
      unwrap(api.POST('/api/v1/documents', { body: { title: t('documents.untitled') } })),
    onSuccess: async (document) => {
      queryClient.setQueryData(documentKeys.detail(document.id), document)
      await queryClient.invalidateQueries({ queryKey: documentKeys.lists() })
      onCreated(document)
    },
  })
}
