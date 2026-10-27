import { useMutation, useQueryClient } from '@tanstack/vue-query'
import { useI18n } from 'vue-i18n'

import { presentationKeys, type PresentationSummary } from '@/entities/presentation'
import { api, expectOk } from '@/shared/api'

/** Deletes a presentation after the user confirms; cache entries for it are dropped on success. */
export function useDeletePresentation() {
  const { t } = useI18n()
  const queryClient = useQueryClient()

  const mutation = useMutation({
    mutationFn: (id: string) =>
      expectOk(api.DELETE('/api/v1/presentations/{id}', { params: { path: { id } } })),
    onSuccess: async (_, id) => {
      queryClient.removeQueries({ queryKey: presentationKeys.detail(id) })
      await queryClient.invalidateQueries({ queryKey: presentationKeys.lists() })
    },
  })

  function confirmAndDelete(presentation: PresentationSummary): void {
    const title = presentation.title || t('slides.untitled')
    if (window.confirm(t('files.deleteConfirm', { title }))) {
      mutation.mutate(presentation.id)
    }
  }

  return { mutation, confirmAndDelete }
}
