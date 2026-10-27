<script setup lang="ts" generic="T extends string | number">
import { Check, ChevronDown } from '@lucide/vue'
import {
  DropdownMenuContent,
  DropdownMenuItemIndicator,
  DropdownMenuPortal,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuRoot,
  DropdownMenuTrigger,
} from 'reka-ui'

/** A dropdown that picks one value (font family, size, zoom, number format). */
const { label, options, value, display, optionStyle, disabled } = defineProps<{
  label: string
  options: readonly T[]
  value: T
  display: (option: T) => string
  optionStyle?: (option: T) => Record<string, string>
  disabled?: boolean
}>()

const emit = defineEmits<{ select: [value: T] }>()

function onSelect(raw: unknown): void {
  const picked = options.find((option) => option === raw)
  if (picked !== undefined) {
    emit('select', picked)
  }
}
</script>

<template>
  <DropdownMenuRoot :modal="false">
    <DropdownMenuTrigger
      class="inline-flex h-7 items-center justify-between gap-1 rounded border border-border bg-surface px-2 text-sm outline-none hover:bg-surface-muted focus-visible:ring-2 focus-visible:ring-focus disabled:opacity-40"
      :aria-label="`${label}: ${display(value)}`"
      :title="label"
      :disabled="disabled"
      @mousedown.prevent
    >
      <span class="truncate" :style="optionStyle?.(value)">{{ display(value) }}</span>
      <ChevronDown class="size-3.5 shrink-0 text-text-muted" aria-hidden="true" />
    </DropdownMenuTrigger>
    <DropdownMenuPortal>
      <DropdownMenuContent
        class="z-50 max-h-80 min-w-32 overflow-y-auto rounded-md border border-border bg-surface-raised py-1 text-sm shadow-lg"
        align="start"
        :side-offset="4"
        @close-auto-focus.prevent
      >
        <DropdownMenuRadioGroup :model-value="value" @update:model-value="onSelect">
          <DropdownMenuRadioItem
            v-for="option in options"
            :key="option"
            :value="option"
            class="relative flex cursor-default items-center py-1.5 ps-8 pe-4 outline-none select-none data-highlighted:bg-surface-muted"
            :style="optionStyle?.(option)"
          >
            <DropdownMenuItemIndicator class="absolute start-2">
              <Check class="size-4" aria-hidden="true" />
            </DropdownMenuItemIndicator>
            {{ display(option) }}
          </DropdownMenuRadioItem>
        </DropdownMenuRadioGroup>
      </DropdownMenuContent>
    </DropdownMenuPortal>
  </DropdownMenuRoot>
</template>
