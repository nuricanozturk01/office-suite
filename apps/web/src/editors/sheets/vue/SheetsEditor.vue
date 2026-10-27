<script setup lang="ts">
import { Download } from '@lucide/vue'
import {
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuPortal,
  DropdownMenuRoot,
  DropdownMenuTrigger,
} from 'reka-ui'
import { computed, reactive, ref } from 'vue'
import { useI18n } from 'vue-i18n'

import { sheetToCsv } from '../core/csv'
import type { WorkbookData } from '../core/workbook'
import { downloadXlsx } from '../core/xlsx'
import FormulaBar from './FormulaBar.vue'
import SheetGrid from './SheetGrid.vue'
import SheetsToolbar from './SheetsToolbar.vue'
import SheetTabs from './SheetTabs.vue'
import { provideSheetModel, useSheetModel } from './use-sheet-model'

const { spreadsheetId, initial, version, title } = defineProps<{
  spreadsheetId: string
  initial: WorkbookData
  version: number
  title: string
}>()

const emit = defineEmits<{ reload: []; newSpreadsheet: [] }>()

defineSlots<{
  leading?: () => unknown
  title?: () => unknown
  actions?: () => unknown
}>()

const { t, locale } = useI18n()
const model = useSheetModel(spreadsheetId, initial, version)
provideSheetModel(model)
const ui = reactive(model)

const saveLabel = computed(() => {
  switch (ui.status) {
    case 'saved':
      return t('sheets.saved')
    case 'saving':
      return t('sheets.saving')
    case 'error':
      return t('sheets.saveError')
    case 'conflict':
      return t('sheets.conflict')
  }
})

const statsLabel = computed(() => {
  const stats = ui.stats
  if (stats === null) {
    return ''
  }
  const format = new Intl.NumberFormat(locale.value, { maximumFractionDigits: 4 })
  return [
    `${t('sheets.sum')}: ${format.format(stats.sum)}`,
    `${t('sheets.average')}: ${format.format(stats.average)}`,
    `${t('sheets.count')}: ${String(stats.count)}`,
  ].join('   ')
})

function downloadCsv(): void {
  const sheet = model.sheet.value
  if (sheet === undefined) {
    return
  }
  const separator = locale.value.toLowerCase().startsWith('tr') ? ';' : ','
  const csv = sheetToCsv(sheet, separator)
  const url = URL.createObjectURL(new Blob([csv], { type: 'text/csv;charset=utf-8' }))
  const link = document.createElement('a')
  link.href = url
  link.download = `${title.trim() || t('sheets.untitled')}.csv`
  link.click()
  URL.revokeObjectURL(url)
}

const exportError = ref(false)

async function downloadWorkbook(): Promise<void> {
  exportError.value = false
  try {
    await downloadXlsx(spreadsheetId, title.trim() || t('sheets.untitled'))
  } catch {
    exportError.value = true
  }
}
</script>

<template>
  <div class="flex h-dvh flex-col bg-surface-muted text-text">
    <header class="flex items-center gap-3 bg-surface px-3 pt-2 pb-1">
      <slot name="leading" />
      <div class="min-w-0 flex-1">
        <div class="flex items-center gap-3">
          <slot name="title" />
          <p class="truncate text-xs text-text-muted">{{ saveLabel }}</p>
          <button
            v-if="ui.status === 'error'"
            type="button"
            class="text-xs text-accent"
            @click="ui.retrySave()"
          >
            {{ t('sheets.retrySave') }}
          </button>
        </div>
      </div>
      <DropdownMenuRoot>
        <DropdownMenuTrigger
          class="rounded px-2 py-1 text-sm hover:bg-surface-muted"
          :aria-label="t('sheets.fileMenu')"
        >
          {{ t('editor.menu.file') }}
        </DropdownMenuTrigger>
        <DropdownMenuPortal>
          <DropdownMenuContent
            class="z-50 min-w-48 rounded-md border border-border bg-surface-raised py-1.5 text-sm shadow-lg"
            align="end"
          >
            <DropdownMenuItem
              class="flex cursor-default items-center gap-3 px-4 py-1.5 outline-none data-highlighted:bg-surface-muted"
              @select="emit('newSpreadsheet')"
            >
              {{ t('sheets.new') }}
            </DropdownMenuItem>
            <DropdownMenuItem
              class="flex cursor-default items-center gap-3 px-4 py-1.5 outline-none data-highlighted:bg-surface-muted"
              @select="downloadCsv"
            >
              <Download class="size-4" aria-hidden="true" />
              {{ t('sheets.downloadCsv') }}
            </DropdownMenuItem>
            <DropdownMenuItem
              class="flex cursor-default items-center gap-3 px-4 py-1.5 outline-none data-highlighted:bg-surface-muted"
              @select="downloadWorkbook"
            >
              <Download class="size-4" aria-hidden="true" />
              {{ t('sheets.downloadXlsx') }}
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenuPortal>
      </DropdownMenuRoot>
      <slot name="actions" />
    </header>
    <p v-if="exportError" class="bg-danger-surface px-4 py-2 text-sm text-danger" role="alert">
      {{ t('sheets.exportError') }}
    </p>
    <div
      v-if="ui.status === 'conflict'"
      class="flex items-center gap-3 bg-danger-surface px-4 py-2 text-sm text-danger"
      role="alert"
    >
      <p class="flex-1">{{ t('sheets.conflict') }}</p>
      <button type="button" class="font-medium underline" @click="emit('reload')">
        {{ t('sheets.reload') }}
      </button>
    </div>
    <div class="bg-surface px-2 pb-1">
      <SheetsToolbar />
    </div>
    <FormulaBar />
    <SheetGrid />
    <footer class="flex items-center gap-4 border-t border-border bg-surface px-2 py-1">
      <SheetTabs class="min-w-0 flex-1" />
      <p class="shrink-0 text-xs text-text-muted" :aria-label="t('sheets.stats')">
        {{ statsLabel }}
      </p>
    </footer>
  </div>
</template>
