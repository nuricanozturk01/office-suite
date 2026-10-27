export const MM_TO_PX = 96 / 25.4

export type PageSize = 'a4' | 'a5' | 'a3' | 'letter' | 'legal'
export type Orientation = 'portrait' | 'landscape'

/** Paper sizes in millimetres, portrait. */
export const PAGE_SIZES: Record<PageSize, { width: number; height: number }> = {
  a3: { width: 297, height: 420 },
  a4: { width: 210, height: 297 },
  a5: { width: 148, height: 210 },
  letter: { width: 215.9, height: 279.4 },
  legal: { width: 215.9, height: 355.6 },
}

export interface Margins {
  top: number
  right: number
  bottom: number
  left: number
}

/** Named margin presets of the Layout tab, in millimetres. */
export const MARGIN_PRESETS: Record<'normal' | 'narrow' | 'moderate' | 'wide', Margins> = {
  normal: { top: 25.4, right: 25.4, bottom: 25.4, left: 25.4 },
  narrow: { top: 12.7, right: 12.7, bottom: 12.7, left: 12.7 },
  moderate: { top: 25.4, right: 19.1, bottom: 25.4, left: 19.1 },
  wide: { top: 25.4, right: 50.8, bottom: 25.4, left: 50.8 },
}

export interface HeaderFooter {
  headerLeft: string
  headerRight: string
  footerLeft: string
  footerCenter: string
  footerRight: string
}

/** Shared page setup of a document (stored in the Yjs document, so collaborators agree). */
export interface PageSetup extends HeaderFooter {
  size: PageSize
  orientation: Orientation
  margins: Margins
}

export const DEFAULT_PAGE_SETUP: PageSetup = {
  size: 'a4',
  orientation: 'portrait',
  margins: MARGIN_PRESETS.normal,
  headerLeft: '',
  headerRight: '',
  footerLeft: '',
  footerCenter: '{page}',
  footerRight: '',
}

/** Page geometry in CSS pixels. */
export interface PageGeometry {
  width: number
  height: number
  marginTop: number
  marginRight: number
  marginBottom: number
  marginLeft: number
  /** Gap between two pages on screen. */
  gap: number
}

export const PAGE_GAP_PX = 24

export function pageGeometry(setup: PageSetup): PageGeometry {
  const { width, height } = PAGE_SIZES[setup.size]
  const landscape = setup.orientation === 'landscape'
  return {
    width: (landscape ? height : width) * MM_TO_PX,
    height: (landscape ? width : height) * MM_TO_PX,
    marginTop: setup.margins.top * MM_TO_PX,
    marginRight: setup.margins.right * MM_TO_PX,
    marginBottom: setup.margins.bottom * MM_TO_PX,
    marginLeft: setup.margins.left * MM_TO_PX,
    gap: PAGE_GAP_PX,
  }
}

/** Fills `{page}` and `{pages}` in a header or footer template. */
export function fillTemplate(template: string, page: number, pages: number): string {
  return template.replaceAll('{page}', String(page)).replaceAll('{pages}', String(pages))
}
