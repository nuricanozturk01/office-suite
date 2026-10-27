<script setup lang="ts">
import { Minus, Plus } from '@lucide/vue'
import type { Editor } from '@tiptap/core'
import { computed, onBeforeUnmount, ref } from 'vue'
import { useI18n } from 'vue-i18n'

import { ZOOM_LEVELS } from '../../core/formatting'
import { useDocsView } from '../docs-view'

const { editor, pages, currentPage } = defineProps<{
  editor: Editor
  pages: number
  currentPage: number
}>()

const { t, locale } = useI18n()
const { zoom } = useDocsView()

const MIN_ZOOM = ZOOM_LEVELS[0]
const MAX_ZOOM = ZOOM_LEVELS[ZOOM_LEVELS.length - 1] ?? 200

interface CountStorage {
  words: () => number
}
const edits = ref(0)
const bump = (): void => {
  edits.value += 1
}
editor.on('update', bump)
onBeforeUnmount(() => {
  editor.off('update', bump)
})
const words = computed(() => {
  if (edits.value < 0) {
    return 0
  }
  const storage = editor.storage.characterCount as CountStorage | undefined // set by the CharacterCount extension
  return storage?.words() ?? 0
})
const language = computed(
  () =>
    new Intl.DisplayNames([locale.value], { type: 'language' }).of(locale.value) ?? locale.value,
)

function setZoom(event: Event): void {
  zoom.value = Number((event.target as HTMLInputElement).value)
}

function nudge(delta: number): void {
  zoom.value = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, zoom.value + delta))
}
</script>

<template>
  <footer
    class="docs-chrome flex items-center gap-4 border-t border-border bg-surface px-3 py-1 text-xs text-text-muted"
  >
    <span>{{ t('ribbon.pageOf', { page: currentPage, pages }) }}</span>
    <span>{{ t('ribbon.words', { count: words }) }}</span>
    <span class="hidden sm:inline">{{ language }}</span>
    <span class="flex-1" />
    <button
      type="button"
      class="rounded p-0.5 hover:bg-surface-muted"
      :aria-label="t('ribbon.zoomOut')"
      @click="nudge(-10)"
    >
      <Minus class="size-3.5" aria-hidden="true" />
    </button>
    <input
      type="range"
      class="w-32 accent-accent"
      :min="MIN_ZOOM"
      :max="MAX_ZOOM"
      step="5"
      :value="zoom"
      :aria-label="t('editor.actions.zoom')"
      @input="setZoom"
    />
    <button
      type="button"
      class="rounded p-0.5 hover:bg-surface-muted"
      :aria-label="t('ribbon.zoomIn')"
      @click="nudge(10)"
    >
      <Plus class="size-3.5" aria-hidden="true" />
    </button>
    <span class="w-10 text-end tabular-nums">{{ zoom }}%</span>
  </footer>
</template>
