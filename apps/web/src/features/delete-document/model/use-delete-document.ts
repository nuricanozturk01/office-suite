import { useMutation, useQueryClient } from '@tanstack/vue-query'
import { useI18n } from 'vue-i18n'

import { documentKeys, type DocumentSummary } from '@/entities/document'
import { api, expectOk } from '@/shared/api'

/** Deletes a document after the user confirms; cache entries for it are dropped on success. */
export function useDeleteDocument() {
  const { t } = useI18n()
  const queryClient = useQueryClient()

  const mutation = useMutation({
    mutationFn: (id: string) =>
      expectOk(api.DELETE('/api/v1/documents/{id}', { params: { path: { id } } })),
    onSuccess: async (_, id) => {
      queryClient.removeQueries({ queryKey: documentKeys.detail(id) })
      await queryClient.invalidateQueries({ queryKey: documentKeys.lists() })
    },
  })

  function confirmAndDelete(document: DocumentSummary): void {
    const title = document.title || t('documents.untitled')
    if (window.confirm(t('documents.deleteConfirm', { title }))) {
      mutation.mutate(document.id)
    }
  }

  return { mutation, confirmAndDelete }
}
