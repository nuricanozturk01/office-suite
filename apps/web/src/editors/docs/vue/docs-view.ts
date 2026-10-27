import type { Mark } from '@tiptap/pm/model'
import { type InjectionKey, inject, provide, ref, shallowRef } from 'vue'

export type DocsDialog = 'image' | 'table' | 'wordCount' | 'headerFooter' | 'symbols' | 'margins'
export type RibbonTab = 'home' | 'insert' | 'layout' | 'review' | 'view'

function createDocsView() {
  return {
    zoom: ref(100),
    outlineOpen: ref(true),
    linkOpen: ref(false),
    dialog: ref<DocsDialog | null>(null),
    ribbonTab: ref<RibbonTab>('home'),
    rulerOpen: ref(true),
    showMarks: ref(false),
    findOpen: ref(false),
    /** Marks copied by the format painter, applied to the next selection. */
    painter: shallowRef<readonly Mark[] | null>(null),
  }
}

/** UI state of the editor chrome (not part of the shared document). */
export type DocsView = ReturnType<typeof createDocsView>

const DOCS_VIEW: InjectionKey<DocsView> = Symbol('docs-view')

export function provideDocsView(): DocsView {
  const view = createDocsView()
  provide(DOCS_VIEW, view)
  return view
}

export function useDocsView(): DocsView {
  const view = inject(DOCS_VIEW)
  if (view === undefined) {
    throw new Error('useDocsView() must be used inside DocsEditor')
  }
  return view
}

const isMac = typeof navigator !== 'undefined' && navigator.userAgent.includes('Mac')

/** Shortcut label using the platform's modifier key, e.g. `mod('K')` → `Ctrl+K` / `⌘K`. */
export function mod(...keys: string[]): string {
  return isMac ? `⌘${keys.join('')}` : ['Ctrl', ...keys].join('+')
}
