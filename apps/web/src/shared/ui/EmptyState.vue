<script setup lang="ts">
defineProps<{
  heading: string
  description?: string
}>()
</script>

<template>
  <div class="flex flex-col items-center gap-4 py-12 text-center">
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="80"
      height="80"
      viewBox="0 0 80 80"
      fill="none"
      aria-hidden="true"
      class="text-text-muted"
    >
      <rect
        x="14"
        y="18"
        width="52"
        height="44"
        rx="4"
        stroke="currentColor"
        stroke-width="2"
        fill="none"
      />
      <rect
        x="14"
        y="18"
        width="52"
        height="10"
        rx="4"
        stroke="currentColor"
        stroke-width="2"
        fill="none"
      />
      <line
        x1="24"
        y1="38"
        x2="56"
        y2="38"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
      />
      <line
        x1="24"
        y1="48"
        x2="46"
        y2="48"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
      />
      <line
        x1="24"
        y1="58"
        x2="40"
        y2="58"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
      />
    </svg>
    <div class="flex flex-col gap-1">
      <p class="font-medium text-text">{{ heading }}</p>
      <p v-if="description" class="text-sm text-text-muted">{{ description }}</p>
    </div>
    <slot />
  </div>
</template>
