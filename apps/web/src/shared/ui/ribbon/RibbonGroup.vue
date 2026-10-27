<script setup lang="ts">
const { label } = defineProps<{ label: string }>()
</script>

<template>
  <div
    role="group"
    :aria-label="label"
    class="flex shrink-0 flex-col border-e border-border px-2 last:border-e-0"
  >
    <div class="flex flex-1 items-start gap-1 py-0.5"><slot /></div>
    <span class="pb-0.5 text-center text-[11px] leading-tight text-text-muted">{{ label }}</span>
  </div>
</template>
