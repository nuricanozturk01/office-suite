<script setup lang="ts">
const { tone = 'danger' } = defineProps<{ tone?: 'danger' | 'success' }>()
</script>

<template>
  <p
    :role="tone === 'danger' ? 'alert' : 'status'"
    :class="tone === 'danger' ? 'bg-danger-surface text-danger' : 'bg-surface-muted text-success'"
    class="rounded-md px-3 py-2 text-sm"
  >
    <slot />
  </p>
</template>
