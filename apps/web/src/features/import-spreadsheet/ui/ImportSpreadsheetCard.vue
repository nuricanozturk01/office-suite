<script setup lang="ts">
import { FileUp } from '@lucide/vue'
import { useTemplateRef } from 'vue'
import { useI18n } from 'vue-i18n'

import type { SpreadsheetDetail } from '@/entities/spreadsheet'
import { errorMessageKey } from '@/shared/api'
import { InlineAlert } from '@/shared/ui'

import { useImportSpreadsheet } from '../model/use-import-spreadsheet'

const emit = defineEmits<{ imported: [spreadsheet: SpreadsheetDetail] }>()

const { t } = useI18n()
const input = useTemplateRef<HTMLInputElement>('input')
const importFile = useImportSpreadsheet((spreadsheet) => {
  emit('imported', spreadsheet)
})

function onPick(event: Event): void {
  const target = event.target as HTMLInputElement // change events from our own <input type=file>
  const file = target.files?.[0]
  target.value = ''
  if (file !== undefined) {
    importFile.mutate(file)
  }
}
</script>

<template>
  <div class="flex flex-col gap-2">
    <input
      ref="input"
      type="file"
      class="hidden"
      aria-hidden="true"
      :aria-label="t('sheets.upload')"
      tabindex="-1"
      accept=".xlsx,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
      @change="onPick"
    />
    <button
      type="button"
      class="inline-flex h-10 items-center gap-2 rounded-full border border-border bg-surface-raised px-4 text-sm font-medium transition hover:bg-surface-muted disabled:opacity-60"
      :disabled="importFile.isPending.value"
      :aria-busy="importFile.isPending.value"
      @click="input?.click()"
    >
      <FileUp class="size-4" aria-hidden="true" />
      {{ t('sheets.upload') }}
    </button>
    <InlineAlert v-if="importFile.isError.value" class="w-52">
      {{ t(errorMessageKey(importFile.error.value)) }}
    </InlineAlert>
  </div>
</template>
