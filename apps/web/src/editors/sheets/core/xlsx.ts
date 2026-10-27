import { api, unwrap } from '@/shared/api'

/** Fetches the spreadsheet as an `.xlsx` file and saves it through the browser. */
export async function downloadXlsx(spreadsheetId: string, fileName: string): Promise<void> {
  const blob = await unwrap(
    api.GET('/api/v1/spreadsheets/{id}/export.xlsx', {
      params: { path: { id: spreadsheetId } },
      parseAs: 'blob',
    }),
  )
  const url = URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = url
  link.download = `${fileName}.xlsx`
  link.click()
  URL.revokeObjectURL(url)
}
