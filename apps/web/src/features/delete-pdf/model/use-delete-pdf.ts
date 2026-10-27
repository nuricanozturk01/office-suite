import { useMutation, useQueryClient } from '@tanstack/vue-query'
import { useI18n } from 'vue-i18n'

import { pdfKeys, type PdfSummary } from '@/entities/pdf'
import { api, expectOk } from '@/shared/api'

/** Deletes a pdf after the user confirms; cache entries for it are dropped on success. */
export function useDeletePdf() {
  const { t } = useI18n()
  const queryClient = useQueryClient()

  const mutation = useMutation({
    mutationFn: (id: string) =>
      expectOk(api.DELETE('/api/v1/pdfs/{id}', { params: { path: { id } } })),
    onSuccess: async (_, id) => {
      queryClient.removeQueries({ queryKey: pdfKeys.detail(id) })
      await queryClient.invalidateQueries({ queryKey: pdfKeys.lists() })
    },
  })

  function confirmAndDelete(pdf: PdfSummary): void {
    const title = pdf.title || t('pdf.untitled')
    if (window.confirm(t('pdf.deleteConfirm', { title }))) {
      mutation.mutate(pdf.id)
    }
  }

  return { mutation, confirmAndDelete }
}
