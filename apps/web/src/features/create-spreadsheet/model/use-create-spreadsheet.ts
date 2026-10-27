import { useMutation, useQueryClient } from '@tanstack/vue-query'
import { useI18n } from 'vue-i18n'

import { spreadsheetKeys, type SpreadsheetDetail } from '@/entities/spreadsheet'
import { api, unwrap } from '@/shared/api'

/** Creates an untitled spreadsheet and primes the cache so the grid opens without a refetch. */
export function useCreateSpreadsheet(onCreated: (spreadsheet: SpreadsheetDetail) => void) {
  const { t } = useI18n()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: () =>
      unwrap(
        api.POST('/api/v1/spreadsheets', {
          body: { title: t('sheets.untitled'), sheet_name: t('sheets.defaultSheet') },
        }),
      ),
    onSuccess: async (spreadsheet) => {
      queryClient.setQueryData(spreadsheetKeys.detail(spreadsheet.id), spreadsheet)
      await queryClient.invalidateQueries({ queryKey: spreadsheetKeys.lists() })
      onCreated(spreadsheet)
    },
  })
}
