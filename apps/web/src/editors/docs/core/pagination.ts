import { Extension } from '@tiptap/core'
import type { Node as ProseNode } from '@tiptap/pm/model'
import { Plugin, PluginKey } from '@tiptap/pm/state'
import { Decoration, DecorationSet, type EditorView } from '@tiptap/pm/view'

import { fillTemplate, type HeaderFooter, type PageGeometry } from './page-layout'
import { PAGE_BREAK } from './page-break'

export interface PaginationOptions {
  geometry: () => PageGeometry
  texts: () => HeaderFooter
  /** Called after each layout pass with the number of pages. */
  onPages: (count: number) => void
}

interface Spacer {
  /** Document position of the block the spacer is placed before. */
  pos: number
  height: number
  /** Number of the page the spacer ends. */
  page: number
}

const paginationKey = new PluginKey<DecorationSet>('docs-pagination')
const SPACER_CLASS = 'page-spacer'

function sameSpacers(a: readonly Spacer[], b: readonly Spacer[]): boolean {
  return (
    a.length === b.length &&
    a.every((spacer, i) => {
      const other = b[i]
      return other?.pos === spacer.pos && Math.abs(other.height - spacer.height) < 0.5
    })
  )
}

function cell(text: string, align: 'left' | 'center' | 'right'): HTMLElement {
  const element = document.createElement('span')
  element.className = 'page-text'
  element.style.textAlign = align
  element.textContent = text
  return element
}

function band(
  kind: 'footer' | 'header',
  height: number,
  geometry: PageGeometry,
  texts: [string, string, string],
): HTMLElement {
  const element = document.createElement('div')
  element.className = `page-${kind}`
  element.style.height = `${String(height)}px`
  element.style.paddingInline = `${String(geometry.marginLeft)}px ${String(geometry.marginRight)}px`
  element.append(cell(texts[0], 'left'), cell(texts[1], 'center'), cell(texts[2], 'right'))
  return element
}

function buildSpacer(spacer: Spacer, options: PaginationOptions, pages: number): HTMLElement {
  const geometry = options.geometry()
  const texts = options.texts()
  const element = document.createElement('div')
  element.className = SPACER_CLASS
  element.contentEditable = 'false'
  element.style.height = `${String(spacer.height)}px`
  element.style.marginInline = `-${String(geometry.marginLeft)}px -${String(geometry.marginRight)}px`
  const fill = (template: string, page: number) => fillTemplate(template, page, pages)
  const footer = band('footer', geometry.marginBottom, geometry, [
    fill(texts.footerLeft, spacer.page),
    fill(texts.footerCenter, spacer.page),
    fill(texts.footerRight, spacer.page),
  ])
  const gap = document.createElement('div')
  gap.className = 'page-gap'
  gap.style.height = `${String(geometry.gap)}px`
  const header = band('header', geometry.marginTop, geometry, [
    fill(texts.headerLeft, spacer.page + 1),
    '',
    fill(texts.headerRight, spacer.page + 1),
  ])
  const filler = document.createElement('div')
  filler.style.height = `${String(Math.max(0, spacer.height - geometry.marginBottom - geometry.gap - geometry.marginTop))}px`
  element.append(filler, footer, gap, header)
  return element
}

/** Vertical extent of a block including its margins, in layout pixels (unaffected by CSS zoom). */
function blockExtent(
  element: HTMLElement,
  container: HTMLElement,
): { top: number; bottom: number } {
  const style = getComputedStyle(element)
  const top = element.offsetTop - container.offsetTop
  return {
    top: top - Number.parseFloat(style.marginTop || '0'),
    bottom: top + element.offsetHeight + Number.parseFloat(style.marginBottom || '0'),
  }
}

/** The page (1-based) the document position `pos` is on, from the laid out DOM. */
export function pageAt(view: EditorView, pos: number, geometry: PageGeometry): number {
  const { node } = view.domAtPos(pos)
  let element: Node | null = node
  while (element !== null && element.parentNode !== view.dom) {
    element = element.parentNode
  }
  if (!(element instanceof HTMLElement)) {
    return 1
  }
  const top = element.offsetTop - view.dom.offsetTop
  return Math.max(1, Math.floor(top / (geometry.height + geometry.gap)) + 1)
}

/**
 * Lays the document out on pages: before each top-level block that would cross the bottom
 * margin it inserts a spacer widget holding the footer of the finished page, the gap and the
 * header of the next page. Page breaks push the next block to a new page.
 */
export const Pagination = Extension.create<PaginationOptions>({
  name: 'pagination',

  addProseMirrorPlugins() {
    const options = this.options
    let frame = 0
    let spacers: Spacer[] = []

    const measure = (view: EditorView): void => {
      const geometry = options.geometry()
      const dom = view.dom
      const blocks = [...dom.children].filter(
        (child): child is HTMLElement =>
          child instanceof HTMLElement &&
          !child.classList.contains(SPACER_CLASS) &&
          !child.classList.contains('ProseMirror-gapcursor') &&
          !child.classList.contains('ProseMirror-trailingBreak'),
      )
      const nodes: { node: ProseNode; pos: number }[] = []
      view.state.doc.forEach((node, offset) => nodes.push({ node, pos: offset }))
      if (blocks.length !== nodes.length) {
        return
      }
      const contentHeight = geometry.height - geometry.marginTop - geometry.marginBottom
      const period = geometry.height + geometry.gap
      const spacerHeight = (remaining: number) =>
        remaining + geometry.marginBottom + geometry.gap + geometry.marginTop
      const origin = geometry.marginTop
      const rendered = [...dom.querySelectorAll<HTMLElement>(`.${SPACER_CLASS}`)]
      const next: Spacer[] = []
      let shift = 0
      let page = 0
      let forceBreak = false

      blocks.forEach((element, index) => {
        const entry = nodes[index]
        if (entry === undefined) {
          return
        }
        const renderedBefore = rendered
          .filter(
            (spacer) => spacer.compareDocumentPosition(element) & Node.DOCUMENT_POSITION_FOLLOWING,
          )
          .reduce((sum, spacer) => sum + spacer.offsetHeight, 0)
        const extent = blockExtent(element, dom)
        const top = extent.top - origin - renderedBefore + shift
        const bottom = extent.bottom - origin - renderedBefore + shift
        const pageEnd = page * period + contentHeight
        const pageStart = page * period
        const overflows = bottom > pageEnd + 0.5 && top > pageStart + 1
        if (forceBreak || overflows) {
          const remaining = Math.max(0, pageEnd - Math.max(top, pageStart))
          const height = spacerHeight(remaining)
          next.push({ pos: entry.pos, height, page: page + 1 })
          shift += height
          page += 1
          forceBreak = false
        }
        if (entry.node.type.name === PAGE_BREAK) {
          forceBreak = true
        }
      })

      const pages = next.length + 1
      options.onPages(pages)
      dom.style.minHeight = `${String(pages * geometry.height + (pages - 1) * geometry.gap)}px`
      if (!sameSpacers(spacers, next)) {
        spacers = next
        view.dispatch(view.state.tr.setMeta(paginationKey, { spacers: next, pages }))
      }
    }

    const schedule = (view: EditorView): void => {
      cancelAnimationFrame(frame)
      frame = requestAnimationFrame(() => {
        measure(view)
      })
    }

    return [
      new Plugin<DecorationSet>({
        key: paginationKey,
        state: {
          init: () => DecorationSet.empty,
          apply: (tr, set) => {
            const meta: unknown = tr.getMeta(paginationKey)
            if (typeof meta === 'object' && meta !== null && 'spacers' in meta) {
              const { spacers: list, pages } = meta as { spacers: Spacer[]; pages: number } // set by this plugin
              return DecorationSet.create(
                tr.doc,
                list.map((spacer) =>
                  Decoration.widget(spacer.pos, () => buildSpacer(spacer, options, pages), {
                    side: -1,
                    key: `spacer-${String(spacer.pos)}-${String(Math.round(spacer.height))}-${String(pages)}-${JSON.stringify(options.texts())}`,
                    ignoreSelection: true,
                  }),
                ),
              )
            }
            return tr.docChanged ? set.map(tr.mapping, tr.doc) : set
          },
        },
        props: {
          decorations: (state) => paginationKey.getState(state) ?? DecorationSet.empty,
        },
        view: (view) => {
          schedule(view)
          const observer = new ResizeObserver(() => {
            schedule(view)
          })
          observer.observe(view.dom)
          return {
            update: () => {
              schedule(view)
            },
            destroy: () => {
              cancelAnimationFrame(frame)
              observer.disconnect()
            },
          }
        },
      }),
    ]
  },
})
