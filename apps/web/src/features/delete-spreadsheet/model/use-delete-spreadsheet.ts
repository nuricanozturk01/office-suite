import { useMutation, useQueryClient } from '@tanstack/vue-query'
import { useI18n } from 'vue-i18n'

import { spreadsheetKeys, type SpreadsheetSummary } from '@/entities/spreadsheet'
import { api, expectOk } from '@/shared/api'

/** Deletes a spreadsheet after the user confirms; cache entries for it are dropped on success. */
export function useDeleteSpreadsheet() {
  const { t } = useI18n()
  const queryClient = useQueryClient()

  const mutation = useMutation({
    mutationFn: (id: string) =>
      expectOk(api.DELETE('/api/v1/spreadsheets/{id}', { params: { path: { id } } })),
    onSuccess: async (_, id) => {
      queryClient.removeQueries({ queryKey: spreadsheetKeys.detail(id) })
      await queryClient.invalidateQueries({ queryKey: spreadsheetKeys.lists() })
    },
  })

  function confirmAndDelete(spreadsheet: SpreadsheetSummary): void {
    const title = spreadsheet.title || t('sheets.untitled')
    if (window.confirm(t('files.deleteConfirm', { title }))) {
      mutation.mutate(spreadsheet.id)
    }
  }

  return { mutation, confirmAndDelete }
}
