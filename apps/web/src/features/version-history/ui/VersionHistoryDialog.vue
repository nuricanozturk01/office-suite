<script setup lang="ts">
import { Download, Eye, RotateCcw } from '@lucide/vue'
import { useI18n } from 'vue-i18n'

import { type PackageVersion, usePackageVersionsQuery } from '@/entities/package'
import { useSessionStore } from '@/entities/session'
import { errorMessageKey } from '@/shared/api'
import { AppButton, AppDialog, InlineAlert } from '@/shared/ui'

import { useVersionActions } from '../model/use-version-actions'

const { resourceId, title, previewable } = defineProps<{
  resourceId: string
  title: string
  /** The caller can show an older version in place (an open editor); a file list cannot. */
  previewable?: boolean
}>()

const emit = defineEmits<{
  restored: []
  view: [version: number, savedAt: Date, canRestore: boolean]
}>()
const open = defineModel<boolean>('open', { required: true })

const { t, d, n } = useI18n()
const session = useSessionStore()
const versions = usePackageVersionsQuery(
  () => resourceId,
  () => open.value,
)
const { download, restore } = useVersionActions(
  () => resourceId,
  () => {
    emit('restored')
    open.value = false
  },
)

function view(version: number, savedAt: Date): void {
  emit('view', version, savedAt, versions.data.value?.canRestore ?? false)
  open.value = false
}

function savedBy(entry: PackageVersion): string {
  if (entry.savedBy === session.user?.id) {
    return t('history.you')
  }
  return entry.savedByName ?? t('history.someone')
}

function sizeLabel(bytes: number): string {
  return bytes < 1024 * 1024
    ? t('history.kb', { size: n(Math.max(1, Math.round(bytes / 1024))) })
    : t('history.mb', { size: n(Math.round(bytes / (1024 * 1024)), { maximumFractionDigits: 1 }) })
}
</script>

<template>
  <AppDialog
    v-model:open="open"
    :title="t('history.title', { title })"
    :description="t('history.description')"
    :close-label="t('history.close')"
  >
    <InlineAlert v-if="versions.isError.value">
      {{ t(errorMessageKey(versions.error.value)) }}
    </InlineAlert>
    <InlineAlert v-if="restore.isError.value">
      {{ t(errorMessageKey(restore.error.value)) }}
    </InlineAlert>
    <InlineAlert v-if="download.isError.value">
      {{ t(errorMessageKey(download.error.value)) }}
    </InlineAlert>
    <p v-if="versions.isPending.value" role="status" class="text-sm text-text-muted">
      {{ t('history.loading') }}
    </p>
    <p v-else-if="versions.data.value?.versions.length === 0" class="text-sm text-text-muted">
      {{ t('history.empty') }}
    </p>
    <ul v-if="versions.data.value" class="flex flex-col divide-y divide-border">
      <li
        v-for="(entry, index) in versions.data.value.versions"
        :key="entry.version"
        class="flex items-center gap-3 py-2.5"
      >
        <div class="min-w-0 flex-1">
          <p class="truncate text-sm font-medium">
            {{ d(new Date(entry.savedAt), { dateStyle: 'medium', timeStyle: 'short' }) }}
            <span
              v-if="index === 0"
              class="ms-2 rounded-full bg-surface-muted px-2 py-0.5 text-xs font-normal text-text-muted"
            >
              {{ t('history.current') }}
            </span>
          </p>
          <p class="truncate text-xs text-text-muted">
            {{ savedBy(entry) }}
            · {{ sizeLabel(entry.size) }}
          </p>
        </div>
        <AppButton
          v-if="previewable && index > 0"
          variant="ghost"
          @click="view(entry.version, new Date(entry.savedAt))"
        >
          <Eye class="size-4" aria-hidden="true" />{{ t('history.view') }}
        </AppButton>
        <AppButton
          variant="ghost"
          :loading="download.isPending.value && download.variables.value === entry.version"
          :aria-label="t('history.download', { n: entry.version })"
          @click="download.mutate(entry.version)"
        >
          <Download class="size-4" aria-hidden="true" />
        </AppButton>
        <AppButton
          v-if="versions.data.value.canRestore && index > 0"
          variant="ghost"
          :loading="restore.isPending.value && restore.variables.value === entry.version"
          @click="restore.mutate(entry.version)"
        >
          <RotateCcw class="size-4" aria-hidden="true" />{{ t('history.restore') }}
        </AppButton>
      </li>
    </ul>
  </AppDialog>
</template>
