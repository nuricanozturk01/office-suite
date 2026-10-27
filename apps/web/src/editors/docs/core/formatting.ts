/** Formatting vocabulary shared by the toolbar and the menu bar. */

export const FONT_FAMILIES = [
  'Arial',
  'Georgia',
  'Times New Roman',
  'Courier New',
  'Verdana',
  'Trebuchet MS',
  'Tahoma',
] as const

export const DEFAULT_FONT_FAMILY = 'Arial'

export const FONT_SIZES = [8, 9, 10, 11, 12, 14, 18, 24, 30, 36, 48, 60, 72, 96] as const
export const MIN_FONT_SIZE = 1
export const MAX_FONT_SIZE = 400

export type BlockStyle = 'paragraph' | 'heading1' | 'heading2' | 'heading3' | 'heading4'

export const BLOCK_STYLES: readonly BlockStyle[] = [
  'paragraph',
  'heading1',
  'heading2',
  'heading3',
  'heading4',
]

/** Point size each block style renders at when no explicit size is set (see `editor.css`). */
export const BLOCK_FONT_SIZE: Record<BlockStyle, number> = {
  paragraph: 11,
  heading1: 20,
  heading2: 16,
  heading3: 14,
  heading4: 12,
}

export const LINE_HEIGHTS = ['1', '1.15', '1.5', '2'] as const

export const ZOOM_LEVELS = [50, 75, 90, 100, 125, 150, 200] as const

export type TextAlignment = 'left' | 'center' | 'right' | 'justify'

/**
 * Text and highlight swatches. These end up inside documents as user content, so they are
 * literal colors rather than theme tokens.
 */
export const COLOR_PALETTE = [
  ['#000000', '#434343', '#666666', '#999999', '#b7b7b7', '#cccccc', '#d9d9d9', '#ffffff'],
  ['#980000', '#ff0000', '#ff9900', '#ffff00', '#00ff00', '#00ffff', '#4a86e8', '#0000ff'],
  ['#e6b8af', '#f4cccc', '#fce5cd', '#fff2cc', '#d9ead3', '#d0e0e3', '#c9daf8', '#cfe2f3'],
  ['#dd7e6b', '#ea9999', '#f9cb9c', '#ffe599', '#b6d7a8', '#a2c4c9', '#a4c2f4', '#9fc5e8'],
  ['#a61c00', '#cc0000', '#e69138', '#f1c232', '#6aa84f', '#45818e', '#3c78d8', '#3d85c6'],
  ['#5b0f00', '#660000', '#783f04', '#7f6000', '#274e13', '#0c343d', '#1c4587', '#073763'],
] as const
