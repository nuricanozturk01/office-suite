<script setup lang="ts">
import { Check } from '@lucide/vue'
import { DropdownMenuItem } from 'reka-ui'
import type { Component } from 'vue'

const { label, icon, checked, disabled, hint } = defineProps<{
  label: string
  icon?: Component
  checked?: boolean
  disabled?: boolean
  hint?: string
}>()

defineEmits<{ select: [] }>()
</script>

<template>
  <DropdownMenuItem
    class="flex cursor-default items-center gap-3 px-4 py-1.5 outline-none select-none data-disabled:opacity-40 data-highlighted:bg-surface-muted"
    :disabled="disabled"
    @select="$emit('select')"
  >
    <component :is="icon" v-if="icon" class="size-4 shrink-0" aria-hidden="true" />
    <span class="flex-1">{{ label }}</span>
    <Check v-if="checked" class="size-4 text-accent" aria-hidden="true" />
    <span v-if="hint" class="ps-4 text-xs text-text-muted">{{ hint }}</span>
  </DropdownMenuItem>
</template>
