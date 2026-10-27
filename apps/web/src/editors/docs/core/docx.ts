import type { JSONContent } from '@tiptap/core'

import { api, unwrap } from '@/shared/api'

/** Converts the editor content to `.docx` on the server and saves it through the browser. */
export async function downloadDocx(
  title: string,
  content: JSONContent,
  fileName: string,
): Promise<void> {
  const blob = await unwrap(
    api.POST('/api/v1/documents/export-docx', {
      body: { title, content },
      parseAs: 'blob',
    }),
  )
  const url = URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = url
  link.download = fileName
  link.click()
  URL.revokeObjectURL(url)
}
