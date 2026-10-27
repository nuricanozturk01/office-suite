import { COL_COUNT, ROW_COUNT, ROW_HEIGHT } from '../core/workbook'

export const ROW_HEADER_WIDTH = 46
export const COL_HEADER_HEIGHT = 24

export interface ColumnLayout {
  xs: number[]
  widths: number[]
  total: number
}

export function columnLayout(widthAt: (col: number) => number): ColumnLayout {
  const xs: number[] = []
  const widths: number[] = []
  let x = 0
  for (let col = 0; col < COL_COUNT; col += 1) {
    xs.push(x)
    const width = widthAt(col)
    widths.push(width)
    x += width
  }
  return { xs, widths, total: x }
}

/** Column under a horizontal offset, clamped to the grid. */
export function columnAt(layout: ColumnLayout, x: number): number {
  let lo = 0
  let hi = layout.xs.length - 1
  while (lo <= hi) {
    const mid = (lo + hi) >> 1
    const start = layout.xs[mid]
    const width = layout.widths[mid]
    if (start === undefined || width === undefined) {
      return 0
    }
    if (x < start) {
      hi = mid - 1
    } else if (x >= start + width) {
      lo = mid + 1
    } else {
      return mid
    }
  }
  return Math.min(layout.xs.length - 1, Math.max(0, lo))
}

export function visibleColumns(
  layout: ColumnLayout,
  scrollLeft: number,
  viewWidth: number,
): { start: number; end: number } {
  return {
    start: Math.max(0, columnAt(layout, scrollLeft) - 1),
    end: Math.min(COL_COUNT - 1, columnAt(layout, scrollLeft + viewWidth) + 1),
  }
}

export function visibleRows(scrollTop: number, viewHeight: number): { start: number; end: number } {
  return {
    start: Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - 1),
    end: Math.min(ROW_COUNT - 1, Math.ceil((scrollTop + viewHeight) / ROW_HEIGHT) + 1),
  }
}
