<script setup lang="ts">
import type { Editor } from '@tiptap/core'
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'

import { insertTable } from '../../core/commands'
import DialogActions from './DialogActions.vue'
import EditorDialog from './EditorDialog.vue'

const { editor } = defineProps<{ editor: Editor }>()

const MAX_CELLS = 20

const open = defineModel<boolean>('open', { required: true })

const { t } = useI18n()

const rows = ref(3)
const columns = ref(3)

const clamp = (value: number) => Math.min(MAX_CELLS, Math.max(1, Math.round(value) || 1))

function submit(): void {
  open.value = false
  insertTable(editor, clamp(rows.value), clamp(columns.value))
}
</script>

<template>
  <EditorDialog v-model:open="open" :title="t('editor.table.title')">
    <form @submit.prevent="submit">
      <div class="grid grid-cols-2 gap-4">
        <label class="text-sm font-medium">
          {{ t('editor.table.rows') }}
          <input
            v-model.number="rows"
            type="number"
            min="1"
            :max="MAX_CELLS"
            class="mt-1 block w-full rounded border border-border bg-surface px-3 py-2 font-normal outline-none focus:ring-2 focus:ring-focus"
          />
        </label>
        <label class="text-sm font-medium">
          {{ t('editor.table.columns') }}
          <input
            v-model.number="columns"
            type="number"
            min="1"
            :max="MAX_CELLS"
            class="mt-1 block w-full rounded border border-border bg-surface px-3 py-2 font-normal outline-none focus:ring-2 focus:ring-focus"
          />
        </label>
      </div>
      <DialogActions :submit-label="t('editor.table.insert')" />
    </form>
  </EditorDialog>
</template>
