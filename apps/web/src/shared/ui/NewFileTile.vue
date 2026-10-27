<script setup lang="ts">
import type { Component } from 'vue'

const { icon, label, hint, tone, busy } = defineProps<{
  icon: Component
  label: string
  hint: string
  tone: 'docs' | 'sheets' | 'slides'
  busy: boolean
}>()

defineEmits<{ click: [] }>()

const TONES = {
  docs: 'bg-docs/12 text-docs',
  sheets: 'bg-sheets/12 text-sheets',
  slides: 'bg-slides/12 text-slides',
} as const
</script>

<template>
  <button
    type="button"
    class="flex w-full items-center gap-3 rounded-md px-2 py-2 text-start transition-colors hover:bg-surface-muted disabled:opacity-60"
    :disabled="busy"
    :aria-busy="busy"
    @click="$emit('click')"
  >
    <span class="grid size-10 shrink-0 place-items-center rounded-md" :class="TONES[tone]">
      <component :is="icon" class="size-5" aria-hidden="true" />
    </span>
    <span class="flex min-w-0 flex-col">
      <span class="truncate text-sm font-medium">{{ label }}</span>
      <span class="truncate text-xs text-text-muted">{{ hint }}</span>
    </span>
  </button>
</template>
