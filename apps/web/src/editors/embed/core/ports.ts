/**
 * Ports of the document editors. The editors (the wordcraft, gridcraft and deckcraft engines
 * built as web libraries) know nothing about this product: they only speak this protocol, and
 * the product only knows these interfaces. Adapters in `../adapters` plug the two together.
 */

export interface EditorFile {
  name: string
  bytes: ArrayBuffer
}

export type SaveStatus = 'loading' | 'saved' | 'saving' | 'dirty' | 'error' | 'conflict' | 'failed'

/** File-level commands an editor asks its host to perform instead of doing them on a disk. */
export type EditorCommand = 'new' | 'open' | 'close' | 'saveAs' | 'share'

export type EngineEvent =
  | { type: 'ready' }
  | { type: 'save'; file: EditorFile }
  | { type: 'dirty'; dirty: boolean }
  | { type: 'command'; command: EditorCommand }
  /** The user renamed the document inside the editor (`name` has no extension). */
  | { type: 'renamed'; name: string }
  | { type: 'failed' }

/** Driven port: what the product can ask of an editor, and what the editor reports back. */
export interface EditorEngine {
  /** Loads a file into the editor. */
  open: (file: EditorFile) => void
  /** Asks the editor to hand over its current file as a `save` event. */
  requestSave: () => void
  subscribe: (listener: (event: EngineEvent) => void) => () => void
}

/** Driven port: where the document of a resource lives. */
export interface FileStore {
  /** The stored file and its version, or `null` for a resource that has none yet. */
  load: () => Promise<{ file: EditorFile; version: number } | null>
  /** A starting file for a resource without one, or `null` when the editor has its own start. */
  blank: () => Promise<EditorFile | null>
  /**
   * Stores the file; `baseVersion` is the version it was edited from (`null` for the first).
   * A store that can combine the file with newer saves does so and reports `merged`.
   */
  save: (file: EditorFile, baseVersion: number | null) => Promise<SavedFile>
}

export interface SavedFile {
  version: number
  /** Others' changes were merged into the stored file; the editor should load it. */
  merged: boolean
}

/** Thrown by a [`FileStore`] when the stored file changed since it was loaded. */
export class VersionConflictError extends Error {
  override readonly name = 'VersionConflictError'
}

/** Driven port: hands a file to the user's computer. */
export interface Downloader {
  download: (file: EditorFile) => void
}
