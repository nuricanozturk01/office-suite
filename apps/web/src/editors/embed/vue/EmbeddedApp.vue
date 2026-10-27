<script setup lang="ts">
import { onBeforeUnmount, onMounted, useTemplateRef } from 'vue'
import { useI18n } from 'vue-i18n'

import { browserDownloader } from '../adapters/browser-downloader'
import { createCloudStore } from '../adapters/cloud-store'
import { PostMessageEngine } from '../adapters/postmessage-engine'
import { EMBEDDED_APPS, type EmbeddedAppId, isStoredFormat } from '../core/apps'
import type { SaveStatus } from '../core/ports'
import { EditorSession } from '../core/session'

/**
 * Hosts an editor library (built by `scripts/build-embeds.sh`) in an iframe. Only wires the
 * adapters to an {@link EditorSession}; the behaviour lives in `core/session.ts`.
 */
const { app, resourceId, title, author } = defineProps<{
  app: EmbeddedAppId
  resourceId: string
  /** Names the starting file of a resource that has none yet. */
  title: string
  author: string
}>()

const emit = defineEmits<{
  status: [status: SaveStatus]
  command: [name: 'new' | 'open' | 'close' | 'share']
  /** A save stored `version`. */
  saved: [version: number]
  /** A save was refused: the file changed elsewhere. */
  saveConflict: []
  merged: []
  preview: [version: number | null]
  /** The user renamed the document inside the editor. */
  renamed: [name: string]
}>()

const { t } = useI18n()
const frame = useTemplateRef<HTMLIFrameElement>('frame')
const config = EMBEDDED_APPS[app]
const src = `/apps/${app}/index.html?host=parent&author=${encodeURIComponent(author)}`
const AUTOSAVE_MS = 2500
let session: EditorSession | undefined

onMounted(() => {
  session = new EditorSession({
    engine: new PostMessageEngine({
      frame: () => frame.value,
      prefix: config.prefix,
      canvasId: config.canvasId,
      origin: window.location.origin,
      target: window,
    }),
    store: createCloudStore(app, resourceId, title),
    downloader: browserDownloader,
    isDocumentFormat: (name) => isStoredFormat(config, name),
    onStatus: (status) => {
      emit('status', status)
    },
    onCommand: (command) => {
      emit('command', command)
    },
    autosaveMs: AUTOSAVE_MS,
    onSaved: (version) => {
      emit('saved', version)
    },
    onPreview: (version) => {
      emit('preview', version)
    },
    onMerged: () => {
      emit('merged')
    },
    onSaveConflict: () => {
      emit('saveConflict')
    },
    onRenamed: (name) => {
      emit('renamed', name)
    },
  })
  window.addEventListener('beforeunload', onBeforeUnload)
})

onBeforeUnmount(() => {
  window.removeEventListener('beforeunload', onBeforeUnload)
  session?.dispose()
})

function onBeforeUnload(event: BeforeUnloadEvent): void {
  if (session?.hasUnsavedChanges === true) {
    event.preventDefault()
  }
}

defineExpose({
  saveToCloud: () => {
    session?.saveToCloud()
  },
  downloadCopy: () => {
    session?.downloadCopy()
  },
  /** The editing session, for things that keep it in step with other people. */
  session: (): EditorSession | undefined => session,
})
</script>

<template>
  <div class="relative min-h-0 flex-1">
    <iframe
      ref="frame"
      :src="src"
      :title="t(`embed.${app}`)"
      class="absolute inset-0 size-full border-0"
      allow="clipboard-read; clipboard-write; fullscreen"
    />
  </div>
</template>
