import type { CellValue } from '@office/formula-engine'

export const ROW_COUNT = 500
export const COL_COUNT = 40
export const ROW_HEIGHT = 22
export const DEFAULT_COL_WIDTH = 100
export const MIN_COL_WIDTH = 32
export const MAX_COL_WIDTH = 800

export type Align = 'left' | 'center' | 'right'
export type NumberFormat = 'number' | 'percent' | 'currency'

export interface CellData {
  input: string
  bold?: boolean
  italic?: boolean
  underline?: boolean
  align?: Align
  color?: string
  fill?: string
  format?: NumberFormat
}

export interface SheetData {
  id: string
  name: string
  cells: Record<string, CellData>
  cols?: Record<string, number>
}

export interface WorkbookData {
  sheets: SheetData[]
}

export interface CellPos {
  row: number
  col: number
}

export interface Rect {
  r0: number
  c0: number
  r1: number
  c1: number
}

export function cellKey(row: number, col: number): string {
  return `${String(row)},${String(col)}`
}

export function rectOf(anchor: CellPos, focus: CellPos): Rect {
  return {
    r0: Math.min(anchor.row, focus.row),
    c0: Math.min(anchor.col, focus.col),
    r1: Math.max(anchor.row, focus.row),
    c1: Math.max(anchor.col, focus.col),
  }
}

export function contains(rect: Rect, row: number, col: number): boolean {
  return row >= rect.r0 && row <= rect.r1 && col >= rect.c0 && col <= rect.c1
}

const HEX = /^#[0-9a-fA-F]{6}$/

function readAlign(value: unknown): Align | undefined {
  if (value === 'left' || value === 'center' || value === 'right') {
    return value
  }
  return undefined
}

function readFormat(value: unknown): NumberFormat | undefined {
  if (value === 'number' || value === 'percent' || value === 'currency') {
    return value
  }
  return undefined
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value)
}

function cellFrom(value: unknown): CellData | null {
  if (!isRecord(value) || typeof value.input !== 'string') {
    return null
  }
  const cell: CellData = { input: value.input }
  if (value.bold === true) {
    cell.bold = true
  }
  if (value.italic === true) {
    cell.italic = true
  }
  if (value.underline === true) {
    cell.underline = true
  }
  const align = readAlign(value.align)
  if (align !== undefined) {
    cell.align = align
  }
  if (typeof value.color === 'string' && HEX.test(value.color)) {
    cell.color = value.color
  }
  if (typeof value.fill === 'string' && HEX.test(value.fill)) {
    cell.fill = value.fill
  }
  const format = readFormat(value.format)
  if (format !== undefined) {
    cell.format = format
  }
  return cell
}

export function parseWorkbook(value: unknown): WorkbookData | null {
  if (!isRecord(value) || !Array.isArray(value.sheets) || value.sheets.length === 0) {
    return null
  }
  const sheets: SheetData[] = []
  for (const raw of value.sheets) {
    if (!isRecord(raw) || typeof raw.id !== 'string' || typeof raw.name !== 'string') {
      return null
    }
    if (!isRecord(raw.cells)) {
      return null
    }
    const cells: Record<string, CellData> = {}
    for (const [key, cell] of Object.entries(raw.cells)) {
      const parsed = cellFrom(cell)
      if (!/^\d+,\d+$/.test(key) || parsed === null) {
        return null
      }
      cells[key] = parsed
    }
    const sheet: SheetData = { id: raw.id, name: raw.name, cells }
    if (raw.cols !== undefined) {
      if (!isRecord(raw.cols)) {
        return null
      }
      const cols: Record<string, number> = {}
      for (const [key, width] of Object.entries(raw.cols)) {
        if (!/^\d+$/.test(key) || typeof width !== 'number') {
          return null
        }
        cols[key] = width
      }
      sheet.cols = cols
    }
    sheets.push(sheet)
  }
  return { sheets }
}

/** JSON object the API accepts for a workbook. */
export function workbookContent(workbook: WorkbookData): Record<string, unknown> {
  const encoded: unknown = JSON.parse(JSON.stringify(workbook))
  if (!isRecord(encoded)) {
    return { sheets: [] }
  }
  return encoded
}

export function colWidth(sheet: SheetData, col: number): number {
  const width = sheet.cols?.[String(col)]
  return width ?? DEFAULT_COL_WIDTH
}

export function displayAlign(cell: CellData | undefined, value: CellValue): Align {
  if (cell?.align !== undefined) {
    return cell.align
  }
  return value.kind === 'number' ? 'right' : 'left'
}
