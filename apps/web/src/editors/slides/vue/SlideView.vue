<script setup lang="ts">
import { computed } from 'vue'

import {
  DEFAULT_FONT_SIZE,
  DEFAULT_TEXT_COLOR,
  type DeckElement,
  type DeckSlide,
} from '../core/deck'

const { slide, width, height, markerId } = defineProps<{
  slide: DeckSlide
  width: number
  height: number
  /** Unique per instance so several views on a page do not share one arrowhead marker. */
  markerId: string
}>()

const arrow = computed(() => `url(#${markerId})`)

function transform(element: DeckElement): string | undefined {
  if (element.rotation === undefined || element.rotation === 0) {
    return undefined
  }
  return `rotate(${String(element.rotation)} ${String(element.x + element.w / 2)} ${String(element.y + element.h / 2)})`
}

function textStyle(element: DeckElement): Record<string, string> {
  return {
    color: element.color ?? DEFAULT_TEXT_COLOR,
    fontSize: `${String(element.fontSize ?? DEFAULT_FONT_SIZE)}px`,
    fontWeight: element.bold === true ? '700' : '400',
    fontStyle: element.italic === true ? 'italic' : 'normal',
    textDecoration: element.underline === true ? 'underline' : 'none',
    textAlign: element.align ?? 'left',
  }
}

function points(element: DeckElement): string {
  const { x, y, w, h } = element
  return `${String(x + w / 2)},${String(y)} ${String(x + w)},${String(y + h)} ${String(x)},${String(y + h)}`
}
</script>

<template>
  <g>
    <defs>
      <marker
        :id="markerId"
        viewBox="0 0 10 10"
        refX="9"
        refY="5"
        markerWidth="6"
        markerHeight="6"
        orient="auto-start-reverse"
      >
        <path d="M 0 0 L 10 5 L 0 10 z" fill="context-stroke" />
      </marker>
    </defs>
    <rect :width="width" :height="height" :fill="slide.background ?? '#ffffff'" />
    <g v-for="element in slide.elements" :key="element.id" :transform="transform(element)">
      <template v-if="element.kind === 'shape'">
        <line
          v-if="element.shape === 'line' || element.shape === 'arrow'"
          :x1="element.x"
          :y1="element.y"
          :x2="element.x + element.w"
          :y2="element.y + element.h"
          :stroke="element.stroke ?? '#202124'"
          :stroke-width="element.strokeWidth ?? 3"
          :marker-end="element.shape === 'arrow' ? arrow : undefined"
        />
        <ellipse
          v-else-if="element.shape === 'ellipse'"
          :cx="element.x + element.w / 2"
          :cy="element.y + element.h / 2"
          :rx="element.w / 2"
          :ry="element.h / 2"
          :fill="element.fill ?? 'none'"
          :stroke="element.stroke ?? 'none'"
          :stroke-width="element.strokeWidth ?? 1"
        />
        <polygon
          v-else-if="element.shape === 'triangle'"
          :points="points(element)"
          :fill="element.fill ?? 'none'"
          :stroke="element.stroke ?? 'none'"
          :stroke-width="element.strokeWidth ?? 1"
        />
        <rect
          v-else
          :x="element.x"
          :y="element.y"
          :width="element.w"
          :height="element.h"
          :rx="element.shape === 'roundRect' ? Math.min(element.w, element.h) / 6 : 0"
          :fill="element.fill ?? 'none'"
          :stroke="element.stroke ?? 'none'"
          :stroke-width="element.strokeWidth ?? 1"
        />
      </template>
      <foreignObject
        v-if="element.text !== undefined && element.text !== ''"
        :x="element.x"
        :y="element.y"
        :width="element.w"
        :height="Math.max(element.h, 1)"
      >
        <div
          class="size-full overflow-hidden p-2 break-words whitespace-pre-wrap"
          :class="element.kind === 'shape' ? 'flex items-center justify-center' : ''"
          :style="textStyle(element)"
        >
          {{ element.text }}
        </div>
      </foreignObject>
    </g>
  </g>
</template>
