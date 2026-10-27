import type { Editor } from '@tiptap/core'

import {
  BLOCK_FONT_SIZE,
  type BlockStyle,
  DEFAULT_FONT_FAMILY,
  MAX_FONT_SIZE,
  MIN_FONT_SIZE,
  type TextAlignment,
} from './formatting'

const HEADING_LEVELS = { heading1: 1, heading2: 2, heading3: 3, heading4: 4 } as const
const HEADING_STYLES = ['heading1', 'heading2', 'heading3', 'heading4'] as const

export function currentBlockStyle(editor: Editor): BlockStyle {
  return (
    HEADING_STYLES.find((style) => editor.isActive('heading', { level: HEADING_LEVELS[style] })) ??
    'paragraph'
  )
}

export function setBlockStyle(editor: Editor, style: BlockStyle): void {
  const chain = editor.chain().focus()
  if (style === 'paragraph') {
    chain.setParagraph().run()
  } else {
    chain.setHeading({ level: HEADING_LEVELS[style] }).run()
  }
}

function textStyleAttribute(editor: Editor, name: string): string | undefined {
  const value: unknown = editor.getAttributes('textStyle')[name]
  return typeof value === 'string' && value !== '' ? value : undefined
}

export function currentFontFamily(editor: Editor): string {
  return textStyleAttribute(editor, 'fontFamily')?.replaceAll('"', '') ?? DEFAULT_FONT_FAMILY
}

export function setFontFamily(editor: Editor, family: string): void {
  const chain = editor.chain().focus()
  if (family === DEFAULT_FONT_FAMILY) {
    chain.unsetFontFamily().run()
  } else {
    chain.setFontFamily(family.includes(' ') ? `"${family}"` : family).run()
  }
}

/** Font size in points: the explicit one, or the block style's default. */
export function currentFontSize(editor: Editor): number {
  const explicit = Number.parseFloat(textStyleAttribute(editor, 'fontSize') ?? '')
  return Number.isFinite(explicit) ? explicit : BLOCK_FONT_SIZE[currentBlockStyle(editor)]
}

export function setFontSize(editor: Editor, points: number): void {
  const size = Math.min(MAX_FONT_SIZE, Math.max(MIN_FONT_SIZE, Math.round(points)))
  editor
    .chain()
    .focus()
    .setFontSize(`${String(size)}pt`)
    .run()
}

export function currentColor(editor: Editor): string | undefined {
  return textStyleAttribute(editor, 'color')
}

export function setTextColor(editor: Editor, color: string | null): void {
  const chain = editor.chain().focus()
  if (color === null) {
    chain.unsetColor().run()
  } else {
    chain.setColor(color).run()
  }
}

export function currentHighlight(editor: Editor): string | undefined {
  const value: unknown = editor.getAttributes('highlight').color
  return typeof value === 'string' ? value : undefined
}

export function setHighlight(editor: Editor, color: string | null): void {
  const chain = editor.chain().focus()
  if (color === null) {
    chain.unsetHighlight().run()
  } else {
    chain.setHighlight({ color }).run()
  }
}

export function currentAlignment(editor: Editor): TextAlignment {
  for (const alignment of ['center', 'right', 'justify'] as const) {
    if (editor.isActive({ textAlign: alignment })) {
      return alignment
    }
  }
  return 'left'
}

export function setAlignment(editor: Editor, alignment: TextAlignment): void {
  editor.chain().focus().setTextAlign(alignment).run()
}

export function setLineHeight(editor: Editor, lineHeight: string): void {
  const multiple = Number.parseFloat(lineHeight)
  editor
    .chain()
    .focus()
    .setLineSpacing(Number.isFinite(multiple) ? multiple : null)
    .run()
}

const LIST_ITEMS = ['listItem', 'taskItem'] as const

/** Nests the current list item; outside lists this is a no-op, as in Google Docs' lists. */
export function indent(editor: Editor): void {
  const item = LIST_ITEMS.find((name) => editor.can().sinkListItem(name))
  if (item !== undefined) {
    editor.chain().focus().sinkListItem(item).run()
  }
}

export function outdent(editor: Editor): void {
  const item = LIST_ITEMS.find((name) => editor.can().liftListItem(name))
  if (item !== undefined) {
    editor.chain().focus().liftListItem(item).run()
  }
}

export function clearFormatting(editor: Editor): void {
  editor.chain().focus().unsetAllMarks().clearNodes().run()
}

/** Accepts only absolute http(s) URLs, adding `https://` when the scheme is missing. */
export function normalizeUrl(raw: string): string | null {
  const trimmed = raw.trim()
  if (trimmed === '') {
    return null
  }
  const candidate = /^[a-z][a-z\d+.-]*:/i.test(trimmed) ? trimmed : `https://${trimmed}`
  try {
    const url = new URL(candidate)
    return url.protocol === 'http:' || url.protocol === 'https:' ? url.href : null
  } catch {
    return null
  }
}

export function currentLink(editor: Editor): string | undefined {
  const value: unknown = editor.getAttributes('link').href
  return typeof value === 'string' ? value : undefined
}

export function setLink(editor: Editor, href: string | null): void {
  const chain = editor.chain().focus().extendMarkRange('link')
  if (href === null) {
    chain.unsetLink().run()
  } else {
    chain.setLink({ href }).run()
  }
}

export function insertImage(editor: Editor, src: string): void {
  editor.chain().focus().setImage({ src }).run()
}

export function insertTable(editor: Editor, rows: number, cols: number): void {
  editor.chain().focus().insertTable({ rows, cols, withHeaderRow: true }).run()
}

export interface OutlineEntry {
  level: number
  text: string
  pos: number
}

export function documentOutline(editor: Editor): OutlineEntry[] {
  const entries: OutlineEntry[] = []
  editor.state.doc.descendants((node, pos) => {
    if (node.type.name === 'heading') {
      const level: unknown = node.attrs.level
      const text = node.textContent.trim()
      if (typeof level === 'number' && text !== '') {
        entries.push({ level, text, pos })
      }
      return false
    }
    return true
  })
  return entries
}

export function jumpTo(editor: Editor, pos: number): void {
  editor
    .chain()
    .focus()
    .setTextSelection(pos + 1)
    .scrollIntoView()
    .run()
}
