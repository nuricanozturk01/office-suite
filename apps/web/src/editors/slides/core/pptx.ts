import { api, unwrap } from '@/shared/api'

/** Fetches the presentation as a `.pptx` file and saves it through the browser. */
export async function downloadPptx(presentationId: string, fileName: string): Promise<void> {
  const blob = await unwrap(
    api.GET('/api/v1/presentations/{id}/export.pptx', {
      params: { path: { id: presentationId } },
      parseAs: 'blob',
    }),
  )
  const url = URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = url
  link.download = `${fileName}.pptx`
  link.click()
  URL.revokeObjectURL(url)
}
