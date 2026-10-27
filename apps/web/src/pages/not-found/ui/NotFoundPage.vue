<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { RouterLink } from 'vue-router'

const { t } = useI18n()
</script>

<template>
  <main id="main" class="grid min-h-dvh place-items-center p-8 text-center">
    <div class="flex flex-col items-center gap-6">
      <svg
        xmlns="http://www.w3.org/2000/svg"
        width="120"
        height="120"
        viewBox="0 0 120 120"
        fill="none"
        aria-hidden="true"
        class="text-text-muted"
      >
        <circle cx="60" cy="60" r="50" stroke="currentColor" stroke-width="2" />
        <path d="M38 44 L44 38 L82 76 L76 82 Z" fill="currentColor" opacity="0.3" />
        <path d="M76 38 L82 44 L44 82 L38 76 Z" fill="currentColor" opacity="0.3" />
        <text
          x="60"
          y="32"
          text-anchor="middle"
          font-size="14"
          font-weight="600"
          fill="currentColor"
          opacity="0.6"
        >
          404
        </text>
      </svg>

      <div class="flex flex-col gap-2">
        <h1 class="text-2xl font-semibold text-text">{{ t('notFound.title') }}</h1>
        <p class="text-text-muted">{{ t('notFound.description') }}</p>
      </div>

      <RouterLink
        to="/"
        class="inline-flex items-center gap-2 rounded-md bg-accent px-4 py-2 text-sm font-medium text-accent-contrast transition-colors hover:bg-accent-hover"
      >
        {{ t('notFound.home') }}
      </RouterLink>
    </div>
  </main>
</template>
