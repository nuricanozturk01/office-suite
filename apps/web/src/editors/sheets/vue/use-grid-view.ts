import { computed, onBeforeUnmount, onMounted, ref, watch, type Ref } from 'vue'

import { colLabel } from '../core/address'
import { cellKey, contains, displayAlign, ROW_COUNT, ROW_HEIGHT } from '../core/workbook'
import type { SheetModel } from './use-sheet-model'
import { columnAt, columnLayout, visibleColumns, visibleRows } from './viewport'

export interface PaintedCell {
  key: string
  x: number
  y: number
  w: number
  text: string
  bold: boolean
  italic: boolean
  underline: boolean
  align: 'left' | 'center' | 'right'
  color: string | undefined
  fill: string | undefined
  selected: boolean
}

export interface HeaderItem {
  index: number
  label: string
  x: number
  y: number
  size: number
  selected: boolean
}

/** Virtual window of the sheet: which cells and headers are on screen, and where they sit. */
export function useGridView(model: SheetModel, scroller: Ref<HTMLElement | null>) {
  const scrollTop = ref(0)
  const scrollLeft = ref(0)
  const viewWidth = ref(800)
  const viewHeight = ref(600)

  const layout = computed(() => columnLayout((col) => model.colWidth(col)))
  const rowWindow = computed(() => visibleRows(scrollTop.value, viewHeight.value))
  const colWindow = computed(() => visibleColumns(layout.value, scrollLeft.value, viewWidth.value))

  const cells = computed(() => {
    const painted: PaintedCell[] = []
    const area = model.rect.value
    const focus = model.focus.value
    const sheet = model.sheet.value
    for (let row = rowWindow.value.start; row <= rowWindow.value.end; row += 1) {
      for (let col = colWindow.value.start; col <= colWindow.value.end; col += 1) {
        const data = sheet?.cells[cellKey(row, col)]
        const active = row === focus.row && col === focus.col
        painted.push({
          key: cellKey(row, col),
          x: layout.value.xs[col] ?? 0,
          y: row * ROW_HEIGHT,
          w: layout.value.widths[col] ?? 100,
          text: model.shown(row, col),
          bold: data?.bold === true,
          italic: data?.italic === true,
          underline: data?.underline === true,
          align: displayAlign(data, model.valueAt(row, col)),
          color: data?.color,
          fill: data?.fill,
          selected: !active && contains(area, row, col),
        })
      }
    }
    return painted
  })

  const columns = computed<HeaderItem[]>(() => {
    const area = model.rect.value
    const items: HeaderItem[] = []
    for (let col = colWindow.value.start; col <= colWindow.value.end; col += 1) {
      items.push({
        index: col,
        label: colLabel(col),
        x: layout.value.xs[col] ?? 0,
        y: 0,
        size: layout.value.widths[col] ?? 100,
        selected: col >= area.c0 && col <= area.c1,
      })
    }
    return items
  })

  const rows = computed<HeaderItem[]>(() => {
    const area = model.rect.value
    const items: HeaderItem[] = []
    for (let row = rowWindow.value.start; row <= rowWindow.value.end; row += 1) {
      items.push({
        index: row,
        label: String(row + 1),
        x: 0,
        y: row * ROW_HEIGHT,
        size: ROW_HEIGHT,
        selected: row >= area.r0 && row <= area.r1,
      })
    }
    return items
  })

  const selection = computed(() => {
    const area = model.rect.value
    const left = layout.value.xs[area.c0] ?? 0
    const right = (layout.value.xs[area.c1] ?? 0) + (layout.value.widths[area.c1] ?? 0)
    return {
      left,
      top: area.r0 * ROW_HEIGHT,
      width: right - left,
      height: (area.r1 - area.r0 + 1) * ROW_HEIGHT,
    }
  })

  const editorBox = computed(() => {
    const current = model.editing.value
    if (current?.source !== 'cell') {
      return null
    }
    const left = layout.value.xs[current.col] ?? 0
    const width = Math.max(layout.value.widths[current.col] ?? 100, current.draft.length * 8 + 16)
    return {
      left: `${String(left)}px`,
      top: `${String(current.row * ROW_HEIGHT)}px`,
      width: `${String(width)}px`,
      height: `${String(ROW_HEIGHT)}px`,
    }
  })

  function onScroll(event: Event): void {
    if (!(event.target instanceof HTMLElement)) {
      return
    }
    scrollTop.value = event.target.scrollTop
    scrollLeft.value = event.target.scrollLeft
  }

  function cellFrom(event: MouseEvent): { row: number; col: number } | null {
    const el = scroller.value
    if (el === null) {
      return null
    }
    const box = el.getBoundingClientRect()
    const x = event.clientX - box.left + el.scrollLeft
    const y = event.clientY - box.top + el.scrollTop
    return {
      row: Math.min(ROW_COUNT - 1, Math.max(0, Math.floor(y / ROW_HEIGHT))),
      col: columnAt(layout.value, Math.max(0, x)),
    }
  }

  onMounted(() => {
    const el = scroller.value
    if (el === null) {
      return
    }
    const observer = new ResizeObserver(() => {
      viewWidth.value = el.clientWidth
      viewHeight.value = el.clientHeight
    })
    observer.observe(el)
    viewWidth.value = el.clientWidth
    viewHeight.value = el.clientHeight
    onBeforeUnmount(() => {
      observer.disconnect()
    })
  })

  watch(
    () => [model.focus.value.row, model.focus.value.col] as const,
    ([row, col]) => {
      const el = scroller.value
      if (el === null) {
        return
      }
      const top = row * ROW_HEIGHT
      const left = layout.value.xs[col] ?? 0
      const width = layout.value.widths[col] ?? 100
      if (top < el.scrollTop) {
        el.scrollTop = top
      } else if (top + ROW_HEIGHT > el.scrollTop + el.clientHeight) {
        el.scrollTop = top + ROW_HEIGHT - el.clientHeight
      }
      if (left < el.scrollLeft) {
        el.scrollLeft = left
      } else if (left + width > el.scrollLeft + el.clientWidth) {
        el.scrollLeft = left + width - el.clientWidth
      }
    },
  )

  return {
    layout,
    cells,
    columns,
    rows,
    selection,
    editorBox,
    scrollTop,
    scrollLeft,
    onScroll,
    cellFrom,
  }
}
