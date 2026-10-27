<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, useTemplateRef } from 'vue'
import { useI18n } from 'vue-i18n'

import { type DeckElement } from '../core/deck'
import SlideView from './SlideView.vue'
import { useDeck } from './use-deck-model'

const { t } = useI18n()
const deck = useDeck()
const svg = useTemplateRef<SVGSVGElement>('svg')
const editingId = ref<string | null>(null)
const vFocus = {
  mounted: (el: HTMLElement) => {
    el.focus()
  },
}

type Handle = 'move' | 'nw' | 'ne' | 'sw' | 'se'
interface Drag {
  handle: Handle
  id: string
  startX: number
  startY: number
  origin: DeckElement
  moved: boolean
}
let drag: Drag | null = null

const MIN_SIZE = 8
const HANDLES: readonly { handle: Handle; fx: number; fy: number }[] = [
  { handle: 'nw', fx: 0, fy: 0 },
  { handle: 'ne', fx: 1, fy: 0 },
  { handle: 'sw', fx: 0, fy: 1 },
  { handle: 'se', fx: 1, fy: 1 },
]

function toSlide(event: PointerEvent): { x: number; y: number } {
  const matrix = svg.value?.getScreenCTM()
  if (!matrix) {
    return { x: 0, y: 0 }
  }
  const inverse = matrix.inverse()
  return {
    x: inverse.a * event.clientX + inverse.c * event.clientY + inverse.e,
    y: inverse.b * event.clientX + inverse.d * event.clientY + inverse.f,
  }
}

function begin(event: PointerEvent, element: DeckElement, handle: Handle): void {
  if (editingId.value === element.id) {
    return
  }
  deck.selectedId.value = element.id
  const point = toSlide(event)
  drag = { handle, id: element.id, startX: point.x, startY: point.y, origin: element, moved: false }
  ;(event.currentTarget as Element).setPointerCapture(event.pointerId) // keep receiving moves
}

function onMove(event: PointerEvent): void {
  if (drag === null) {
    return
  }
  const point = toSlide(event)
  const dx = point.x - drag.startX
  const dy = point.y - drag.startY
  const o = drag.origin
  if (!drag.moved) {
    if (Math.abs(dx) + Math.abs(dy) < 2) {
      return
    }
    deck.snapshot()
    drag.moved = true
  }
  const isLine = o.shape === 'line' || o.shape === 'arrow'
  let patch: Partial<DeckElement>
  switch (drag.handle) {
    case 'move':
      patch = { x: o.x + dx, y: o.y + dy }
      break
    case 'se':
      patch = {
        w: Math.max(isLine ? 0 : MIN_SIZE, o.w + dx),
        h: isLine ? o.h + dy : Math.max(MIN_SIZE, o.h + dy),
      }
      break
    case 'ne':
      patch = { y: o.y + dy, w: Math.max(MIN_SIZE, o.w + dx), h: Math.max(MIN_SIZE, o.h - dy) }
      break
    case 'sw':
      patch = { x: o.x + dx, w: Math.max(MIN_SIZE, o.w - dx), h: Math.max(MIN_SIZE, o.h + dy) }
      break
    case 'nw':
      patch = {
        x: o.x + dx,
        y: o.y + dy,
        w: Math.max(MIN_SIZE, o.w - dx),
        h: Math.max(MIN_SIZE, o.h - dy),
      }
      break
  }
  deck.updateElement(drag.id, patch, false)
}

function end(): void {
  drag = null
}

function edit(element: DeckElement): void {
  deck.selectedId.value = element.id
  editingId.value = element.id
}

function onInput(event: Event, element: DeckElement): void {
  deck.updateElement(element.id, { text: (event.target as HTMLTextAreaElement).value }, false)
}

function onKey(event: KeyboardEvent): void {
  if (document.activeElement !== svg.value && editingId.value === null) {
    return
  }
  if (editingId.value !== null) {
    if (event.key === 'Escape') {
      editingId.value = null
    }
    return
  }
  const selected = deck.selected.value
  if (selected === undefined) {
    return
  }
  const step = event.shiftKey ? 10 : 1
  const moves: Record<string, Partial<DeckElement>> = {
    ArrowLeft: { x: selected.x - step },
    ArrowRight: { x: selected.x + step },
    ArrowUp: { y: selected.y - step },
    ArrowDown: { y: selected.y + step },
  }
  const patch = moves[event.key]
  if (patch !== undefined) {
    deck.updateElement(selected.id, patch)
  } else if (event.key === 'Delete' || event.key === 'Backspace') {
    deck.deleteSelected()
  } else if (event.key === 'Enter') {
    editingId.value = selected.id
  } else {
    return
  }
  event.preventDefault()
}

function onBackground(event: PointerEvent): void {
  if (event.target === svg.value) {
    deck.selectedId.value = null
  }
}

onMounted(() => {
  window.addEventListener('keydown', onKey)
  window.addEventListener('pointermove', onMove)
  window.addEventListener('pointerup', end)
  window.addEventListener('pointerdown', onBackground)
})
onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKey)
  window.removeEventListener('pointermove', onMove)
  window.removeEventListener('pointerup', end)
  window.removeEventListener('pointerdown', onBackground)
})

function label(element: DeckElement): string {
  return element.kind === 'text' ? t('slides.textBox') : t('slides.shape')
}
</script>

<template>
  <svg
    ref="svg"
    class="h-auto max-h-full w-full max-w-5xl bg-white shadow-md outline-none focus-visible:ring-2 focus-visible:ring-focus"
    :viewBox="`0 0 ${String(deck.deck.value.width)} ${String(deck.deck.value.height)}`"
    role="application"
    tabindex="0"
    :aria-label="t('slides.canvas')"
  >
    <SlideView
      :slide="deck.slide.value"
      :width="deck.deck.value.width"
      :height="deck.deck.value.height"
      marker-id="canvas-arrow"
    />
    <g v-for="element in deck.slide.value.elements" :key="element.id">
      <rect
        :x="element.x"
        :y="element.y"
        :width="Math.max(element.w, 12)"
        :height="Math.max(element.h, 12)"
        fill="transparent"
        class="cursor-move"
        role="button"
        :aria-label="label(element)"
        @pointerdown.stop="begin($event, element, 'move')"
        @dblclick="edit(element)"
      />
      <g
        v-if="element.kind === 'text' && (element.text ?? '') === '' && editingId !== element.id"
        pointer-events="none"
      >
        <rect
          :x="element.x"
          :y="element.y"
          :width="element.w"
          :height="element.h"
          fill="none"
          stroke="#9aa0a6"
          stroke-width="1.5"
          stroke-dasharray="6 5"
        />
        <text
          :x="element.x + element.w / 2"
          :y="element.y + element.h / 2"
          text-anchor="middle"
          dominant-baseline="middle"
          fill="#9aa0a6"
          font-size="20"
        >
          {{ t('slides.textHint') }}
        </text>
      </g>
      <template v-if="deck.selectedId.value === element.id">
        <rect
          :x="element.x"
          :y="element.y"
          :width="element.w"
          :height="element.h"
          fill="none"
          stroke="var(--color-focus)"
          stroke-width="2"
          pointer-events="none"
        />
        <rect
          v-for="h in HANDLES"
          :key="h.handle"
          :x="element.x + element.w * h.fx - 6"
          :y="element.y + element.h * h.fy - 6"
          width="12"
          height="12"
          fill="white"
          stroke="var(--color-focus)"
          stroke-width="2"
          class="cursor-nwse-resize"
          role="button"
          :aria-label="t('slides.resize')"
          @pointerdown.stop="begin($event, element, h.handle)"
        />
      </template>
      <foreignObject
        v-if="editingId === element.id"
        :x="element.x"
        :y="element.y"
        :width="Math.max(element.w, 40)"
        :height="Math.max(element.h, 40)"
      >
        <textarea
          v-focus
          class="size-full resize-none border-0 bg-white/90 p-2 outline-2 outline-focus"
          :style="{ fontSize: `${String(element.fontSize ?? 24)}px`, color: element.color }"
          :value="element.text ?? ''"
          :aria-label="label(element)"
          @input="onInput($event, element)"
          @blur="editingId = null"
        />
      </foreignObject>
    </g>
  </svg>
</template>
