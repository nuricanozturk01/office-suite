// @vitest-environment node
/// <reference types="node" />
import { readFileSync } from 'node:fs'

import { type CellValue, loadFormulaEngine } from '@office/formula-engine'
import { beforeAll, describe, expect, it } from 'vitest'

import { shiftFormula } from './address'
import { createWorkbookCalc } from './calc'
import type { WorkbookData } from './workbook'

beforeAll(async () => {
  const wasm = new URL(
    '../../../../../../packages/formula-engine/pkg/spreadsheets_engine_wasm_bg.wasm',
    import.meta.url,
  )
  await loadFormulaEngine(readFileSync(wasm))
})

function book(cells: Record<string, string>, name = 'Sayfa1'): WorkbookData {
  return {
    sheets: [
      {
        id: 's1',
        name,
        cells: Object.fromEntries(Object.entries(cells).map(([key, input]) => [key, { input }])),
      },
    ],
  }
}

function plain(value: CellValue): number | string | boolean | null {
  return value.kind === 'empty' ? null : value.value
}

function evaluate(formula: string, others: Record<string, string> = {}): CellValue {
  const calc = createWorkbookCalc(book({ '0,0': formula, ...others }))
  const value = calc.valueAt(0, 0, 0)
  calc.dispose()
  return value
}

describe('formulas', () => {
  it('respects arithmetic precedence', () => {
    expect(plain(evaluate('=1+2*3'))).toBe(7)
    expect(plain(evaluate('=(1+2)*3'))).toBe(9)
    // Excel (and the engine) read a^b^c left to right.
    expect(plain(evaluate('=2^3^2'))).toBe(64)
  })

  it('accepts Turkish names, decimal commas and semicolons', () => {
    expect(plain(evaluate('=TOPLA(1,5;2,5)'))).toBe(4)
    expect(plain(evaluate('=EĞER(1>2;"hayır";"evet")'))).toBe('evet')
    expect(plain(evaluate('=1,5+2'))).toBe(3.5)
  })

  it('sums a range and ignores text', () => {
    const value = evaluate('=SUM(A2:A4)', { '1,0': '10', '2,0': 'hello', '3,0': '5' })
    expect(plain(value)).toBe(15)
  })

  it('reports division by zero, unknown names and cycles', () => {
    expect(plain(evaluate('=1/0'))).toBe('#DIV/0!')
    expect(plain(evaluate('=NOPE(1)'))).toBe('#NAME?')
    expect(plain(evaluate('=A2', { '1,0': '=A1' }))).toBe('#CIRC!')
  })

  it('reads percent and boolean literals', () => {
    expect(plain(evaluate('12%'))).toBe(0.12)
    expect(plain(evaluate('DOĞRU'))).toBe(true)
    expect(plain(evaluate('Q3'))).toBe('Q3')
  })

  it('evaluates functions beyond the basic set', () => {
    expect(
      plain(
        evaluate('=XLOOKUP("b";A2:A3;B2:B3)', { '1,0': 'a', '2,0': 'b', '1,1': '1', '2,1': '2' }),
      ),
    ).toBe(2)
    expect(
      plain(
        evaluate('=SUMIFS(B2:B3;A2:A3;"b")', { '1,0': 'a', '2,0': 'b', '1,1': '1', '2,1': '2' }),
      ),
    ).toBe(2)
    expect(plain(evaluate('=TEXT(1234,5;"#,##0.0")'))).toBe('1,234.5')
  })
})

describe('createWorkbookCalc', () => {
  it('patches edits and recalculates dependents', () => {
    const first = book({ '0,0': '2', '1,0': '=A1*10' })
    const calc = createWorkbookCalc(first)
    expect(plain(calc.valueAt(0, 1, 0))).toBe(20)

    calc.sync(book({ '0,0': '5', '1,0': '=A1*10' }))
    expect(plain(calc.valueAt(0, 1, 0))).toBe(50)

    calc.sync(book({ '1,0': '=A1*10' }))
    expect(plain(calc.valueAt(0, 0, 0))).toBeNull()
    expect(plain(calc.valueAt(0, 1, 0))).toBe(0)
    calc.dispose()
  })

  it('reloads when sheets are added, renamed or removed', () => {
    const calc = createWorkbookCalc(book({ '0,0': '1' }))
    calc.sync({
      sheets: [
        { id: 's1', name: 'Sayfa1', cells: { '0,0': { input: '1' } } },
        { id: 's2', name: 'Toplam', cells: { '0,0': { input: '=Sayfa1!A1+1' } } },
      ],
    })
    expect(plain(calc.valueAt(1, 0, 0))).toBe(2)

    calc.sync(book({ '0,0': '7' }, 'Yeni'))
    expect(plain(calc.valueAt(0, 0, 0))).toBe(7)
    calc.dispose()
  })

  it('ignores style-only changes', () => {
    const initial = book({ '0,0': '3' })
    const calc = createWorkbookCalc(initial)
    const bold: WorkbookData = {
      sheets: [{ id: 's1', name: 'Sayfa1', cells: { '0,0': { input: '3', bold: true } } }],
    }
    calc.sync(bold)
    expect(plain(calc.valueAt(0, 0, 0))).toBe(3)
    calc.dispose()
  })

  it('falls back to a reload for a bulk change', () => {
    const calc = createWorkbookCalc(book({ '0,0': '1' }))
    const many: Record<string, string> = {}
    for (let row = 0; row < 2500; row += 1) {
      many[`${String(row)},0`] = String(row)
    }
    calc.sync(book(many))
    expect(plain(calc.valueAt(0, 2499, 0))).toBe(2499)
    calc.dispose()
  })
})

describe('shiftFormula', () => {
  it('moves relative references and keeps absolute ones', () => {
    expect(shiftFormula('=A1+$B$2', 1, 1)).toBe('=B2+$B$2')
    expect(shiftFormula('hello', 1, 0)).toBe('hello')
  })
})
