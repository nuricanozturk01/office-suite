import { ref } from 'vue'

import type { FileRef } from './file-kind'
import { useDeleteFile } from './use-delete-file'
import { useDownloadFile } from './use-download-file'

/** Browsers drop downloads that start in the same instant, so they are spaced out. */
const DOWNLOAD_GAP_MS = 350

function pause(ms: number): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, ms)
  })
}

/** Runs one action over many files and reports how many failed. */
export function useBulkActions() {
  const deleteFile = useDeleteFile()
  const downloadFile = useDownloadFile()
  const busy = ref(false)
  const failed = ref(0)

  /** Deletes all files; resolves with the ones that were deleted. */
  async function removeAll(files: FileRef[]): Promise<FileRef[]> {
    busy.value = true
    failed.value = 0
    const results = await Promise.allSettled(files.map((file) => deleteFile.mutateAsync(file)))
    failed.value = results.filter((result) => result.status === 'rejected').length
    busy.value = false
    return files.filter((_, index) => results[index]?.status === 'fulfilled')
  }

  async function downloadAll(files: FileRef[]): Promise<void> {
    busy.value = true
    failed.value = 0
    for (const file of files) {
      try {
        await downloadFile.mutateAsync(file.id)
      } catch {
        failed.value += 1
      }
      await pause(DOWNLOAD_GAP_MS)
    }
    busy.value = false
  }

  return { busy, failed, removeAll, downloadAll, removeOne: deleteFile }
}
