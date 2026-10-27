import type { Downloader } from '../core/ports'

/** Adapter: gives a file to the user's computer through the browser's download. */
export const browserDownloader: Downloader = {
  download: (file) => {
    const url = URL.createObjectURL(new Blob([file.bytes]))
    const link = document.createElement('a')
    link.href = url
    link.download = file.name
    link.click()
    URL.revokeObjectURL(url)
  },
}
