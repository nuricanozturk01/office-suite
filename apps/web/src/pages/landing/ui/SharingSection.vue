<script setup lang="ts">
import { useI18n } from 'vue-i18n'

const { t } = useI18n()
const ROLES = ['viewer', 'commenter', 'editor'] as const
</script>

<template>
  <section id="sharing" aria-labelledby="sharing-title" class="mx-auto max-w-6xl px-5 py-20">
    <div class="grid gap-10 md:grid-cols-[1fr_1fr] md:gap-16">
      <div>
        <h2 id="sharing-title" class="max-w-lg text-3xl font-semibold">
          {{ t('landing.sharing.title') }}
        </h2>
        <p class="mt-4 max-w-md text-text-muted">{{ t('landing.sharing.lead') }}</p>
      </div>
      <dl class="divide-y divide-border rounded-md border border-border bg-surface-raised">
        <div
          v-for="role in ROLES"
          :key="role"
          class="grid gap-1 px-5 py-4 sm:grid-cols-[9rem_1fr] sm:gap-4"
        >
          <dt class="font-medium">{{ t(`landing.sharing.${role}.name`) }}</dt>
          <dd class="text-text-muted">{{ t(`landing.sharing.${role}.text`) }}</dd>
        </div>
      </dl>
    </div>
  </section>
</template>
