<script setup lang="ts">
import { Plus, X } from '@lucide/vue'
import { reactive, ref } from 'vue'
import { useI18n } from 'vue-i18n'

import { useSheet } from './use-sheet-model'

const { t } = useI18n()
const model = reactive(useSheet())
const renaming = ref<number | null>(null)
const draft = ref('')

function begin(index: number, name: string): void {
  renaming.value = index
  draft.value = name
}

function focusInput(element: unknown): void {
  if (element instanceof HTMLInputElement) {
    element.focus()
    element.select()
  }
}

function commit(index: number): void {
  model.renameSheet(index, draft.value)
  renaming.value = null
}

function remove(index: number, name: string): void {
  if (window.confirm(t('sheets.deleteSheetConfirm', { name }))) {
    model.deleteSheet(index)
  }
}

function onDraft(event: Event): void {
  if (event.target instanceof HTMLInputElement) {
    draft.value = event.target.value
  }
}
</script>

<template>
  <div
    class="flex min-w-0 items-end gap-1 overflow-x-auto"
    role="tablist"
    :aria-label="t('sheets.tabs')"
  >
    <div v-for="(sheet, index) in model.workbook.sheets" :key="sheet.id" class="flex items-center">
      <input
        v-if="renaming === index"
        :ref="focusInput"
        class="h-7 w-28 rounded border border-accent bg-surface px-2 text-sm"
        :value="draft"
        :aria-label="t('sheets.renameSheet')"
        maxlength="100"
        @input="onDraft"
        @blur="commit(index)"
        @keydown.enter.prevent="commit(index)"
        @keydown.esc.prevent="renaming = null"
      />
      <button
        v-else
        type="button"
        role="tab"
        class="h-7 max-w-40 truncate rounded-t px-3 text-sm"
        :class="
          index === model.active
            ? 'bg-white font-medium text-[#202124]'
            : 'text-text-muted hover:bg-surface-muted'
        "
        :aria-selected="index === model.active"
        @click="model.chooseSheet(index)"
        @dblclick="begin(index, sheet.name)"
      >
        {{ sheet.name }}
      </button>
      <button
        v-if="renaming !== index && model.workbook.sheets.length > 1 && index === model.active"
        type="button"
        class="rounded p-1 text-text-muted hover:bg-surface-muted"
        :aria-label="t('sheets.deleteSheet', { name: sheet.name })"
        @click="remove(index, sheet.name)"
      >
        <X class="size-3.5" aria-hidden="true" />
      </button>
    </div>
    <button
      type="button"
      class="mb-0.5 rounded p-1 text-text-muted hover:bg-surface-muted"
      :aria-label="t('sheets.addSheet')"
      :disabled="model.workbook.sheets.length >= 20"
      @click="model.addSheet(t('sheets.sheetName', { index: model.workbook.sheets.length + 1 }))"
    >
      <Plus class="size-4" aria-hidden="true" />
    </button>
  </div>
</template>
