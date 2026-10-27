export type EmbeddedAppId = 'wordcraft' | 'gridcraft' | 'deckcraft' | 'pdfcraft'

export interface EmbeddedAppConfig {
  /** Message type prefix of the app's `?host=parent` bridge (`<prefix>:ready`, ...). */
  prefix: EmbeddedAppId
  /** Id of the editor's canvas element (where key presses are sent). */
  canvasId: string
  /** Extensions the app saves as the document itself; anything else is an export (a download). */
  stored: readonly string[]
}

export const EMBEDDED_APPS: Record<EmbeddedAppId, EmbeddedAppConfig> = {
  wordcraft: {
    prefix: 'wordcraft',
    canvasId: 'wordcraft_canvas',
    // Only `.docx` is the document; `.odt` and `.rtf` come from File > Export and are copies.
    stored: ['docx'],
  },
  gridcraft: { prefix: 'gridcraft', canvasId: 'gridcraft_canvas', stored: ['xlsx'] },
  deckcraft: { prefix: 'deckcraft', canvasId: 'deckcraft_canvas', stored: ['pptx'] },
  pdfcraft: { prefix: 'pdfcraft', canvasId: 'pdfcraft', stored: ['pdf'] },
}

export function extensionOf(name: string): string {
  const dot = name.lastIndexOf('.')
  return dot < 0 ? '' : name.slice(dot + 1).toLowerCase()
}

/** Whether a file the app sends replaces the stored document rather than being a copy. */
export function isStoredFormat(app: EmbeddedAppConfig, name: string): boolean {
  return app.stored.includes(extensionOf(name))
}
