import type { Editor } from '@tiptap/core'
import type { Mark } from '@tiptap/pm/model'

import type { DocStyle } from './paragraph-format'

export type StyleId =
  'normal' | 'noSpacing' | 'heading1' | 'heading2' | 'heading3' | 'title' | 'subtitle' | 'quote'

export const STYLE_IDS: readonly StyleId[] = [
  'normal',
  'noSpacing',
  'heading1',
  'title',
  'heading2',
  'subtitle',
  'heading3',
  'quote',
]

const HEADINGS = { heading1: 1, heading2: 2, heading3: 3 } as const
const DOC_STYLES: Partial<Record<StyleId, DocStyle>> = {
  noSpacing: 'noSpacing',
  title: 'title',
  subtitle: 'subtitle',
  quote: 'quote',
}

export function currentStyle(editor: Editor): StyleId {
  for (const [id, level] of Object.entries(HEADINGS)) {
    if (editor.isActive('heading', { level })) {
      return id as StyleId // keys of HEADINGS
    }
  }
  const docStyle: unknown = editor.getAttributes('paragraph').docStyle
  const found = STYLE_IDS.find((id) => DOC_STYLES[id] === docStyle)
  return found ?? 'normal'
}

export function applyStyle(editor: Editor, id: StyleId): void {
  const chain = editor.chain().focus()
  if (id === 'heading1' || id === 'heading2' || id === 'heading3') {
    chain.setHeading({ level: HEADINGS[id] }).setDocStyle(null).run()
    return
  }
  chain
    .setParagraph()
    .setDocStyle(DOC_STYLES[id] ?? null)
    .run()
}

export type CaseMode = 'sentence' | 'lower' | 'upper' | 'title' | 'toggle'

function convertCase(text: string, mode: CaseMode, locale: string): string {
  switch (mode) {
    case 'lower':
      return text.toLocaleLowerCase(locale)
    case 'upper':
      return text.toLocaleUpperCase(locale)
    case 'title':
      return text.replaceAll(
        /\p{L}[\p{L}\p{N}']*/gu,
        (word) =>
          word.charAt(0).toLocaleUpperCase(locale) + word.slice(1).toLocaleLowerCase(locale),
      )
    case 'sentence':
      return text
        .toLocaleLowerCase(locale)
        .replaceAll(
          /(^|[.!?]\s+)(\p{L})/gu,
          (_, lead: string, letter: string) => lead + letter.toLocaleUpperCase(locale),
        )
    case 'toggle':
      return Array.from(text)
        .map((char) =>
          char === char.toLocaleUpperCase(locale)
            ? char.toLocaleLowerCase(locale)
            : char.toLocaleUpperCase(locale),
        )
        .join('')
  }
}

/** Changes the case of the selected text, keeping its formatting. */
export function changeCase(editor: Editor, mode: CaseMode, locale: string): void {
  const { from, to, empty } = editor.state.selection
  if (empty) {
    return
  }
  const { tr, schema } = editor.state
  const edits: { pos: number; end: number; text: string; marks: readonly Mark[] }[] = []
  editor.state.doc.nodesBetween(from, to, (node, pos) => {
    if (node.isText && node.text !== undefined) {
      const start = Math.max(from, pos)
      const end = Math.min(to, pos + node.text.length)
      edits.push({
        pos: start,
        end,
        text: convertCase(node.text.slice(start - pos, end - pos), mode, locale),
        marks: node.marks,
      })
    }
    return true
  })
  for (const edit of edits.reverse()) {
    tr.replaceWith(edit.pos, edit.end, schema.text(edit.text, edit.marks))
  }
  editor.view.dispatch(tr)
  editor.commands.focus()
}

export function clearAllFormatting(editor: Editor): void {
  editor.chain().focus().unsetAllMarks().setParagraph().setDocStyle(null).setLineSpacing(null).run()
}

/** Reads the clipboard (needs a user gesture and permission) and inserts its text. */
export async function pasteFromClipboard(editor: Editor): Promise<void> {
  const text = await navigator.clipboard.readText()
  if (text !== '') {
    editor.chain().focus().insertContent(text).run()
  }
}

export async function copySelection(editor: Editor, cut: boolean): Promise<void> {
  const { from, to, empty } = editor.state.selection
  if (empty) {
    return
  }
  await navigator.clipboard.writeText(editor.state.doc.textBetween(from, to, '\n'))
  if (cut) {
    editor.chain().focus().deleteSelection().run()
  }
}

export function currentLineSpacing(editor: Editor): number | null {
  const value: unknown = editor.getAttributes('paragraph').lineHeight
  return typeof value === 'number' ? value : null
}

export function currentSpacing(editor: Editor): { before: number; after: number } {
  const attrs = editor.getAttributes('paragraph')
  const number = (value: unknown) => (typeof value === 'number' ? value : 0)
  return { before: number(attrs.spaceBefore), after: number(attrs.spaceAfter) }
}

export function insertDate(editor: Editor, locale: string): void {
  const text = new Intl.DateTimeFormat(locale, { dateStyle: 'long' }).format(new Date())
  editor.chain().focus().insertContent(text).run()
}

export function insertSymbol(editor: Editor, symbol: string): void {
  editor.chain().focus().insertContent(symbol).run()
}
