<script setup lang="ts">
import { useI18n } from 'vue-i18n'

const { t } = useI18n()
const ROWS = [
  { key: 'docs', ext: '.docx', both: true },
  { key: 'sheets', ext: '.xlsx', both: true },
  { key: 'slides', ext: '.pptx', both: true },
  { key: 'pdf', ext: '.pdf', both: false },
] as const
</script>

<template>
  <section id="formats" aria-labelledby="formats-title" class="bg-surface-muted py-20">
    <div class="mx-auto grid max-w-6xl gap-10 px-5 md:grid-cols-[1.4fr_1fr] md:gap-16">
      <table class="order-2 w-full border-collapse text-start md:order-1">
        <thead>
          <tr class="border-b border-text text-sm">
            <th scope="col" class="py-3 pe-4 text-start font-medium">
              {{ t('landing.formats.columns.app') }}
            </th>
            <th scope="col" class="py-3 pe-4 text-start font-medium">
              {{ t('landing.formats.columns.upload') }}
            </th>
            <th scope="col" class="py-3 text-start font-medium">
              {{ t('landing.formats.columns.download') }}
            </th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="row in ROWS" :key="row.key" class="border-b border-border">
            <th scope="row" class="py-4 pe-4 text-start font-display text-lg font-semibold">
              {{ t(`landing.tools.${row.key}.name`) }}
            </th>
            <td class="py-4 pe-4 font-medium tabular-nums">{{ row.ext }}</td>
            <td class="py-4" :class="row.both ? 'font-medium tabular-nums' : 'text-text-muted'">
              {{ row.both ? row.ext : t('landing.formats.none') }}
            </td>
          </tr>
        </tbody>
      </table>
      <div class="order-1 md:order-2">
        <h2 id="formats-title" class="text-3xl font-semibold">{{ t('landing.formats.title') }}</h2>
        <p class="mt-4 max-w-md text-text-muted">{{ t('landing.formats.lead') }}</p>
      </div>
    </div>
  </section>
</template>
