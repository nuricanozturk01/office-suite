<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { RouterLink } from 'vue-router'

import { RouteName } from '@/shared/config/routes'

defineProps<{ title: string; lead?: string }>()

const { t } = useI18n()
</script>

<template>
  <div class="grid min-h-dvh lg:grid-cols-[minmax(0,5fr)_minmax(0,6fr)]">
    <aside class="hidden flex-col justify-between bg-ink p-10 text-white lg:flex">
      <RouterLink :to="{ name: RouteName.Landing }" class="font-display text-xl font-semibold">
        {{ t('app.name') }}
      </RouterLink>
      <div>
        <div class="mb-6 flex h-1.5 w-40 gap-1" aria-hidden="true">
          <span class="flex-1 bg-docs"></span>
          <span class="flex-1 bg-sheets"></span>
          <span class="flex-1 bg-slides"></span>
          <span class="flex-1 bg-pdf"></span>
        </div>
        <p class="max-w-sm font-display text-3xl leading-tight font-semibold">
          {{ t('auth.panel') }}
        </p>
      </div>
    </aside>
    <main id="main" class="flex items-center justify-center p-6">
      <div class="w-full max-w-sm">
        <RouterLink
          :to="{ name: RouteName.Landing }"
          class="mb-10 inline-block font-display text-xl font-semibold lg:hidden"
        >
          {{ t('app.name') }}
        </RouterLink>
        <h1 class="text-3xl font-semibold">{{ title }}</h1>
        <p v-if="lead" class="mt-2 mb-8 text-text-muted">{{ lead }}</p>
        <div v-else class="mb-8"></div>
        <slot />
      </div>
    </main>
  </div>
</template>
