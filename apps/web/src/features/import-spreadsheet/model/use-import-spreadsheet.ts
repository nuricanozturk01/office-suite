import { useMutation, useQueryClient } from '@tanstack/vue-query'

import { spreadsheetKeys, type SpreadsheetDetail } from '@/entities/spreadsheet'
import { storeOriginalFile } from '@/entities/package'
import { api, unwrap } from '@/shared/api'

const EXTENSION = /\.xlsx$/i

/** Uploads an `.xlsx` file as a new spreadsheet and primes the cache with the result. */
export function useImportSpreadsheet(onImported: (spreadsheet: SpreadsheetDetail) => void) {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (file: File) => {
      const imported = await unwrap(
        api.POST('/api/v1/spreadsheets/import', {
          params: { query: { name: file.name.replace(EXTENSION, '') } },
          // The endpoint takes the raw file; the generated body type only documents it.
          body: '',
          bodySerializer: () => file,
          headers: { 'Content-Type': 'application/octet-stream' },
        }),
      )
      // Keep the file exactly as uploaded; the editor opens this, not the converted copy.
      await storeOriginalFile(imported.spreadsheet.id, file)
      return imported
    },
    onSuccess: async ({ spreadsheet }) => {
      queryClient.setQueryData(spreadsheetKeys.detail(spreadsheet.id), spreadsheet)
      await queryClient.invalidateQueries({ queryKey: spreadsheetKeys.lists() })
      onImported(spreadsheet)
    },
  })
}
