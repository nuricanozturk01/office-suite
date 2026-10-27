export type FileKind = 'document' | 'spreadsheet' | 'presentation' | 'pdf'

/** Points at one file of any kind. */
export interface FileRef {
  kind: FileKind
  id: string
}
