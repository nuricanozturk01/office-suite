import { useMutation, useQueryClient } from '@tanstack/vue-query'

import { storeOriginalFile } from '@/entities/package'
import { pdfKeys, type PdfSummary } from '@/entities/pdf'
import { api, unwrap } from '@/shared/api'

const EXTENSION = /\.pdf$/i

/** Creates a PDF resource for an uploaded file and stores the file exactly as uploaded. */
export function useImportPdf(onImported: (pdf: PdfSummary) => void) {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (file: File) => {
      const pdf = await unwrap(
        api.POST('/api/v1/pdfs', { body: { title: file.name.replace(EXTENSION, '') } }),
      )
      await storeOriginalFile(pdf.id, file)
      return pdf
    },
    onSuccess: async (pdf) => {
      queryClient.setQueryData(pdfKeys.detail(pdf.id), pdf)
      await queryClient.invalidateQueries({ queryKey: pdfKeys.lists() })
      onImported(pdf)
    },
  })
}
