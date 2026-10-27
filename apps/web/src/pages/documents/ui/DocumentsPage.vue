<script setup lang="ts">
import { Search } from '@lucide/vue'
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'

import type { DocumentSummary } from '@/entities/document'
import type { PdfSummary } from '@/entities/pdf'
import type { PresentationDetail } from '@/entities/presentation'
import type { SpreadsheetDetail } from '@/entities/spreadsheet'
import { BlankDocumentCard } from '@/features/create-document'
import { BlankPresentationCard } from '@/features/create-presentation'
import { BlankSpreadsheetCard } from '@/features/create-spreadsheet'
import { ImportDocumentCard } from '@/features/import-document'
import { ImportPdfCard } from '@/features/import-pdf'
import { ImportPresentationCard } from '@/features/import-presentation'
import { ImportSpreadsheetCard } from '@/features/import-spreadsheet'
import { RouteName } from '@/shared/config/routes'
import { AppHeader } from '@/widgets/app-header'
import { RecentFiles } from '@/widgets/recent-files'
import { SharedFiles } from '@/widgets/shared-files'

const { t } = useI18n()
const router = useRouter()
const search = ref('')
const KINDS = ['all', 'document', 'spreadsheet', 'presentation', 'pdf'] as const
const kind = ref<(typeof KINDS)[number]>('all')
const view = ref<'mine' | 'shared'>('mine')
const sort = ref<'updated' | 'name'>('updated')
const KIND_DOTS = {
  all: 'bg-text-muted',
  document: 'bg-docs',
  spreadsheet: 'bg-sheets',
  presentation: 'bg-slides',
  pdf: 'bg-pdf',
} as const

async function openDocument(document: DocumentSummary): Promise<void> {
  await router.push({ name: RouteName.Document, params: { id: document.id } })
}

async function openPdf(pdf: PdfSummary): Promise<void> {
  await router.push({ name: RouteName.Pdf, params: { id: pdf.id } })
}

async function openPresentation(presentation: PresentationDetail): Promise<void> {
  await router.push({ name: RouteName.Presentation, params: { id: presentation.id } })
}

async function openSpreadsheet(spreadsheet: SpreadsheetDetail): Promise<void> {
  await router.push({ name: RouteName.Spreadsheet, params: { id: spreadsheet.id } })
}
</script>

<template>
  <div class="min-h-dvh bg-surface">
    <AppHeader>
      <label class="relative w-full max-w-xl">
        <span class="sr-only">{{ t('files.search') }}</span>
        <Search
          class="pointer-events-none absolute start-3 top-1/2 size-4 -translate-y-1/2 text-text-muted"
          aria-hidden="true"
        />
        <input
          v-model="search"
          type="search"
          :placeholder="t('files.search')"
          class="h-10 w-full rounded-md border border-border bg-surface-raised ps-9 pe-3 text-sm outline-none focus:border-accent"
        />
      </label>
    </AppHeader>
    <div class="mx-auto grid max-w-7xl gap-x-10 px-4 py-6 lg:grid-cols-[14rem_1fr]">
      <nav :aria-label="t('files.navLabel')" class="flex flex-col gap-6">
        <section aria-labelledby="new-files">
          <h2 id="new-files" class="mb-1 px-2 text-sm font-medium text-text-muted">
            {{ t('files.newLabel') }}
          </h2>
          <BlankDocumentCard @created="openDocument" />
          <BlankSpreadsheetCard @created="openSpreadsheet" />
          <BlankPresentationCard @created="openPresentation" />
        </section>
        <section aria-labelledby="upload-files">
          <h2 id="upload-files" class="mb-2 px-2 text-sm font-medium text-text-muted">
            {{ t('files.uploadLabel') }}
          </h2>
          <div class="flex flex-wrap gap-2 px-2 lg:flex-col lg:items-start">
            <ImportDocumentCard @imported="openDocument" />
            <ImportSpreadsheetCard @imported="openSpreadsheet" />
            <ImportPresentationCard @imported="openPresentation" />
            <ImportPdfCard @imported="openPdf" />
          </div>
        </section>
        <section>
          <ul class="flex gap-1 lg:flex-col">
            <li v-for="option in ['mine', 'shared'] as const" :key="option">
              <button
                type="button"
                class="w-full rounded-md px-2 py-2 text-start text-sm"
                :class="
                  view === option
                    ? 'bg-surface-muted font-medium'
                    : 'text-text-muted hover:bg-surface-muted'
                "
                :aria-current="view === option ? 'page' : undefined"
                @click="view = option"
              >
                {{ option === 'mine' ? t('files.myFiles') : t('files.sharedWithMe') }}
              </button>
            </li>
          </ul>
        </section>
      </nav>
      <main id="main" tabindex="-1" class="min-w-0">
        <div class="mb-4 flex flex-wrap items-center justify-between gap-3">
          <h1 class="text-2xl font-semibold">
            {{ view === 'mine' ? t('files.myFiles') : t('files.sharedWithMe') }}
          </h1>
          <div v-if="view === 'mine'" class="flex flex-wrap items-center gap-4">
            <div role="group" :aria-label="t('files.filterLabel')" class="flex flex-wrap gap-1">
              <button
                v-for="option in KINDS"
                :key="option"
                type="button"
                class="inline-flex items-center gap-2 rounded-md px-2.5 py-1.5 text-sm"
                :class="
                  kind === option
                    ? 'bg-text text-surface'
                    : 'text-text-muted hover:bg-surface-muted'
                "
                :aria-pressed="kind === option"
                @click="kind = option"
              >
                <span
                  class="size-2 rounded-full"
                  :class="KIND_DOTS[option]"
                  aria-hidden="true"
                ></span>
                {{ t(`files.kinds.${option}`) }}
              </button>
            </div>
            <label class="flex items-center gap-2 text-sm text-text-muted">
              {{ t('files.sortLabel') }}
              <select
                v-model="sort"
                class="rounded-md border border-border bg-surface-raised px-2 py-1.5 text-text"
              >
                <option value="updated">{{ t('files.sort.updated') }}</option>
                <option value="name">{{ t('files.sort.name') }}</option>
              </select>
            </label>
          </div>
        </div>
        <RecentFiles v-if="view === 'mine'" :filter="search" :kind="kind" :sort="sort" />
        <SharedFiles v-else />
      </main>
    </div>
  </div>
</template>
