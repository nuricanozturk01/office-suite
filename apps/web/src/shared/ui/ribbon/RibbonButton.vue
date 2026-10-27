<script setup lang="ts">
import { computed } from 'vue'
import { ChevronDown } from '@lucide/vue'
import type { Component } from 'vue'

/** `large` stacks icon over label (Word's big buttons); `small` is icon + label in a row. */
const { icon, label, size, active, disabled, caret, title } = defineProps<{
  icon: Component
  label: string
  size?: 'large' | 'small' | 'icon'
  active?: boolean
  disabled?: boolean
  caret?: boolean
  title?: string
}>()

defineEmits<{ click: [] }>()

const mode = computed(() => size ?? 'icon')
</script>

<template>
  <button
    type="button"
    class="inline-flex items-center justify-center rounded text-sm text-text outline-none select-none hover:bg-surface-muted focus-visible:ring-2 focus-visible:ring-focus disabled:opacity-40"
    :class="[
      mode === 'large' ? 'min-h-[3.75rem] min-w-12 flex-col gap-1 px-2 py-1' : 'h-7 gap-1.5 px-1.5',
      mode === 'icon' ? 'min-w-7' : '',
      active ? 'bg-accent/15 text-accent' : '',
    ]"
    :aria-label="label"
    :aria-pressed="active ? true : undefined"
    :title="title ?? label"
    :disabled="disabled"
    @mousedown.prevent
    @click="$emit('click')"
  >
    <component :is="icon" :class="mode === 'large' ? 'size-6' : 'size-4'" aria-hidden="true" />
    <span v-if="mode !== 'icon'" class="text-xs leading-tight whitespace-nowrap">
      {{ label }}
      <ChevronDown v-if="caret" class="ms-0.5 inline size-3 text-text-muted" aria-hidden="true" />
    </span>
  </button>
</template>
