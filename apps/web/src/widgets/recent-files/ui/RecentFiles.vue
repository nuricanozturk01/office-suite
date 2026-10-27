<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'

import {
  isNothingToDownload,
  RenameFileDialog,
  useBulkActions,
  useDownloadFile,
} from '@/features/file-actions'
import { ShareDialog } from '@/features/share-file'
import { VersionHistoryDialog } from '@/features/version-history'
import { errorMessageKey } from '@/shared/api'
import { AppButton, EmptyState, InlineAlert } from '@/shared/ui'

import { type LibraryFile, useAllFiles } from '../model/use-all-files'
import { useSelection } from '../model/use-selection'

import BulkBar from './BulkBar.vue'
import FileTable, { type FileTableRow } from './FileTable.vue'

/** Filters the loaded pages by title; server-side search arrives with the search module. */
const {
  filter = '',
  kind = 'all',
  sort = 'updated',
} = defineProps<{
  filter?: string
  kind?: 'all' | LibraryFile['kind']
  sort?: 'updated' | 'name'
}>()

const { t, locale } = useI18n()
const { files, error, failed, ready, hasMore, loadingMore, loadMore } = useAllFiles()
const bulk = useBulkActions()

const dateFormat = computed(() => new Intl.DateTimeFormat(locale.value, { dateStyle: 'medium' }))
const needle = computed(() => filter.trim().toLocaleLowerCase(locale.value))
const visible = computed(() => {
  const matching = files.value.filter(
    (file) =>
      (kind === 'all' || file.kind === kind) &&
      titleOf(file).toLocaleLowerCase(locale.value).includes(needle.value),
  )
  return sort === 'name'
    ? [...matching].sort((a, b) => titleOf(a).localeCompare(titleOf(b), locale.value))
    : matching
})
const renameTarget = ref<LibraryFile>()
const renameOpen = computed({
  get: () => renameTarget.value !== undefined,
  set: (open: boolean) => {
    if (!open) {
      renameTarget.value = undefined
    }
  },
})
const download = useDownloadFile()

const shareTarget = ref<LibraryFile>()
const shareOpen = computed({
  get: () => shareTarget.value !== undefined,
  set: (open: boolean) => {
    if (!open) {
      shareTarget.value = undefined
    }
  },
})

const historyTarget = ref<LibraryFile>()
const historyOpen = computed({
  get: () => historyTarget.value !== undefined,
  set: (open: boolean) => {
    if (!open) {
      historyTarget.value = undefined
    }
  },
})

const UNTITLED = {
  document: 'documents.untitled',
  spreadsheet: 'sheets.untitled',
  presentation: 'slides.untitled',
  pdf: 'pdf.untitled',
} as const

function titleOf(file: LibraryFile): string {
  return file.item.title || t(UNTITLED[file.kind])
}

const rows = computed<FileTableRow[]>(() =>
  visible.value.map((file) => ({
    key: `${file.kind}:${file.item.id}`,
    kind: file.kind,
    id: file.item.id,
    title: titleOf(file),
    editedAt: dateFormat.value.format(new Date(file.updatedAt)),
  })),
)
const selection = useSelection(() => rows.value.map((row) => row.key))
const chosen = computed(() => rows.value.filter((row) => selection.selected.value.has(row.key)))

function fileOf(row: FileTableRow): LibraryFile | undefined {
  return files.value.find((file) => file.kind === row.kind && file.item.id === row.id)
}

function openRename(row: FileTableRow): void {
  renameTarget.value = fileOf(row)
}

function openShare(row: FileTableRow): void {
  shareTarget.value = fileOf(row)
}

function openHistory(row: FileTableRow): void {
  historyTarget.value = fileOf(row)
}

async function removeRows(targets: FileTableRow[]): Promise<void> {
  const count = targets.length
  const message =
    count === 1
      ? t('files.deleteConfirm', { title: targets[0]?.title ?? '' })
      : t('files.bulk.deleteConfirm', { count })
  if (count === 0 || !window.confirm(message)) {
    return
  }
  await bulk.removeAll(targets)
  selection.clear()
}

async function downloadRows(targets: FileTableRow[]): Promise<void> {
  await bulk.downloadAll(targets)
}
</script>

<template>
  <div class="flex flex-col gap-4">
    <InlineAlert v-if="failed">
      {{ t(errorMessageKey(error)) }}
    </InlineAlert>
    <EmptyState
      v-if="ready && files.length === 0"
      :heading="t('files.empty')"
      :description="t('files.emptyHint')"
    />
    <EmptyState
      v-else-if="files.length > 0 && visible.length === 0"
      :heading="t('files.noMatch')"
    />
    <InlineAlert v-if="download.isError.value">
      {{
        isNothingToDownload(download.error.value)
          ? t('fileActions.downloadEmpty')
          : t(errorMessageKey(download.error.value))
      }}
    </InlineAlert>
    <BulkBar
      v-if="selection.count.value > 0"
      :count="selection.count.value"
      :busy="bulk.busy.value"
      :failed="bulk.failed.value"
      @download="downloadRows(chosen)"
      @delete="removeRows(chosen)"
      @clear="selection.clear()"
    />
    <FileTable
      v-if="rows.length > 0"
      :rows="rows"
      :selected="selection.selected.value"
      :all-selected="selection.allSelected.value"
      @toggle="selection.toggle"
      @toggle-all="selection.toggleAll()"
      @rename="openRename"
      @share="openShare"
      @history="openHistory"
      @download="download.mutate($event.id)"
      @delete="removeRows([$event])"
    />
    <RenameFileDialog
      v-if="renameTarget"
      :id="renameTarget.item.id"
      v-model:open="renameOpen"
      :kind="renameTarget.kind"
      :title="titleOf(renameTarget)"
    />
    <ShareDialog
      v-if="shareTarget"
      v-model:open="shareOpen"
      :file-id="shareTarget.item.id"
      :title="titleOf(shareTarget)"
    />
    <VersionHistoryDialog
      v-if="historyTarget"
      v-model:open="historyOpen"
      :resource-id="historyTarget.item.id"
      :title="titleOf(historyTarget)"
    />
    <AppButton
      v-if="hasMore"
      variant="ghost"
      :loading="loadingMore"
      class="self-center"
      @click="loadMore"
    >
      {{ t('files.loadMore') }}
    </AppButton>
  </div>
</template>
