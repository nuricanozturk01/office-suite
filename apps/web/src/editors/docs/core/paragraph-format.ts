import { Extension } from '@tiptap/core'

export type DocStyle = 'title' | 'subtitle' | 'noSpacing' | 'quote'

const TYPES = ['paragraph', 'heading']
const INDENT_STEP_PT = 36
const MAX_INDENT_STEPS = 12

declare module '@tiptap/core' {
  interface Commands<ReturnType> {
    paragraphFormat: {
      /** Applies a named paragraph style (Title, Subtitle, ...) or clears it with `null`. */
      setDocStyle: (style: DocStyle | null) => ReturnType
      /** Line height as a multiple of the font size; `null` clears it. */
      setLineSpacing: (multiple: number | null) => ReturnType
      /** Space before and after the paragraph in points; `null` leaves a side unchanged. */
      setParagraphSpacing: (spacing: {
        before?: number | null
        after?: number | null
      }) => ReturnType
      /** Moves the paragraph's left edge by `steps` of half an inch. */
      changeParagraphIndent: (steps: number) => ReturnType
    }
  }
}

function numberAttribute(name: string, cssProperty: string, unit: string) {
  return {
    default: null,
    parseHTML: (element: HTMLElement) => {
      const value = Number.parseFloat(element.style.getPropertyValue(cssProperty))
      return Number.isFinite(value) ? value : null
    },
    renderHTML: (attributes: Record<string, unknown>) => {
      const value = attributes[name]
      return typeof value === 'number' ? { style: `${cssProperty}: ${String(value)}${unit}` } : {}
    },
  }
}

/** Word-like paragraph formatting: named styles, line and paragraph spacing, indentation. */
export const ParagraphFormat = Extension.create({
  name: 'paragraphFormat',

  addGlobalAttributes() {
    return [
      {
        types: TYPES,
        attributes: {
          docStyle: {
            default: null,
            parseHTML: (element: HTMLElement) => element.dataset.docStyle ?? null,
            renderHTML: (attributes: Record<string, unknown>) =>
              typeof attributes.docStyle === 'string'
                ? { 'data-doc-style': attributes.docStyle }
                : {},
          },
          lineHeight: numberAttribute('lineHeight', 'line-height', ''),
          spaceBefore: numberAttribute('spaceBefore', 'margin-top', 'pt'),
          spaceAfter: numberAttribute('spaceAfter', 'margin-bottom', 'pt'),
          indent: numberAttribute('indent', 'margin-left', 'pt'),
        },
      },
    ]
  },

  addCommands() {
    const update =
      (attributes: Record<string, unknown>) =>
      (props: {
        commands: { updateAttributes: (type: string, attrs: Record<string, unknown>) => boolean }
      }) =>
        TYPES.map((type) => props.commands.updateAttributes(type, attributes)).some(Boolean)
    return {
      setDocStyle: (style) => update({ docStyle: style }),
      setLineSpacing: (multiple) => update({ lineHeight: multiple }),
      setParagraphSpacing: (spacing) =>
        update({
          ...(spacing.before === undefined ? {} : { spaceBefore: spacing.before }),
          ...(spacing.after === undefined ? {} : { spaceAfter: spacing.after }),
        }),
      changeParagraphIndent:
        (steps) =>
        ({ editor, commands }) => {
          const current = Number(editor.getAttributes('paragraph').indent ?? 0)
          const next = Math.min(
            MAX_INDENT_STEPS * INDENT_STEP_PT,
            Math.max(0, current + steps * INDENT_STEP_PT),
          )
          return TYPES.map((type) =>
            commands.updateAttributes(type, { indent: next === 0 ? null : next }),
          ).some(Boolean)
        },
    }
  },
})
