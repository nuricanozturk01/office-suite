import type { JSONContent } from '@tiptap/core'

const pending = new Map<string, JSONContent>()

/** Remembers converted file content until the editor of the new document has synced. */
export function setPendingImport(documentId: string, content: JSONContent): void {
  pending.set(documentId, content)
}

/** Returns the content stored for `documentId` once. */
export function takePendingImport(documentId: string): JSONContent | undefined {
  const content = pending.get(documentId)
  pending.delete(documentId)
  return content
}
