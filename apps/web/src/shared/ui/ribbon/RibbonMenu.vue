<script setup lang="ts">
import { computed } from 'vue'
import { ChevronDown } from '@lucide/vue'
import {
  DropdownMenuContent,
  DropdownMenuPortal,
  DropdownMenuRoot,
  DropdownMenuTrigger,
} from 'reka-ui'
import type { Component } from 'vue'

/** A ribbon button that opens a menu; fill the default slot with `RibbonMenuItem`s. */
const { icon, label, size, disabled } = defineProps<{
  icon: Component
  label: string
  size?: 'large' | 'small' | 'icon'
  disabled?: boolean
}>()

const mode = computed(() => size ?? 'icon')
</script>

<template>
  <DropdownMenuRoot :modal="false">
    <DropdownMenuTrigger
      class="inline-flex items-center justify-center rounded text-sm text-text outline-none select-none hover:bg-surface-muted focus-visible:ring-2 focus-visible:ring-focus disabled:opacity-40"
      :class="
        mode === 'large' ? 'min-h-[3.75rem] min-w-12 flex-col gap-1 px-2 py-1' : 'h-7 gap-1 px-1.5'
      "
      :aria-label="label"
      :title="label"
      :disabled="disabled"
      @mousedown.prevent
    >
      <component :is="icon" :class="mode === 'large' ? 'size-6' : 'size-4'" aria-hidden="true" />
      <span v-if="mode !== 'icon'" class="text-xs leading-tight whitespace-nowrap">
        {{ label }}
        <ChevronDown class="ms-0.5 inline size-3 text-text-muted" aria-hidden="true" />
      </span>
      <ChevronDown v-else class="size-3 text-text-muted" aria-hidden="true" />
    </DropdownMenuTrigger>
    <DropdownMenuPortal>
      <DropdownMenuContent
        class="z-50 min-w-52 rounded-md border border-border bg-surface-raised py-1.5 text-sm shadow-lg"
        align="start"
        :side-offset="4"
        @close-auto-focus.prevent
      >
        <slot />
      </DropdownMenuContent>
    </DropdownMenuPortal>
  </DropdownMenuRoot>
</template>
