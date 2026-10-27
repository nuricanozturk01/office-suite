<script setup lang="ts">
import {
  AlignCenter,
  AlignLeft,
  AlignRight,
  ArrowRight,
  Bold,
  BringToFront,
  Circle,
  Copy,
  Italic,
  Minus,
  Plus,
  Redo2,
  SendToBack,
  Square,
  SquareRoundCorner,
  Triangle,
  Trash2,
  Type,
  Underline,
  Undo2,
} from '@lucide/vue'
import { type Component, computed } from 'vue'
import { useI18n } from 'vue-i18n'

import { DEFAULT_FILL, DEFAULT_FONT_SIZE, DEFAULT_TEXT_COLOR, type ShapeKind } from '../core/deck'
import { useDeck } from './use-deck-model'

const { t } = useI18n()
const deck = useDeck()

const inserts: readonly { kind: 'text' | ShapeKind; icon: Component; label: string }[] = [
  { kind: 'text', icon: Type, label: 'slides.insert.text' },
  { kind: 'rect', icon: Square, label: 'slides.insert.rect' },
  { kind: 'roundRect', icon: SquareRoundCorner, label: 'slides.insert.roundRect' },
  { kind: 'ellipse', icon: Circle, label: 'slides.insert.ellipse' },
  { kind: 'triangle', icon: Triangle, label: 'slides.insert.triangle' },
  { kind: 'line', icon: Minus, label: 'slides.insert.line' },
  { kind: 'arrow', icon: ArrowRight, label: 'slides.insert.arrow' },
]

const selected = computed(() => deck.selected.value)
const hasText = computed(() => selected.value !== undefined)
const button =
  'inline-flex size-8 items-center justify-center rounded hover:bg-surface-muted disabled:opacity-40 aria-pressed:bg-surface-muted'

function patch(change: Parameters<typeof deck.updateElement>[1]): void {
  if (selected.value !== undefined) {
    deck.updateElement(selected.value.id, change)
  }
}

function toggle(key: 'bold' | 'italic' | 'underline'): void {
  const current = selected.value?.[key] === true
  patch({ [key]: current ? undefined : true })
}

function onSize(event: Event): void {
  const size = Number((event.target as HTMLInputElement).value)
  if (Number.isFinite(size) && size >= 4 && size <= 400) {
    patch({ fontSize: size })
  }
}

function onColor(event: Event, key: 'color' | 'fill' | 'stroke'): void {
  patch({ [key]: (event.target as HTMLInputElement).value })
}

function onBackground(event: Event): void {
  deck.setBackground((event.target as HTMLInputElement).value)
}
</script>

<template>
  <div role="toolbar" :aria-label="t('slides.toolbar')" class="flex flex-wrap items-center gap-1">
    <button
      type="button"
      :class="button"
      :disabled="!deck.canUndo.value"
      :title="t('slides.undo')"
      :aria-label="t('slides.undo')"
      @click="deck.undo()"
    >
      <Undo2 class="size-4" aria-hidden="true" />
    </button>
    <button
      type="button"
      :class="button"
      :disabled="!deck.canRedo.value"
      :title="t('slides.redo')"
      :aria-label="t('slides.redo')"
      @click="deck.redo()"
    >
      <Redo2 class="size-4" aria-hidden="true" />
    </button>
    <span class="mx-1 h-5 w-px bg-border" aria-hidden="true" />
    <button
      type="button"
      :class="button"
      :title="t('slides.addSlide')"
      :aria-label="t('slides.addSlide')"
      @click="deck.addSlide()"
    >
      <Plus class="size-4" aria-hidden="true" />
    </button>
    <button
      v-for="item in inserts"
      :key="item.kind"
      type="button"
      :class="button"
      :title="t(item.label)"
      :aria-label="t(item.label)"
      @click="deck.addElement(item.kind)"
    >
      <component :is="item.icon" class="size-4" aria-hidden="true" />
    </button>
    <span class="mx-1 h-5 w-px bg-border" aria-hidden="true" />
    <label class="flex items-center gap-1 text-xs">
      {{ t('slides.fontSize') }}
      <input
        type="number"
        min="4"
        max="400"
        class="w-14 rounded border border-border bg-surface px-1 py-0.5"
        :disabled="!hasText"
        :value="selected?.fontSize ?? DEFAULT_FONT_SIZE"
        @change="onSize"
      />
    </label>
    <button
      type="button"
      :class="button"
      :aria-pressed="selected?.bold === true"
      :disabled="!hasText"
      :title="t('slides.bold')"
      :aria-label="t('slides.bold')"
      @click="toggle('bold')"
    >
      <Bold class="size-4" aria-hidden="true" />
    </button>
    <button
      type="button"
      :class="button"
      :aria-pressed="selected?.italic === true"
      :disabled="!hasText"
      :title="t('slides.italic')"
      :aria-label="t('slides.italic')"
      @click="toggle('italic')"
    >
      <Italic class="size-4" aria-hidden="true" />
    </button>
    <button
      type="button"
      :class="button"
      :aria-pressed="selected?.underline === true"
      :disabled="!hasText"
      :title="t('slides.underline')"
      :aria-label="t('slides.underline')"
      @click="toggle('underline')"
    >
      <Underline class="size-4" aria-hidden="true" />
    </button>
    <button
      type="button"
      :class="button"
      :disabled="!hasText"
      :title="t('slides.alignLeft')"
      :aria-label="t('slides.alignLeft')"
      @click="patch({ align: 'left' })"
    >
      <AlignLeft class="size-4" aria-hidden="true" />
    </button>
    <button
      type="button"
      :class="button"
      :disabled="!hasText"
      :title="t('slides.alignCenter')"
      :aria-label="t('slides.alignCenter')"
      @click="patch({ align: 'center' })"
    >
      <AlignCenter class="size-4" aria-hidden="true" />
    </button>
    <button
      type="button"
      :class="button"
      :disabled="!hasText"
      :title="t('slides.alignRight')"
      :aria-label="t('slides.alignRight')"
      @click="patch({ align: 'right' })"
    >
      <AlignRight class="size-4" aria-hidden="true" />
    </button>
    <label class="flex items-center gap-1 text-xs" :title="t('slides.textColor')"
      ><span class="sr-only">{{ t('slides.textColor') }}</span
      ><input
        class="size-7 cursor-pointer rounded border border-border bg-transparent p-0.5 disabled:opacity-40"
        type="color"
        :disabled="!hasText"
        :value="selected?.color ?? DEFAULT_TEXT_COLOR"
        @input="onColor($event, 'color')"
      />
    </label>
    <label class="flex items-center gap-1 text-xs" :title="t('slides.fillColor')"
      ><span class="sr-only">{{ t('slides.fillColor') }}</span
      ><input
        class="size-7 cursor-pointer rounded border border-border bg-transparent p-0.5 disabled:opacity-40"
        type="color"
        :disabled="!hasText"
        :value="selected?.fill ?? DEFAULT_FILL"
        @input="onColor($event, 'fill')"
      />
    </label>
    <label class="flex items-center gap-1 text-xs" :title="t('slides.lineColor')"
      ><span class="sr-only">{{ t('slides.lineColor') }}</span
      ><input
        class="size-7 cursor-pointer rounded border border-border bg-transparent p-0.5 disabled:opacity-40"
        type="color"
        :disabled="!hasText"
        :value="selected?.stroke ?? '#202124'"
        @input="onColor($event, 'stroke')"
      />
    </label>
    <label class="flex items-center gap-1 text-xs" :title="t('slides.background')"
      ><span class="sr-only">{{ t('slides.background') }}</span
      ><input
        class="size-7 cursor-pointer rounded border border-border bg-transparent p-0.5 disabled:opacity-40"
        type="color"
        :value="deck.slide.value.background ?? '#ffffff'"
        @input="onBackground"
      />
    </label>
    <span class="mx-1 h-5 w-px bg-border" aria-hidden="true" />
    <button
      type="button"
      :class="button"
      :disabled="!hasText"
      :title="t('slides.duplicate')"
      :aria-label="t('slides.duplicate')"
      @click="deck.duplicateSelected()"
    >
      <Copy class="size-4" aria-hidden="true" />
    </button>
    <button
      type="button"
      :class="button"
      :disabled="!hasText"
      :title="t('slides.bringToFront')"
      :aria-label="t('slides.bringToFront')"
      @click="deck.bringToFront()"
    >
      <BringToFront class="size-4" aria-hidden="true" />
    </button>
    <button
      type="button"
      :class="button"
      :disabled="!hasText"
      :title="t('slides.sendToBack')"
      :aria-label="t('slides.sendToBack')"
      @click="deck.sendToBack()"
    >
      <SendToBack class="size-4" aria-hidden="true" />
    </button>
    <button
      type="button"
      :class="button"
      :disabled="!hasText"
      :title="t('slides.delete')"
      :aria-label="t('slides.delete')"
      @click="deck.deleteSelected()"
    >
      <Trash2 class="size-4" aria-hidden="true" />
    </button>
  </div>
</template>
