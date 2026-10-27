<script setup lang="ts">
import { useI18n } from 'vue-i18n'

const { t } = useI18n()
const TOOLS = [
  { key: 'docs', bar: 'bg-docs' },
  { key: 'sheets', bar: 'bg-sheets' },
  { key: 'slides', bar: 'bg-slides' },
  { key: 'pdf', bar: 'bg-pdf' },
] as const
</script>

<template>
  <section id="tools" aria-labelledby="tools-title" class="mx-auto max-w-6xl px-5 py-20">
    <div class="grid gap-10 md:grid-cols-[1fr_1.4fr] md:gap-16">
      <div>
        <h2 id="tools-title" class="text-3xl font-semibold">{{ t('landing.tools.title') }}</h2>
        <p class="mt-4 max-w-md text-text-muted">{{ t('landing.tools.lead') }}</p>
      </div>
      <ul class="border-t border-border">
        <li
          v-for="tool in TOOLS"
          :key="tool.key"
          class="grid grid-cols-[0.375rem_1fr] gap-5 border-b border-border py-6"
        >
          <span :class="tool.bar" class="rounded-full" aria-hidden="true"></span>
          <div class="flex flex-col gap-1 sm:flex-row sm:items-baseline sm:gap-6">
            <h3 class="w-24 shrink-0 text-xl font-semibold">
              {{ t(`landing.tools.${tool.key}.name`) }}
            </h3>
            <div>
              <p>{{ t(`landing.tools.${tool.key}.text`) }}</p>
              <p class="mt-1 text-sm text-text-muted">
                {{ t(`landing.tools.${tool.key}.formats`) }}
              </p>
            </div>
          </div>
        </li>
      </ul>
    </div>
  </section>
</template>
