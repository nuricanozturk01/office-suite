import type { CellValue } from '@office/formula-engine'
import type { NumberFormat } from './workbook'

/** How a computed value is shown in a cell. The formula bar shows the raw input instead. */
export function formatValue(
  value: CellValue,
  format: NumberFormat | undefined,
  locale: string,
  bools: { on: string; off: string },
): string {
  switch (value.kind) {
    case 'empty':
      return ''
    case 'error':
      return value.value
    case 'bool':
      return value.value ? bools.on : bools.off
    case 'text':
      return value.value
    case 'number':
      return formatNumber(value.value, format, locale)
  }
}

function formatNumber(value: number, format: NumberFormat | undefined, locale: string): string {
  if (!Number.isFinite(value)) {
    return '#NUM!'
  }
  if (format === 'percent') {
    return new Intl.NumberFormat(locale, { style: 'percent', maximumFractionDigits: 2 }).format(
      value,
    )
  }
  if (format === 'currency') {
    const currency = locale.toLowerCase().startsWith('tr') ? 'TRY' : 'USD'
    return new Intl.NumberFormat(locale, { style: 'currency', currency }).format(value)
  }
  return new Intl.NumberFormat(locale, {
    maximumFractionDigits: 10,
    useGrouping: format === 'number',
  }).format(value)
}
