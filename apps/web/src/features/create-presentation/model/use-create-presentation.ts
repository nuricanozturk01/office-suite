import { useMutation, useQueryClient } from '@tanstack/vue-query'
import { useI18n } from 'vue-i18n'

import { presentationKeys, type PresentationDetail } from '@/entities/presentation'
import { api, unwrap } from '@/shared/api'

/** Creates an untitled presentation and primes the cache so the grid opens without a refetch. */
export function useCreatePresentation(onCreated: (presentation: PresentationDetail) => void) {
  const { t } = useI18n()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: () =>
      unwrap(
        api.POST('/api/v1/presentations', {
          body: { title: t('slides.untitled') },
        }),
      ),
    onSuccess: async (presentation) => {
      queryClient.setQueryData(presentationKeys.detail(presentation.id), presentation)
      await queryClient.invalidateQueries({ queryKey: presentationKeys.lists() })
      onCreated(presentation)
    },
  })
}
