<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { RouterView } from 'vue-router'

const { t } = useI18n()
</script>

<template>
  <a
    href="#main"
    class="sr-only rounded-md bg-surface-raised px-3 py-2 focus:not-sr-only focus:fixed focus:start-2 focus:top-2 focus:z-50"
  >
    {{ t('app.skipToContent') }}
  </a>
  <RouterView />
</template>
