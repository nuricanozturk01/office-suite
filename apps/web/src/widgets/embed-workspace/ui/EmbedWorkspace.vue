<script setup lang="ts">
import { History, TriangleAlert } from '@lucide/vue'
import { computed, onMounted, ref, useTemplateRef, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'

import { useSessionStore } from '@/entities/session'
import {
  LiveSyncNotice,
  PresenceStack,
  type SyncTarget,
  useLiveSync,
  WebSocketLiveChannel,
} from '@/editors/collab'
import { EmbeddedApp, type EmbeddedAppId, type SaveStatus } from '@/editors/embed'
import { type FileKind, useRenameFile } from '@/features/file-actions'
import { ShareDialog } from '@/features/share-file'
import { VersionHistoryDialog } from '@/features/version-history'
import { apiWebSocketOrigin } from '@/shared/api/origin'
import { RouteName } from '@/shared/config/routes'

import { useVersionPreview } from '../model/use-version-preview'

import VersionPreviewBar from './VersionPreviewBar.vue'

const { app, resourceId, title, untitled } = defineProps<{
  app: EmbeddedAppId
  resourceId: string
  title: string
  /** Shown in the share dialog when the file has no title. */
  untitled: string
}>()

const emit = defineEmits<{ new: [] }>()

const { t } = useI18n()
const router = useRouter()
const session = useSessionStore()
const status = ref<SaveStatus>('loading')
const shareOpen = ref(false)
const historyOpen = ref(false)
const embedded = useTemplateRef<InstanceType<typeof EmbeddedApp>>('embedded')
const author = computed(() => session.user?.display_name ?? '')

const KINDS: Record<EmbeddedAppId, FileKind> = {
  wordcraft: 'document',
  gridcraft: 'spreadsheet',
  deckcraft: 'presentation',
  pdfcraft: 'pdf',
}
const rename = useRenameFile()

/** A rename inside the editor is the file's new name here too (the file list shows it). */
function onRenamed(name: string): void {
  if (name !== title.trim()) {
    rename.mutate({ kind: KINDS[app], id: resourceId, title: name })
  }
}

const preview = useVersionPreview(
  () => embedded.value?.session(),
  () => resourceId,
)

const live = useLiveSync(
  () =>
    new WebSocketLiveChannel({
      url: `${apiWebSocketOrigin()}/api/v1/livesync/${resourceId}`,
      token: () => session.token(),
    }),
)

onMounted(() => {
  // The editor session exists once `EmbeddedApp` has mounted (children mount first).
  const target: SyncTarget | undefined = embedded.value?.session()
  if (target !== undefined) {
    live.start(target)
  }
})

/** Only trouble is shown here; the editor itself shows "Saved" and "Edited". */
const problem = computed(() =>
  status.value === 'error' || status.value === 'failed' ? status.value : undefined,
)
const showFooter = computed(() => live.state.value.notice.kind !== 'none')

// Tell the others when this user has unsaved edits (and when they are done, a moment later).
const EDITING_LINGER_MS = 3000
let doneTimer: ReturnType<typeof setTimeout> | undefined
watch(status, (next) => {
  clearTimeout(doneTimer)
  if (next === 'dirty' || next === 'saving') {
    live.announce(true)
  } else if (next === 'saved') {
    doneTimer = setTimeout(() => {
      live.announce(false)
    }, EDITING_LINGER_MS)
  }
})

/** An older version became the current one: show it in the editor. */
function onRestored(): void {
  void embedded.value?.session()?.reload()
}

function onCommand(name: 'new' | 'open' | 'close' | 'share'): void {
  if (name === 'new') {
    emit('new')
  } else if (name === 'share') {
    shareOpen.value = true
  } else {
    void router.push({ name: RouteName.Documents })
  }
}
</script>

<template>
  <div class="flex h-dvh flex-col bg-surface text-text">
    <p
      v-if="problem"
      role="alert"
      class="flex shrink-0 items-center gap-2 bg-danger-surface px-4 py-2 text-sm text-danger"
    >
      <TriangleAlert class="size-4 shrink-0" aria-hidden="true" />
      {{ t(`embed.status.${problem}`) }}
    </p>
    <VersionPreviewBar
      v-if="preview.previewing.value !== null && preview.savedAt.value"
      :saved-at="preview.savedAt.value"
      :can-restore="preview.canRestore.value"
      :busy="preview.restore.isPending.value"
      :error="preview.restore.error.value"
      @restore="preview.restorePreviewed()"
      @back="preview.back()"
    />
    <div class="relative flex min-h-0 flex-1 flex-col">
      <EmbeddedApp
        ref="embedded"
        :app="app"
        :resource-id="resourceId"
        :title="title"
        :author="author"
        @status="status = $event"
        @command="onCommand"
        @saved="live.localSaved($event)"
        @merged="live.localMerged()"
        @preview="preview.previewing.value = $event"
        @save-conflict="live.localSaveConflicted()"
        @renamed="onRenamed"
      />
      <!-- Over the editor's own status bar, whose middle is free in every editor. -->
      <div
        class="pointer-events-none absolute inset-x-0 bottom-0.5 z-10 flex items-center justify-center gap-2"
      >
        <button
          type="button"
          class="pointer-events-auto inline-flex items-center gap-1.5 rounded-full px-2.5 py-1 text-xs text-text-muted hover:bg-surface-muted"
          @click="historyOpen = true"
        >
          <History class="size-3.5" aria-hidden="true" />{{ t('history.open') }}
        </button>
        <PresenceStack class="pointer-events-auto" :participants="live.state.value.participants" />
      </div>
    </div>
    <footer
      v-if="showFooter"
      class="flex shrink-0 flex-wrap items-center justify-between gap-x-6 gap-y-2 border-t border-border bg-surface px-4 py-1.5"
    >
      <LiveSyncNotice
        :notice="live.state.value.notice"
        :participants="live.state.value.participants"
        @take-theirs="live.takeTheirs()"
        @keep-mine="live.keepMine()"
        @dismiss="live.dismiss()"
      />
    </footer>
    <VersionHistoryDialog
      v-model:open="historyOpen"
      :resource-id="resourceId"
      :title="title || untitled"
      previewable
      @view="(version, at, mayRestore) => void preview.view(version, at, mayRestore)"
      @restored="onRestored"
    />
    <ShareDialog v-model:open="shareOpen" :file-id="resourceId" :title="title || untitled" />
  </div>
</template>
