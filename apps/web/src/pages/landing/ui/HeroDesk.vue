<script setup lang="ts">
import { useI18n } from 'vue-i18n'

const { t } = useI18n()
</script>

<template>
  <div
    role="img"
    :aria-label="t('landing.desk.label')"
    class="relative mx-auto aspect-[6/5] w-full max-w-2xl select-none"
  >
    <!-- Sheets: furthest back -->
    <div
      aria-hidden="true"
      class="absolute start-[38%] top-0 w-[60%] -rotate-3 overflow-hidden rounded-sm border border-border bg-surface-raised shadow-[0_18px_40px_-24px_oklch(30%_0.05_240/0.5)]"
    >
      <div class="h-1.5 bg-sheets"></div>
      <table class="w-full border-collapse text-xs leading-6">
        <thead>
          <tr class="bg-surface-muted text-text-muted">
            <th class="border-b border-border px-2 text-start font-medium">
              {{ t('landing.desk.colRegion') }}
            </th>
            <th class="border-s border-b border-border px-2 text-end font-medium">
              {{ t('landing.desk.colQ1') }}
            </th>
            <th class="border-s border-b border-border px-2 text-end font-medium">
              {{ t('landing.desk.colQ2') }}
            </th>
          </tr>
        </thead>
        <tbody class="tabular-nums">
          <tr>
            <td class="border-b border-border px-2">{{ t('landing.desk.rowNorth') }}</td>
            <td class="border-s border-b border-border px-2 text-end">412</td>
            <td class="border-s border-b border-border px-2 text-end">468</td>
          </tr>
          <tr>
            <td class="border-b border-border px-2">{{ t('landing.desk.rowSouth') }}</td>
            <td class="border-s border-b border-border px-2 text-end">389</td>
            <td class="border-s border-b border-border px-2 text-end">401</td>
          </tr>
          <tr>
            <td class="px-2">{{ t('landing.desk.rowEast') }}</td>
            <td class="border-s border-border px-2 text-end">350</td>
            <td
              class="border-s border-border px-2 text-end outline-2 -outline-offset-2 outline-sheets"
            >
              377
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Slides -->
    <div
      aria-hidden="true"
      class="absolute end-0 bottom-[4%] w-[50%] rotate-2 overflow-hidden rounded-sm border border-border bg-surface-raised shadow-[0_18px_40px_-24px_oklch(30%_0.05_240/0.5)]"
    >
      <div class="aspect-video px-4 py-3">
        <p class="font-display text-sm font-semibold text-slides">
          {{ t('landing.desk.slideTitle') }}
        </p>
        <ul class="mt-2 space-y-1 text-xs text-text-muted">
          <li class="flex gap-1.5">
            <span class="mt-1.5 size-1 shrink-0 rounded-full bg-slides"></span
            >{{ t('landing.desk.slideBullet1') }}
          </li>
          <li class="flex gap-1.5">
            <span class="mt-1.5 size-1 shrink-0 rounded-full bg-slides"></span
            >{{ t('landing.desk.slideBullet2') }}
          </li>
        </ul>
      </div>
      <div class="h-1.5 bg-slides"></div>
    </div>

    <!-- PDF -->
    <div
      aria-hidden="true"
      class="absolute start-[2%] bottom-0 w-[30%] rotate-[5deg] rounded-sm border border-border bg-surface-raised p-3 shadow-[0_18px_40px_-24px_oklch(30%_0.05_240/0.5)]"
    >
      <div class="mb-2 h-1 w-8 bg-pdf"></div>
      <div class="space-y-1.5">
        <div class="h-1 rounded-full bg-surface-muted"></div>
        <div class="h-1 rounded-full bg-surface-muted"></div>
        <div class="h-1 w-4/5 rounded-full bg-surface-muted"></div>
        <div class="mt-3 h-6 rounded-sm border border-dashed border-pdf/50"></div>
      </div>
      <p class="mt-2 truncate text-[0.6rem] text-text-muted">{{ t('landing.desk.pdfLabel') }}</p>
    </div>

    <!-- Docs: on top -->
    <div
      aria-hidden="true"
      class="absolute start-0 top-[22%] w-[58%] overflow-hidden rounded-sm border border-border bg-surface-raised shadow-[0_24px_50px_-26px_oklch(30%_0.05_240/0.6)]"
    >
      <div class="h-1.5 bg-docs"></div>
      <div class="px-5 pt-4 pb-6">
        <p class="font-display text-lg leading-tight font-semibold">
          {{ t('landing.desk.docHeading') }}
        </p>
        <p class="mt-3 text-xs leading-5 text-text-muted">
          {{ t('landing.desk.docLine1') }}
        </p>
        <p class="mt-2 text-xs leading-5 text-text-muted">
          {{ t('landing.desk.docLine2')
          }}<span class="caret ms-px inline-block h-3.5 w-px translate-y-0.5 bg-docs"></span>
        </p>
      </div>
    </div>
  </div>
</template>

<style scoped>
.caret {
  animation: blink 1.1s steps(1) infinite;
}

@keyframes blink {
  50% {
    opacity: 0;
  }
}

@media (prefers-reduced-motion: reduce) {
  .caret {
    animation: none;
  }
}
</style>
