<script setup lang="ts">
import { Ban, ChevronDown } from '@lucide/vue'
import { PopoverClose, PopoverContent, PopoverPortal, PopoverRoot, PopoverTrigger } from 'reka-ui'
import type { Component } from 'vue'

/** A color button with a palette popover; the swatch under the icon shows the current color. */
const { icon, label, current, fallback, palette, noColorLabel, disabled } = defineProps<{
  icon: Component
  label: string
  current: string | undefined
  fallback: string
  palette: readonly (readonly string[])[]
  noColorLabel: string
  disabled?: boolean
}>()

const emit = defineEmits<{ pick: [color: string | null] }>()
</script>

<template>
  <PopoverRoot>
    <div class="inline-flex items-center">
      <button
        type="button"
        class="inline-flex h-7 min-w-7 flex-col items-center justify-center rounded-s px-1 outline-none hover:bg-surface-muted focus-visible:ring-2 focus-visible:ring-focus disabled:opacity-40"
        :aria-label="label"
        :title="label"
        :disabled="disabled"
        @mousedown.prevent
        @click="emit('pick', current ?? fallback)"
      >
        <component :is="icon" class="size-4" aria-hidden="true" />
        <span
          class="h-1 w-4 rounded-sm border border-border"
          :style="{ backgroundColor: current ?? fallback }"
        />
      </button>
      <PopoverTrigger
        class="inline-flex h-7 w-4 items-center justify-center rounded-e outline-none hover:bg-surface-muted focus-visible:ring-2 focus-visible:ring-focus disabled:opacity-40"
        :aria-label="`${label} ▾`"
        :disabled="disabled"
        @mousedown.prevent
      >
        <ChevronDown class="size-3 text-text-muted" aria-hidden="true" />
      </PopoverTrigger>
    </div>
    <PopoverPortal>
      <PopoverContent
        class="z-50 rounded-md border border-border bg-surface-raised p-3 shadow-lg"
        :side-offset="4"
        align="start"
        @close-auto-focus.prevent
      >
        <PopoverClose
          class="mb-2 flex w-full items-center gap-2 rounded px-2 py-1 text-sm hover:bg-surface-muted"
          @click="emit('pick', null)"
        >
          <Ban class="size-4" aria-hidden="true" />
          {{ noColorLabel }}
        </PopoverClose>
        <div class="grid grid-cols-8 gap-1" role="group" :aria-label="label">
          <template v-for="row in palette" :key="row[0]">
            <PopoverClose
              v-for="color in row"
              :key="color"
              class="size-5 rounded-full border border-border hover:scale-110 focus-visible:ring-2 focus-visible:ring-focus"
              :class="{ 'ring-2 ring-accent': current === color }"
              :style="{ backgroundColor: color }"
              :aria-label="color"
              :aria-pressed="current === color"
              :title="color"
              @click="emit('pick', color)"
            />
          </template>
        </div>
      </PopoverContent>
    </PopoverPortal>
  </PopoverRoot>
</template>
