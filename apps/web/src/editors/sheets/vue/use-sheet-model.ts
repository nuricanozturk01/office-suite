import { useQueryClient } from '@tanstack/vue-query'
import { computed, type InjectionKey, inject, onBeforeUnmount, provide, ref, toValue } from 'vue'
import { useI18n } from 'vue-i18n'

import { spreadsheetKeys, type SpreadsheetDetail } from '@/entities/spreadsheet'
import { api, ApiError, unwrap } from '@/shared/api'

import { formatAddress, parseAddress, shiftFormula } from '../core/address'
import { createWorkbookCalc } from '../core/calc'
import type { CellValue } from '@office/formula-engine'
import { formatValue } from '../core/format'
import {
  cellKey,
  COL_COUNT,
  colWidth,
  contains,
  MAX_COL_WIDTH,
  MIN_COL_WIDTH,
  parseWorkbook,
  rectOf,
  DEFAULT_COL_WIDTH,
  ROW_COUNT,
  type Align,
  type CellData,
  type CellPos,
  type NumberFormat,
  type Rect,
  type SheetData,
  workbookContent,
  type WorkbookData,
} from '../core/workbook'

export type SaveStatus = 'saved' | 'saving' | 'error' | 'conflict'
type Flag = 'bold' | 'italic' | 'underline'

export interface Editing {
  row: number
  col: number
  draft: string
  /** `bar` keeps focus in the formula bar; `cell` focuses the in-grid editor. */
  source: 'cell' | 'bar'
}

function cloneBook(workbook: WorkbookData): WorkbookData {
  return parseWorkbook(workbookContent(workbook)) ?? { sheets: [] }
}

function mod(value: number, size: number): number {
  return ((value % size) + size) % size
}

function styled(from: CellData | undefined, input: string): CellData {
  const next: CellData = { input }
  if (from?.bold === true) {
    next.bold = true
  }
  if (from?.italic === true) {
    next.italic = true
  }
  if (from?.underline === true) {
    next.underline = true
  }
  if (from?.align !== undefined) {
    next.align = from.align
  }
  if (from?.color !== undefined) {
    next.color = from.color
  }
  if (from?.fill !== undefined) {
    next.fill = from.fill
  }
  if (from?.format !== undefined) {
    next.format = from.format
  }
  return next
}

function withoutKey(cells: Record<string, CellData>, key: string): Record<string, CellData> {
  const next: Record<string, CellData> = {}
  for (const [name, cell] of Object.entries(cells)) {
    if (name !== key) {
      next[name] = cell
    }
  }
  return next
}

function outside(cells: Record<string, CellData>, area: Rect): Record<string, CellData> {
  const next: Record<string, CellData> = {}
  for (const [name, cell] of Object.entries(cells)) {
    const [rowText, colText] = name.split(',')
    if (!contains(area, Number(rowText), Number(colText))) {
      next[name] = cell
    }
  }
  return next
}

function isBlank(cell: CellData): boolean {
  return (
    cell.input === '' &&
    cell.bold !== true &&
    cell.italic !== true &&
    cell.underline !== true &&
    cell.align === undefined &&
    cell.color === undefined &&
    cell.fill === undefined &&
    cell.format === undefined
  )
}

/**
 * Workbook state for one spreadsheet: selection, editing, undo and debounced saves.
 * The parent remounts this after a reload, so `initial` is whatever the server last stored.
 */
export function useSheetModel(id: string, initial: WorkbookData, initialVersion: number) {
  const { t, locale } = useI18n()
  const queryClient = useQueryClient()
  const workbook = ref<WorkbookData>(cloneBook(initial))
  const active = ref(0)
  const anchor = ref<CellPos>({ row: 0, col: 0 })
  const focus = ref<CellPos>({ row: 0, col: 0 })
  const editing = ref<Editing | null>(null)
  const version = ref(initialVersion)
  const status = ref<SaveStatus>('saved')
  const past = ref<WorkbookData[]>([])
  const future = ref<WorkbookData[]>([])
  const dirty = ref(false)
  let timer = 0
  let flushing: Promise<unknown> | null = null
  let resizeArmed = false

  const sheet = computed(() => workbook.value.sheets[active.value] ?? workbook.value.sheets[0])
  const rect = computed(() => rectOf(anchor.value, focus.value))
  const calc = createWorkbookCalc(workbook.value)
  onBeforeUnmount(() => {
    calc.dispose()
  })
  // Reading this brings the engine up to date and makes the grid re-render after every edit.
  const calculated = computed(() => {
    calc.sync(workbook.value)
    return calc
  })
  const focusKey = computed(() => cellKey(focus.value.row, focus.value.col))
  const focusCell = computed(() => sheet.value?.cells[focusKey.value])

  function scheduleSave(): void {
    dirty.value = true
    if (status.value !== 'conflict') {
      status.value = 'saving'
    }
    window.clearTimeout(timer)
    timer = window.setTimeout(() => {
      void flush()
    }, 500)
  }

  async function flush(): Promise<void> {
    if (flushing) {
      await flushing
      if (dirty.value) {
        await flush()
      }
      return
    }
    if (!dirty.value || status.value === 'conflict') {
      return
    }
    dirty.value = false
    const content = workbookContent(workbook.value)
    const run = send(content).finally(() => {
      flushing = null
    })
    flushing = run
    if (await run) {
      await flush()
    }
  }

  /** Saves the snapshot taken by the caller. Returns true when another save is already pending. */
  async function send(content: Record<string, unknown>): Promise<boolean> {
    try {
      const saved = await unwrap(
        api.PUT('/api/v1/spreadsheets/{id}/content', {
          params: { path: { id } },
          body: { version: version.value, content },
        }),
      )
      version.value = saved.version
      queryClient.setQueryData<SpreadsheetDetail>(spreadsheetKeys.detail(id), (current) =>
        current === undefined ? current : { ...current, version: saved.version, content },
      )
      status.value = dirty.value ? 'saving' : 'saved'
      return dirty.value
    } catch (error) {
      if (error instanceof ApiError && error.code === 'version_conflict') {
        status.value = 'conflict'
        return false
      }
      dirty.value = true
      status.value = 'error'
      return false
    }
  }

  onBeforeUnmount(() => {
    window.clearTimeout(timer)
    if (dirty.value) {
      void flush()
    }
  })

  function snapshot(): void {
    past.value.push(cloneBook(workbook.value))
    if (past.value.length > 100) {
      past.value.shift()
    }
    future.value = []
  }

  function replaceSheet(next: SheetData, record: boolean): void {
    if (record) {
      snapshot()
    }
    workbook.value = {
      sheets: workbook.value.sheets.map((item, index) => (index === active.value ? next : item)),
    }
    scheduleSave()
  }

  function mapCells(
    update: (cells: Record<string, CellData>) => Record<string, CellData>,
    record = true,
  ): void {
    const current = sheet.value
    if (current === undefined) {
      return
    }
    replaceSheet({ ...current, cells: update(current.cells) }, record)
  }

  function valueAt(row: number, col: number): CellValue {
    return calculated.value.valueAt(active.value, row, col)
  }

  function shown(row: number, col: number): string {
    const current = editing.value
    if (current?.row === row && current.col === col) {
      return current.draft
    }
    return formatValue(
      valueAt(row, col),
      sheet.value?.cells[cellKey(row, col)]?.format,
      toValue(locale),
      {
        on: t('sheets.boolTrue'),
        off: t('sheets.boolFalse'),
      },
    )
  }

  function clamp(pos: CellPos): CellPos {
    return {
      row: Math.min(ROW_COUNT - 1, Math.max(0, pos.row)),
      col: Math.min(COL_COUNT - 1, Math.max(0, pos.col)),
    }
  }

  function select(next: CellPos, extend: boolean): void {
    const pos = clamp(next)
    if (!extend) {
      anchor.value = pos
    }
    focus.value = pos
    editing.value = null
  }

  function move(dRow: number, dCol: number, extend: boolean): void {
    select({ row: focus.value.row + dRow, col: focus.value.col + dCol }, extend)
  }

  function startEdit(source: Editing['source'], seed?: string): void {
    editing.value = {
      row: focus.value.row,
      col: focus.value.col,
      draft: seed ?? focusCell.value?.input ?? '',
      source,
    }
  }

  function commit(next: CellPos | null): void {
    const current = editing.value
    if (current === null) {
      if (next !== null) {
        select(next, false)
      }
      return
    }
    const key = cellKey(current.row, current.col)
    const previous = sheet.value?.cells[key]
    if (current.draft !== (previous?.input ?? '')) {
      mapCells((cells) =>
        current.draft === ''
          ? withoutKey(cells, key)
          : { ...cells, [key]: styled(previous, current.draft) },
      )
    }
    editing.value = null
    if (next !== null) {
      select(next, false)
    }
  }

  function setDraft(draft: string): void {
    if (editing.value !== null) {
      editing.value = { ...editing.value, draft }
    }
  }

  function mapSelection(produce: (cell: CellData | undefined) => CellData | null): void {
    const area = rect.value
    mapCells((cells) => {
      const next = outside(cells, area)
      for (let row = area.r0; row <= area.r1; row += 1) {
        for (let col = area.c0; col <= area.c1; col += 1) {
          const produced = produce(cells[cellKey(row, col)])
          if (produced !== null) {
            next[cellKey(row, col)] = produced
          }
        }
      }
      return next
    })
  }

  function toggleFlag(flag: Flag): void {
    const area = rect.value
    const current = sheet.value
    if (current === undefined) {
      return
    }
    let allOn = true
    for (let row = area.r0; row <= area.r1 && allOn; row += 1) {
      for (let col = area.c0; col <= area.c1; col += 1) {
        if (current.cells[cellKey(row, col)]?.[flag] !== true) {
          allOn = false
        }
      }
    }
    mapSelection((cell) => {
      const next = styled(cell, cell?.input ?? '')
      if (allOn && flag === 'bold') {
        delete next.bold
      } else if (allOn && flag === 'italic') {
        delete next.italic
      } else if (allOn) {
        delete next.underline
      } else if (flag === 'bold') {
        next.bold = true
      } else if (flag === 'italic') {
        next.italic = true
      } else {
        next.underline = true
      }
      return isBlank(next) ? null : next
    })
  }

  function setAlign(align: Align): void {
    mapSelection((cell) => {
      const next = styled(cell, cell?.input ?? '')
      next.align = align
      return next
    })
  }

  function setFormat(format: NumberFormat | null): void {
    mapSelection((cell) => {
      const next = styled(cell, cell?.input ?? '')
      if (format === null) {
        delete next.format
      } else {
        next.format = format
      }
      return isBlank(next) ? null : next
    })
  }

  function setPaint(kind: 'color' | 'fill', color: string | null): void {
    mapSelection((cell) => {
      const next = styled(cell, cell?.input ?? '')
      if (kind === 'color') {
        if (color === null) {
          delete next.color
        } else {
          next.color = color
        }
      } else if (color === null) {
        delete next.fill
      } else {
        next.fill = color
      }
      return isBlank(next) ? null : next
    })
  }

  function clearSelection(): void {
    const area = rect.value
    mapCells((cells) => outside(cells, area))
  }

  function undo(): void {
    const previous = past.value.pop()
    if (previous === undefined) {
      return
    }
    future.value.push(cloneBook(workbook.value))
    workbook.value = previous
    editing.value = null
    scheduleSave()
  }

  function redo(): void {
    const next = future.value.pop()
    if (next === undefined) {
      return
    }
    past.value.push(cloneBook(workbook.value))
    workbook.value = next
    editing.value = null
    scheduleSave()
  }

  function addSheet(name: string): void {
    if (workbook.value.sheets.length >= 20) {
      return
    }
    snapshot()
    const sheets = [...workbook.value.sheets, { id: crypto.randomUUID(), name, cells: {} }]
    workbook.value = { sheets }
    active.value = sheets.length - 1
    select({ row: 0, col: 0 }, false)
    scheduleSave()
  }

  function renameSheet(index: number, name: string): void {
    const trimmed = name.trim()
    const current = workbook.value.sheets[index]
    if (current === undefined || trimmed === '' || trimmed === current.name) {
      return
    }
    snapshot()
    workbook.value = {
      sheets: workbook.value.sheets.map((item, i) =>
        i === index ? { ...item, name: trimmed } : item,
      ),
    }
    scheduleSave()
  }

  function deleteSheet(index: number): void {
    if (workbook.value.sheets.length < 2) {
      return
    }
    snapshot()
    const sheets = workbook.value.sheets.filter((_, i) => i !== index)
    workbook.value = { sheets }
    active.value = Math.min(active.value, sheets.length - 1)
    scheduleSave()
  }

  function armColumnResize(): void {
    resizeArmed = true
  }

  function setColWidth(col: number, width: number): void {
    const current = sheet.value
    if (current === undefined) {
      return
    }
    const clamped = Math.min(MAX_COL_WIDTH, Math.max(MIN_COL_WIDTH, Math.round(width)))
    const previous = current.cols?.[String(col)] ?? DEFAULT_COL_WIDTH
    if (clamped === previous) {
      return
    }
    const record = resizeArmed
    resizeArmed = false
    replaceSheet({ ...current, cols: { ...current.cols, [String(col)]: clamped } }, record)
  }

  function fillTo(target: CellPos): void {
    const source = rect.value
    const dest = rectOf({ row: source.r0, col: source.c0 }, clamp(target))
    const height = source.r1 - source.r0 + 1
    const width = source.c1 - source.c0 + 1
    const original = sheet.value?.cells ?? {}
    mapCells((cells) => {
      const dropped = new Set<string>()
      const written: Record<string, CellData> = {}
      for (let row = dest.r0; row <= dest.r1; row += 1) {
        for (let col = dest.c0; col <= dest.c1; col += 1) {
          if (contains(source, row, col)) {
            continue
          }
          const srcRow = source.r0 + mod(row - source.r0, height)
          const srcCol = source.c0 + mod(col - source.c0, width)
          const src = original[cellKey(srcRow, srcCol)]
          const key = cellKey(row, col)
          if (src === undefined) {
            dropped.add(key)
          } else {
            written[key] = styled(src, shiftFormula(src.input, row - srcRow, col - srcCol))
          }
        }
      }
      const next: Record<string, CellData> = {}
      for (const [name, cell] of Object.entries(cells)) {
        if (!dropped.has(name) && written[name] === undefined) {
          next[name] = cell
        }
      }
      for (const [name, cell] of Object.entries(written)) {
        next[name] = cell
      }
      return next
    })
    anchor.value = { row: dest.r0, col: dest.c0 }
    focus.value = { row: dest.r1, col: dest.c1 }
  }

  function copyText(): string {
    const area = rect.value
    const lines: string[] = []
    for (let row = area.r0; row <= area.r1; row += 1) {
      const columns: string[] = []
      for (let col = area.c0; col <= area.c1; col += 1) {
        columns.push(sheet.value?.cells[cellKey(row, col)]?.input ?? '')
      }
      lines.push(columns.join('\t'))
    }
    return lines.join('\n')
  }

  function paste(text: string): void {
    const lines = text.replaceAll('\r\n', '\n').replaceAll('\r', '\n').split('\n')
    if (lines.at(-1) === '') {
      lines.pop()
    }
    const origin = focus.value
    mapCells((cells) => {
      const dropped = new Set<string>()
      const written: Record<string, CellData> = {}
      for (const [rowOffset, line] of lines.entries()) {
        for (const [colOffset, input] of line.split('\t').entries()) {
          const row = origin.row + rowOffset
          const col = origin.col + colOffset
          if (row >= ROW_COUNT || col >= COL_COUNT) {
            continue
          }
          const key = cellKey(row, col)
          if (input === '') {
            dropped.add(key)
          } else {
            written[key] = styled(cells[key], input)
          }
        }
      }
      const next: Record<string, CellData> = {}
      for (const [name, cell] of Object.entries(cells)) {
        if (!dropped.has(name) && written[name] === undefined) {
          next[name] = cell
        }
      }
      for (const [name, cell] of Object.entries(written)) {
        next[name] = cell
      }
      return next
    })
  }

  function jumpTo(text: string): void {
    const [start, end] = text.split(':')
    const from = start === undefined ? null : parseAddress(start)
    if (from === null || from.row >= ROW_COUNT || from.col >= COL_COUNT) {
      return
    }
    const to = end === undefined ? null : parseAddress(end)
    anchor.value = { row: from.row, col: from.col }
    focus.value =
      to !== null && to.row < ROW_COUNT && to.col < COL_COUNT
        ? { row: to.row, col: to.col }
        : { row: from.row, col: from.col }
    editing.value = null
  }

  function onKeydown(event: KeyboardEvent): void {
    if (editing.value !== null) {
      return
    }
    if (event.ctrlKey || event.metaKey) {
      const key = event.key.toLowerCase()
      if (key === 'b') {
        toggleFlag('bold')
      } else if (key === 'i') {
        toggleFlag('italic')
      } else if (key === 'u') {
        toggleFlag('underline')
      } else if (key === 'z' && !event.shiftKey) {
        undo()
      } else if (key === 'z' || key === 'y') {
        redo()
      } else if (key === 'a') {
        anchor.value = { row: 0, col: 0 }
        focus.value = { row: ROW_COUNT - 1, col: COL_COUNT - 1 }
        editing.value = null
      } else {
        return
      }
      event.preventDefault()
      return
    }
    const extend = event.shiftKey
    if (event.key === 'Enter') {
      move(extend ? -1 : 1, 0, false)
    } else if (event.key === 'Tab') {
      move(0, extend ? -1 : 1, false)
    } else if (event.key === 'ArrowUp') {
      move(-1, 0, extend)
    } else if (event.key === 'ArrowDown') {
      move(1, 0, extend)
    } else if (event.key === 'ArrowLeft') {
      move(0, -1, extend)
    } else if (event.key === 'ArrowRight') {
      move(0, 1, extend)
    } else if (event.key === 'Delete' || event.key === 'Backspace') {
      clearSelection()
    } else if (event.key === 'F2') {
      startEdit('cell')
    } else if (event.key.length === 1) {
      startEdit('cell', event.key)
    } else {
      return
    }
    event.preventDefault()
  }

  const stats = computed(() => {
    const area = rect.value
    if (area.r0 === area.r1 && area.c0 === area.c1) {
      return null
    }
    const numbers: number[] = []
    for (let row = area.r0; row <= area.r1; row += 1) {
      for (let col = area.c0; col <= area.c1; col += 1) {
        const value = valueAt(row, col)
        if (value.kind === 'number') {
          numbers.push(value.value)
        }
      }
    }
    if (numbers.length === 0) {
      return null
    }
    const sum = numbers.reduce((total, value) => total + value, 0)
    return { sum, average: sum / numbers.length, count: numbers.length }
  })

  const name = computed(() => {
    const area = rect.value
    const start = formatAddress(area.r0, area.c0)
    if (area.r0 === area.r1 && area.c0 === area.c1) {
      return start
    }
    return `${start}:${formatAddress(area.r1, area.c1)}`
  })

  return {
    workbook,
    active,
    sheet,
    anchor,
    focus,
    rect,
    editing,
    status,
    name,
    stats,
    focusCell,
    colWidth: (col: number) => (sheet.value === undefined ? 100 : colWidth(sheet.value, col)),
    shown,
    valueAt,
    select,
    selectColumn: (col: number, extend: boolean) => {
      anchor.value = { row: 0, col: extend ? anchor.value.col : col }
      focus.value = { row: ROW_COUNT - 1, col }
      editing.value = null
    },
    selectRow: (row: number, extend: boolean) => {
      anchor.value = { row: extend ? anchor.value.row : row, col: 0 }
      focus.value = { row, col: COL_COUNT - 1 }
      editing.value = null
    },
    selectAll: () => {
      anchor.value = { row: 0, col: 0 }
      focus.value = { row: ROW_COUNT - 1, col: COL_COUNT - 1 }
      editing.value = null
    },
    move,
    startEdit,
    commit,
    cancelEdit: () => {
      editing.value = null
    },
    setDraft,
    toggleFlag,
    setAlign,
    setFormat,
    setPaint,
    undo,
    redo,
    canUndo: computed(() => past.value.length > 0),
    canRedo: computed(() => future.value.length > 0),
    armColumnResize,
    retrySave: () => {
      if (status.value === 'error') {
        scheduleSave()
      }
    },
    addSheet,
    renameSheet,
    deleteSheet,
    chooseSheet: (index: number) => {
      active.value = index
      editing.value = null
      select({ row: 0, col: 0 }, false)
    },
    setColWidth,
    fillTo,
    clearSelection,
    copyText,
    paste,
    jumpTo,
    onKeydown,
    formulaText: computed(() => editing.value?.draft ?? focusCell.value?.input ?? ''),
  }
}

export type SheetModel = ReturnType<typeof useSheetModel>

const SHEET_MODEL: InjectionKey<SheetModel> = Symbol('sheet-model')

export function provideSheetModel(model: SheetModel): void {
  provide(SHEET_MODEL, model)
}

export function useSheet(): SheetModel {
  const model = inject(SHEET_MODEL)
  if (model === undefined) {
    throw new Error('useSheet() must be used inside SheetsEditor')
  }
  return model
}
