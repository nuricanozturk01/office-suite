import { mergeAttributes, Node } from '@tiptap/core'

declare module '@tiptap/core' {
  interface Commands<ReturnType> {
    pageBreak: {
      /** Inserts a manual page break; the next block starts on a new page. */
      setPageBreak: () => ReturnType
    }
  }
}

export const PAGE_BREAK = 'pageBreak'

export const PageBreak = Node.create({
  name: PAGE_BREAK,
  group: 'block',
  atom: true,
  selectable: true,

  parseHTML() {
    return [{ tag: 'div[data-type="page-break"]' }]
  },

  renderHTML({ HTMLAttributes }) {
    return [
      'div',
      mergeAttributes(HTMLAttributes, { 'data-type': 'page-break', class: 'page-break' }),
    ]
  },

  addCommands() {
    return {
      setPageBreak:
        () =>
        ({ chain }) =>
          chain().insertContent({ type: PAGE_BREAK }).run(),
    }
  },
})
