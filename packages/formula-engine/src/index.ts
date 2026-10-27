/**
 * Browser build of the Sheets calculation engine (Rust, WebAssembly). `just wasm` generates
 * `pkg/`; nothing in it is edited by hand.
 */
import init, { Engine, initSync } from '../pkg/spreadsheets_engine_wasm.js'
import wasmUrl from '../pkg/spreadsheets_engine_wasm_bg.wasm?url'

export type ErrorCode =
  | '#NULL!'
  | '#DIV/0!'
  | '#VALUE!'
  | '#REF!'
  | '#NAME?'
  | '#NUM!'
  | '#N/A'
  | '#GETTING_DATA'
  | '#SPILL!'
  | '#CALC!'
  | '#CIRC!'

export type CellValue =
  | { kind: 'empty' }
  | { kind: 'number'; value: number }
  | { kind: 'text'; value: string }
  | { kind: 'bool'; value: boolean }
  | { kind: 'error'; value: ErrorCode }

/** One cell edit. Inputs starting with `=` are formulas in the stored syntax. */
export interface CellEdit {
  row: number
  col: number
  input: string
}

/** A workbook being calculated. Sheets are addressed by their position. */
export interface FormulaEngine {
  /** Replaces the whole workbook with a stored document (`{ sheets: [...] }`). */
  load: (workbookJson: string) => void
  sheetCount: () => number
  valueAt: (sheet: number, row: number, col: number) => CellValue
  /** Applies edits to one sheet and recalculates the cells that depend on them. */
  setInputs: (sheet: number, edits: readonly CellEdit[]) => void
  /** Frees the WebAssembly memory. The engine is unusable afterwards. */
  dispose: () => void
}

const ERROR_CODES: ReadonlySet<string> = new Set<ErrorCode>([
  '#NULL!',
  '#DIV/0!',
  '#VALUE!',
  '#REF!',
  '#NAME?',
  '#NUM!',
  '#N/A',
  '#GETTING_DATA',
  '#SPILL!',
  '#CALC!',
  '#CIRC!',
])

function isErrorCode(value: unknown): value is ErrorCode {
  return typeof value === 'string' && ERROR_CODES.has(value)
}

/** Validates the object the WebAssembly side returns; anything unexpected reads as `#VALUE!`. */
function toCellValue(raw: unknown): CellValue {
  if (typeof raw === 'object' && raw !== null && 'kind' in raw) {
    const value = 'value' in raw ? raw.value : undefined
    switch (raw.kind) {
      case 'empty':
        return { kind: 'empty' }
      case 'number':
        if (typeof value === 'number') {
          return { kind: 'number', value }
        }
        break
      case 'text':
        if (typeof value === 'string') {
          return { kind: 'text', value }
        }
        break
      case 'bool':
        if (typeof value === 'boolean') {
          return { kind: 'bool', value }
        }
        break
      case 'error':
        if (isErrorCode(value)) {
          return { kind: 'error', value }
        }
        break
    }
  }
  return { kind: 'error', value: '#VALUE!' }
}

const EMPTY_WORKBOOK = JSON.stringify({ sheets: [{ id: 's1', name: 'Sheet1', cells: {} }] })

let ready: Promise<void> | null = null

/**
 * Compiles the WebAssembly module once. Browsers fetch it; tests pass the bytes read from disk
 * (`initSync` needs no network).
 */
export function loadFormulaEngine(bytes?: BufferSource): Promise<void> {
  if (bytes !== undefined) {
    initSync({ module: bytes })
    ready = Promise.resolve()
  }
  ready ??= init({ module_or_path: wasmUrl }).then(() => undefined)
  return ready
}

/** Creates an engine holding `workbookJson`. {@link loadFormulaEngine} must have resolved. */
export function createFormulaEngine(workbookJson: string = EMPTY_WORKBOOK): FormulaEngine {
  const engine = new Engine(workbookJson)
  return {
    load: (json) => {
      engine.load(json)
    },
    sheetCount: () => engine.sheetCount(),
    valueAt: (sheet, row, col) => toCellValue(engine.valueAt(sheet, row, col)),
    setInputs: (sheet, edits) => {
      engine.setInputs(
        sheet,
        Uint32Array.from(edits, (edit) => edit.row),
        Uint32Array.from(edits, (edit) => edit.col),
        edits.map((edit) => edit.input),
      )
    },
    dispose: () => {
      engine.free()
    },
  }
}
