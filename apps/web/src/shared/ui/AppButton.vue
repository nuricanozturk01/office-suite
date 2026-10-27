<script setup lang="ts">
const {
  variant = 'primary',
  type = 'button',
  loading,
} = defineProps<{
  variant?: 'primary' | 'ghost' | 'danger'
  type?: 'button' | 'submit'
  loading?: boolean
}>()

const variants = {
  primary: 'bg-accent text-accent-contrast hover:bg-accent-hover',
  ghost: 'text-text hover:bg-surface-muted',
  danger: 'text-danger hover:bg-danger-surface',
} as const
</script>

<template>
  <button
    :type="type"
    :disabled="loading"
    :aria-busy="loading"
    :class="variants[variant]"
    class="inline-flex items-center justify-center gap-2 rounded-md px-3 py-2 text-sm font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-60"
  >
    <slot />
  </button>
</template>
