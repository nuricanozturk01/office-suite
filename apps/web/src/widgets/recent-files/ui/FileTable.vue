<script setup lang="ts">
import { useI18n } from 'vue-i18n'

import type { FileKind } from '@/features/file-actions'

import FileRow from './FileRow.vue'

export interface FileTableRow {
  key: string
  kind: FileKind
  id: string
  title: string
  editedAt: string
}

const { rows, selected, allSelected } = defineProps<{
  rows: FileTableRow[]
  selected: ReadonlySet<string>
  allSelected: boolean
}>()

const emit = defineEmits<{
  toggle: [key: string]
  toggleAll: []
  rename: [row: FileTableRow]
  share: [row: FileTableRow]
  history: [row: FileTableRow]
  download: [row: FileTableRow]
  delete: [row: FileTableRow]
}>()

const { t } = useI18n()
</script>

<template>
  <div>
    <div
      class="grid grid-cols-[auto_1fr] items-center gap-x-3 border-b border-text px-3 pb-2 text-sm font-medium sm:grid-cols-[auto_1fr_7rem_8rem_2.25rem]"
    >
      <input
        type="checkbox"
        class="size-4 accent-[var(--color-accent)]"
        :checked="allSelected"
        :aria-label="t('files.selectAll')"
        @change="emit('toggleAll')"
      />
      <span>{{ t('files.columns.name') }}</span>
      <span class="hidden sm:block">{{ t('files.columns.type') }}</span>
      <span class="hidden sm:block">{{ t('files.columns.edited') }}</span>
      <span class="hidden sm:block"></span>
    </div>
    <ul>
      <FileRow
        v-for="row in rows"
        :id="row.id"
        :key="row.key"
        :kind="row.kind"
        :title="row.title"
        :edited-at="row.editedAt"
        :selected="selected.has(row.key)"
        @toggle="emit('toggle', row.key)"
        @rename="emit('rename', row)"
        @share="emit('share', row)"
        @history="emit('history', row)"
        @download="emit('download', row)"
        @delete="emit('delete', row)"
      />
    </ul>
  </div>
</template>
