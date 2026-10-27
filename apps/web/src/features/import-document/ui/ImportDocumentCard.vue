<script setup lang="ts">
import { FileUp } from '@lucide/vue'
import { useTemplateRef } from 'vue'
import { useI18n } from 'vue-i18n'

import type { DocumentSummary } from '@/entities/document'
import { errorMessageKey } from '@/shared/api'
import { InlineAlert } from '@/shared/ui'

import { useImportDocument } from '../model/use-import-document'

const emit = defineEmits<{ imported: [document: DocumentSummary] }>()

const { t } = useI18n()
const input = useTemplateRef<HTMLInputElement>('input')
const importFile = useImportDocument((document) => {
  emit('imported', document)
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
      :aria-label="t('documents.upload')"
      tabindex="-1"
      accept=".docx,application/vnd.openxmlformats-officedocument.wordprocessingml.document"
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
      {{ t('documents.upload') }}
    </button>
    <InlineAlert v-if="importFile.isError.value" class="w-52">
      {{ t(errorMessageKey(importFile.error.value)) }}
    </InlineAlert>
  </div>
</template>
