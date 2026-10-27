<script setup lang="ts">
import { reactive, ref } from 'vue'
import { useI18n } from 'vue-i18n'

import { useSheet } from './use-sheet-model'

const { t } = useI18n()
const model = reactive(useSheet())
const naming = ref(false)
const nameDraft = ref('')

function onNameFocus(): void {
  naming.value = true
  nameDraft.value = model.name
}

function onNameInput(event: Event): void {
  if (event.target instanceof HTMLInputElement) {
    nameDraft.value = event.target.value
  }
}

function jump(event: KeyboardEvent): void {
  model.jumpTo(nameDraft.value)
  naming.value = false
  if (event.target instanceof HTMLInputElement) {
    event.target.blur()
  }
}

function onFormulaInput(event: Event): void {
  if (!(event.target instanceof HTMLInputElement)) {
    return
  }
  if (model.editing?.source !== 'bar') {
    model.startEdit('bar', event.target.value)
    return
  }
  model.setDraft(event.target.value)
}

function onFormulaKeydown(event: KeyboardEvent): void {
  if (event.key === 'Enter') {
    model.commit({ row: model.focus.row + (event.shiftKey ? -1 : 1), col: model.focus.col })
  } else if (event.key === 'Tab') {
    model.commit({ row: model.focus.row, col: model.focus.col + (event.shiftKey ? -1 : 1) })
  } else if (event.key === 'Escape') {
    model.cancelEdit()
  } else {
    return
  }
  event.preventDefault()
  if (event.target instanceof HTMLInputElement) {
    event.target.blur()
  }
}
</script>

<template>
  <div class="flex items-center gap-2 border-b border-border bg-surface px-2 py-1">
    <input
      class="h-7 w-28 shrink-0 rounded border border-border bg-surface px-2 text-center text-sm"
      :aria-label="t('sheets.nameBox')"
      :value="naming ? nameDraft : model.name"
      @focus="onNameFocus"
      @input="onNameInput"
      @keydown.enter.prevent="jump"
      @blur="naming = false"
    />
    <span class="text-text-muted" aria-hidden="true">fx</span>
    <input
      class="h-7 min-w-0 flex-1 rounded border border-border bg-surface px-2 font-mono text-sm"
      :aria-label="t('sheets.formulaBar')"
      :value="model.formulaText"
      @focus="model.startEdit('bar')"
      @input="onFormulaInput"
      @keydown="onFormulaKeydown"
      @blur="model.commit(null)"
    />
  </div>
</template>
