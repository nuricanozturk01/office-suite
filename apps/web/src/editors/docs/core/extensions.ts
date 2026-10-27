import type { Extensions } from '@tiptap/core'
import Collaboration from '@tiptap/extension-collaboration'
import { CollaborationCaret } from '@tiptap/extension-collaboration-caret'
import { Highlight } from '@tiptap/extension-highlight'
import { Image } from '@tiptap/extension-image'
import { TaskItem, TaskList } from '@tiptap/extension-list'
import { TableKit } from '@tiptap/extension-table'
import { Subscript } from '@tiptap/extension-subscript'
import { Superscript } from '@tiptap/extension-superscript'
import { TextAlign } from '@tiptap/extension-text-align'
import { TextStyleKit } from '@tiptap/extension-text-style'
import { CharacterCount, Placeholder } from '@tiptap/extensions'
import StarterKit from '@tiptap/starter-kit'
import type * as Y from 'yjs'

import type { CollabProvider } from './collab-provider'
import { PageBreak } from './page-break'
import type { HeaderFooter, PageGeometry } from './page-layout'
import { Pagination } from './pagination'
import { ParagraphFormat } from './paragraph-format'
import { SearchReplace } from './search-replace'

export interface DocsExtensionOptions {
  doc: Y.Doc
  provider: CollabProvider
  user: { name: string; color: string }
  placeholder: string
  geometry: () => PageGeometry
  headerFooter: () => HeaderFooter
  onPages: (count: number) => void
}

export function docsExtensions(options: DocsExtensionOptions): Extensions {
  return [
    StarterKit.configure({
      // History comes from Yjs (Collaboration) so undo is per-user, not per-document.
      undoRedo: false,
      heading: { levels: [1, 2, 3, 4] },
      link: { openOnClick: false, autolink: true, defaultProtocol: 'https' },
    }),
    TextStyleKit.configure({ lineHeight: false }),
    Subscript,
    Superscript,
    ParagraphFormat,
    PageBreak,
    SearchReplace,
    Pagination.configure({
      geometry: options.geometry,
      texts: options.headerFooter,
      onPages: options.onPages,
    }),
    Highlight.configure({ multicolor: true }),
    TextAlign.configure({ types: ['heading', 'paragraph'] }),
    TaskList,
    TaskItem.configure({ nested: true }),
    Image.configure({ inline: false, allowBase64: false }),
    TableKit.configure({ table: { resizable: true } }),
    CharacterCount,
    Placeholder.configure({ placeholder: options.placeholder }),
    Collaboration.configure({ document: options.doc }),
    CollaborationCaret.configure({ provider: options.provider, user: options.user }),
  ]
}
