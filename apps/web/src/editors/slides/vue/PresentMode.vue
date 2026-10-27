<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, useTemplateRef } from 'vue'
import { useI18n } from 'vue-i18n'

import type { DeckData } from '../core/deck'
import SlideView from './SlideView.vue'

const { deck, start } = defineProps<{ deck: DeckData; start: number }>()
const emit = defineEmits<{ close: [] }>()

const { t } = useI18n()
const index = ref(start)
const current = computed(() => deck.slides[index.value])
const root = useTemplateRef<HTMLDivElement>('root')

function go(delta: number): void {
  index.value = Math.min(deck.slides.length - 1, Math.max(0, index.value + delta))
}

function onKey(event: KeyboardEvent): void {
  if (event.key === 'Escape') {
    emit('close')
  } else if (['ArrowRight', 'ArrowDown', 'PageDown', ' ', 'Enter'].includes(event.key)) {
    go(1)
  } else if (['ArrowLeft', 'ArrowUp', 'PageUp', 'Backspace'].includes(event.key)) {
    go(-1)
  } else {
    return
  }
  event.preventDefault()
}

function onFullscreenChange(): void {
  if (!document.fullscreenElement) {
    emit('close')
  }
}

onMounted(() => {
  document.addEventListener('fullscreenchange', onFullscreenChange)
  window.addEventListener('keydown', onKey)
  void root.value?.requestFullscreen().catch(() => undefined) // fullscreen is optional
  root.value?.focus()
})
onBeforeUnmount(() => {
  document.removeEventListener('fullscreenchange', onFullscreenChange)
  window.removeEventListener('keydown', onKey)
  if (document.fullscreenElement) {
    void document.exitFullscreen()
  }
})
</script>

<template>
  <div
    ref="root"
    class="fixed inset-0 z-50 flex items-center justify-center bg-black outline-none"
    role="dialog"
    aria-modal="true"
    :aria-label="t('slides.present')"
    tabindex="-1"
  >
    <svg
      class="h-full w-full"
      :viewBox="`0 0 ${String(deck.width)} ${String(deck.height)}`"
      role="img"
      :aria-label="t('slides.goTo', { n: index + 1 })"
    >
      <SlideView
        v-if="current"
        :slide="current"
        :width="deck.width"
        :height="deck.height"
        marker-id="present-arrow"
      />
    </svg>
    <div class="absolute start-1/2 bottom-4 flex -translate-x-1/2 gap-2">
      <button
        type="button"
        class="rounded bg-white/20 px-3 py-1 text-sm text-white hover:bg-white/30"
        :disabled="index === 0"
        @click.stop="go(-1)"
      >
        {{ t('slides.previous') }}
      </button>
      <button
        type="button"
        class="rounded bg-white/20 px-3 py-1 text-sm text-white hover:bg-white/30"
        :disabled="index === deck.slides.length - 1"
        @click.stop="go(1)"
      >
        {{ t('slides.next') }}
      </button>
    </div>
    <button
      type="button"
      class="absolute end-4 top-4 rounded bg-white/20 px-3 py-1 text-sm text-white hover:bg-white/30"
      @click.stop="emit('close')"
    >
      {{ t('slides.exitPresent') }}
    </button>
  </div>
</template>
