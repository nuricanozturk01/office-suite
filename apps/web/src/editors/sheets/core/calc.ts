import {
  type CellEdit,
  type CellValue,
  createFormulaEngine,
  type FormulaEngine,
} from '@office/formula-engine'

import { workbookContent, type WorkbookData } from './workbook'

/** More edits than this reload the workbook instead: cheaper than patching cell by cell. */
const PATCH_LIMIT = 2000

type Inputs = Map<string, string>

function inputsOf(cells: Record<string, { input: string }>): Inputs {
  return new Map(Object.entries(cells).map(([key, cell]) => [key, cell.input]))
}

function parseKey(key: string): { row: number; col: number } | null {
  const [row, col] = key.split(',').map(Number)
  return row === undefined || col === undefined || Number.isNaN(row) || Number.isNaN(col)
    ? null
    : { row, col }
}

/**
 * Keeps a WebAssembly {@link FormulaEngine} in step with the workbook the grid edits. The grid
 * replaces workbook data instead of mutating it, so {@link WorkbookCalc.sync} finds what
 * changed and patches only that; structural changes (sheets added, renamed, reordered) reload.
 */
export interface WorkbookCalc {
  /** Brings the engine up to date with `workbook`. Cheap when nothing changed. */
  sync: (workbook: WorkbookData) => void
  valueAt: (sheet: number, row: number, col: number) => CellValue
  dispose: () => void
}

export function createWorkbookCalc(initial: WorkbookData): WorkbookCalc {
  const engine: FormulaEngine = createFormulaEngine(JSON.stringify(workbookContent(initial)))
  let synced: WorkbookData = initial
  let ids: string[] = initial.sheets.map((sheet) => `${sheet.id}\u0000${sheet.name}`)
  let inputs: Inputs[] = initial.sheets.map((sheet) => inputsOf(sheet.cells))

  function reload(workbook: WorkbookData): void {
    engine.load(JSON.stringify(workbookContent(workbook)))
    synced = workbook
    ids = workbook.sheets.map((sheet) => `${sheet.id}\u0000${sheet.name}`)
    inputs = workbook.sheets.map((sheet) => inputsOf(sheet.cells))
  }

  function changes(workbook: WorkbookData): CellEdit[][] | null {
    const perSheet: CellEdit[][] = []
    let total = 0
    for (const [index, sheet] of workbook.sheets.entries()) {
      const before = inputs[index]
      const edits: CellEdit[] = []
      const previous = synced.sheets[index]
      if (before === undefined) {
        return null
      }
      if (previous?.cells !== sheet.cells) {
        for (const [key, cell] of Object.entries(sheet.cells)) {
          if (before.get(key) !== cell.input) {
            const at = parseKey(key)
            if (at === null) {
              return null
            }
            edits.push({ ...at, input: cell.input })
          }
        }
        for (const key of before.keys()) {
          if (!(key in sheet.cells)) {
            const at = parseKey(key)
            if (at === null) {
              return null
            }
            edits.push({ ...at, input: '' })
          }
        }
      }
      total += edits.length
      if (total > PATCH_LIMIT) {
        return null
      }
      perSheet.push(edits)
    }
    return perSheet
  }

  function sync(workbook: WorkbookData): void {
    if (workbook === synced) {
      return
    }
    const sameShape =
      workbook.sheets.length === ids.length &&
      workbook.sheets.every((sheet, index) => `${sheet.id}\u0000${sheet.name}` === ids[index])
    const perSheet = sameShape ? changes(workbook) : null
    if (perSheet === null) {
      reload(workbook)
      return
    }
    try {
      for (const [index, edits] of perSheet.entries()) {
        if (edits.length > 0) {
          engine.setInputs(index, edits)
        }
      }
    } catch {
      reload(workbook)
      return
    }
    inputs = workbook.sheets.map((sheet, index) =>
      synced.sheets[index]?.cells === sheet.cells
        ? (inputs[index] ?? new Map())
        : inputsOf(sheet.cells),
    )
    synced = workbook
  }

  return {
    sync,
    valueAt: (sheet, row, col) => engine.valueAt(sheet, row, col),
    dispose: () => {
      engine.dispose()
    },
  }
}
