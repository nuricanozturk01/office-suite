import { cellKey, type SheetData } from './workbook'

function escape(value: string, separator: string): string {
  if (/["\n\r]/.test(value) || value.includes(separator)) {
    return `"${value.replaceAll('"', '""')}"`
  }
  return value
}

/** Used range as CSV. Turkish Excel expects `;` between columns. */
export function sheetToCsv(sheet: SheetData, separator: string): string {
  let maxRow = 0
  let maxCol = 0
  for (const key of Object.keys(sheet.cells)) {
    const [row, col] = key.split(',')
    maxRow = Math.max(maxRow, Number(row))
    maxCol = Math.max(maxCol, Number(col))
  }
  const lines: string[] = []
  for (let row = 0; row <= maxRow; row += 1) {
    const columns: string[] = []
    for (let col = 0; col <= maxCol; col += 1) {
      columns.push(escape(sheet.cells[cellKey(row, col)]?.input ?? '', separator))
    }
    lines.push(columns.join(separator))
  }
  return `\uFEFF${lines.join('\n')}`
}
