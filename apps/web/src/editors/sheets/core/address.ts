/** Column and cell addresses (`A`, `AA`, `A1`, `$B$2`). */

const ADDRESS = /^(\$?)([A-Za-z]+)(\$?)(\d+)$/

export interface CellAddress {
  row: number
  col: number
  absRow: boolean
  absCol: boolean
}

export function colLabel(index: number): string {
  let n = index + 1
  let label = ''
  while (n > 0) {
    const rem = (n - 1) % 26
    label = String.fromCharCode(65 + rem) + label
    n = Math.floor((n - 1) / 26)
  }
  return label
}

/** Zero-based column index, or `-1` when `label` is not letters. */
export function parseCol(label: string): number {
  let n = 0
  for (const char of label.toUpperCase()) {
    if (char < 'A' || char > 'Z') {
      return -1
    }
    n = n * 26 + (char.charCodeAt(0) - 64)
  }
  return n - 1
}

export function parseAddress(text: string): CellAddress | null {
  const match = ADDRESS.exec(text.trim())
  if (match?.[2] === undefined || match[4] === undefined) {
    return null
  }
  const row = Number(match[4]) - 1
  const col = parseCol(match[2])
  if (col < 0 || row < 0) {
    return null
  }
  return { row, col, absCol: match[1] === '$', absRow: match[3] === '$' }
}

export function formatAddress(row: number, col: number): string {
  return `${colLabel(col)}${String(row + 1)}`
}

/** Shifts relative references inside a formula. Absolute ones (`$A$1`) stay put. */
export function shiftFormula(input: string, dRow: number, dCol: number): string {
  if (!input.startsWith('=') || (dRow === 0 && dCol === 0)) {
    return input
  }
  return input.replaceAll(
    /(\$?)([A-Za-z]+)(\$?)(\d+)/g,
    (_match, absCol: string, col: string, absRow: string, row: string) => {
      const nextCol = absCol === '$' ? parseCol(col) : parseCol(col) + dCol
      const nextRow = absRow === '$' ? Number(row) : Number(row) + dRow
      if (nextCol < 0 || nextRow < 1) {
        return '#REF!'
      }
      return `${absCol}${colLabel(nextCol)}${absRow}${String(nextRow)}`
    },
  )
}
