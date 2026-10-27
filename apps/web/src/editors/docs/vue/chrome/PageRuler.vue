<script setup lang="ts">
import { computed } from 'vue'

import { MM_TO_PX, type PageGeometry } from '../../core/page-layout'

const { geometry } = defineProps<{ geometry: PageGeometry }>()

const widthMm = computed(() => geometry.width / MM_TO_PX)
const ticks = computed(() => {
  const list: { x: number; long: boolean; label: string | null }[] = []
  for (let mm = 0; mm <= widthMm.value; mm += 5) {
    const cm = mm / 10
    list.push({
      x: mm * MM_TO_PX,
      long: mm % 10 === 0,
      label: mm % 10 === 0 && mm > 0 ? String(cm) : null,
    })
  }
  return list
})
</script>

<template>
  <div
    class="docs-chrome relative mx-auto h-5 select-none"
    :style="{ width: `${String(geometry.width)}px` }"
    aria-hidden="true"
  >
    <svg
      class="absolute inset-0 size-full text-text-muted"
      :viewBox="`0 0 ${String(geometry.width)} 20`"
    >
      <rect :width="geometry.width" height="20" class="fill-surface" />
      <rect :width="geometry.marginLeft" height="20" class="fill-border" />
      <rect
        :x="geometry.width - geometry.marginRight"
        :width="geometry.marginRight"
        height="20"
        class="fill-border"
      />
      <g v-for="tick in ticks" :key="tick.x">
        <line
          :x1="tick.x"
          :x2="tick.x"
          :y1="tick.long ? 11 : 14"
          y2="20"
          stroke="currentColor"
          stroke-width="1"
        />
        <text v-if="tick.label" :x="tick.x + 3" y="10" font-size="9" fill="currentColor">
          {{ tick.label }}
        </text>
      </g>
    </svg>
  </div>
</template>
