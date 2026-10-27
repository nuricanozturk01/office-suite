<script setup lang="ts">
import { Plus } from '@lucide/vue'
import { useI18n } from 'vue-i18n'

import SlideView from './SlideView.vue'
import { useDeck } from './use-deck-model'

const { t } = useI18n()
const deck = useDeck()
</script>

<template>
  <nav
    :aria-label="t('slides.rail')"
    class="flex w-48 shrink-0 flex-col gap-2 overflow-y-auto border-e border-border bg-surface p-3"
  >
    <ol class="flex flex-col gap-2">
      <li v-for="(slide, index) in deck.deck.value.slides" :key="slide.id" class="flex gap-1">
        <span class="w-5 pt-1 text-end text-xs text-text-muted">{{ index + 1 }}</span>
        <button
          type="button"
          class="min-w-0 flex-1 rounded border-2 bg-white"
          :class="index === deck.activeIndex.value ? 'border-focus' : 'border-border'"
          :aria-current="index === deck.activeIndex.value ? 'true' : undefined"
          :aria-label="t('slides.goTo', { n: index + 1 })"
          @click="deck.chooseSlide(index)"
        >
          <svg
            class="block h-auto w-full"
            :viewBox="`0 0 ${String(deck.deck.value.width)} ${String(deck.deck.value.height)}`"
            aria-hidden="true"
          >
            <SlideView
              :slide="slide"
              :width="deck.deck.value.width"
              :height="deck.deck.value.height"
              :marker-id="`thumb-${slide.id}`"
            />
          </svg>
        </button>
      </li>
    </ol>
    <button
      type="button"
      class="mx-auto mt-1 inline-flex items-center gap-1.5 rounded-full border border-border px-3 py-1.5 text-sm hover:bg-surface-muted"
      @click="deck.addSlide()"
    >
      <Plus class="size-4" aria-hidden="true" />
      {{ t('slides.addSlide') }}
    </button>
  </nav>
</template>
