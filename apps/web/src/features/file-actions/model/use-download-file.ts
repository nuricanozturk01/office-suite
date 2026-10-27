import { useMutation } from '@tanstack/vue-query'

import { readPackage } from '@/entities/package'
import { ApiError } from '@/shared/api'

/** Thrown when a file was created but never saved, so there is nothing to download yet. */
export class NothingToDownloadError extends Error {
  constructor() {
    super('nothing_to_download')
  }
}

/** Saves the stored file (`.docx`, `.xlsx`, `.pptx` or `.pdf`) of a resource through the browser. */
export function useDownloadFile() {
  return useMutation({
    mutationFn: async (id: string) => {
      const stored = await readPackage(id)
      if (stored === null) {
        throw new NothingToDownloadError()
      }
      const url = URL.createObjectURL(new Blob([stored.bytes]))
      const link = document.createElement('a')
      link.href = url
      link.download = stored.name
      link.click()
      URL.revokeObjectURL(url)
    },
  })
}

export function isNothingToDownload(error: unknown): boolean {
  return (
    error instanceof NothingToDownloadError || (error instanceof ApiError && error.status === 404)
  )
}
