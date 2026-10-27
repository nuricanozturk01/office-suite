import type { JSONContent } from '@tiptap/core'

import { api, unwrap } from '@/shared/api'

import type { EmbeddedAppId } from '../core/apps'

const EMPTY_DOCUMENT: JSONContent = { type: 'doc', content: [{ type: 'paragraph' }] }

/**
 * A starting file for a resource that has none stored yet, made by the backend converters from
 * the resource's own (empty) content. `null` for apps that open on their own start screen.
 */
export async function blankFile(
  app: EmbeddedAppId,
  resourceId: string,
  title: string,
): Promise<{ name: string; bytes: ArrayBuffer } | null> {
  const name = title.trim() === '' ? 'Untitled' : title.trim().replaceAll(/[\\/]+/g, ' ')
  switch (app) {
    case 'wordcraft': {
      const blob = await unwrap(
        api.POST('/api/v1/documents/export-docx', {
          body: { title: name, content: EMPTY_DOCUMENT },
          parseAs: 'blob',
        }),
      )
      return { name: `${name}.docx`, bytes: await blob.arrayBuffer() }
    }
    case 'gridcraft': {
      const blob = await unwrap(
        api.GET('/api/v1/spreadsheets/{id}/export.xlsx', {
          params: { path: { id: resourceId } },
          parseAs: 'blob',
        }),
      )
      return { name: `${name}.xlsx`, bytes: await blob.arrayBuffer() }
    }
    case 'deckcraft': {
      const blob = await unwrap(
        api.GET('/api/v1/presentations/{id}/export.pptx', {
          params: { path: { id: resourceId } },
          parseAs: 'blob',
        }),
      )
      return { name: `${name}.pptx`, bytes: await blob.arrayBuffer() }
    }
    case 'pdfcraft':
      return null
  }
}
