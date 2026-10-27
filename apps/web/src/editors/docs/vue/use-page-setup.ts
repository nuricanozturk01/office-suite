import { computed, onScopeDispose, shallowRef } from 'vue'
import type * as Y from 'yjs'

import {
  DEFAULT_PAGE_SETUP,
  type HeaderFooter,
  type Margins,
  type Orientation,
  type PageSetup,
  type PageSize,
  pageGeometry,
  PAGE_SIZES,
} from '../core/page-layout'

const MAP = 'page'

function readSetup(map: Y.Map<unknown>): PageSetup {
  const size = map.get('size')
  const orientation = map.get('orientation')
  const margins = map.get('margins')
  const text = (key: keyof HeaderFooter): string => {
    const value = map.get(key)
    return typeof value === 'string' ? value : DEFAULT_PAGE_SETUP[key]
  }
  return {
    size:
      typeof size === 'string' && size in PAGE_SIZES ? (size as PageSize) : DEFAULT_PAGE_SETUP.size, // key membership checked above
    orientation: orientation === 'landscape' ? 'landscape' : 'portrait',
    margins: isMargins(margins) ? margins : DEFAULT_PAGE_SETUP.margins,
    headerLeft: text('headerLeft'),
    headerRight: text('headerRight'),
    footerLeft: text('footerLeft'),
    footerCenter: text('footerCenter'),
    footerRight: text('footerRight'),
  }
}

function isMargins(value: unknown): value is Margins {
  if (typeof value !== 'object' || value === null) {
    return false
  }
  return ['top', 'right', 'bottom', 'left'].every(
    (key) => typeof (value as Record<string, unknown>)[key] === 'number', // keys listed above
  )
}

/** Page size, orientation, margins and header/footer text, shared through the Yjs document. */
export function usePageSetup(doc: Y.Doc) {
  const map = doc.getMap<unknown>(MAP)
  const setup = shallowRef<PageSetup>(readSetup(map))
  const onChange = (): void => {
    setup.value = readSetup(map)
  }
  map.observe(onChange)
  onScopeDispose(() => {
    map.unobserve(onChange)
  })

  function update(patch: Partial<PageSetup>): void {
    doc.transact(() => {
      for (const [key, value] of Object.entries(patch)) {
        map.set(key, value)
      }
    })
  }

  return {
    setup,
    geometry: computed(() => pageGeometry(setup.value)),
    setSize: (size: PageSize) => {
      update({ size })
    },
    setOrientation: (orientation: Orientation) => {
      update({ orientation })
    },
    setMargins: (margins: Margins) => {
      update({ margins })
    },
    setTexts: (texts: Partial<HeaderFooter>) => {
      update(texts)
    },
  }
}

export type PageSetupModel = ReturnType<typeof usePageSetup>
