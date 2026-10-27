<script setup lang="ts">
import { FileText, FileType, Presentation, Table2 } from '@lucide/vue'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { RouterLink } from 'vue-router'

import { type SharedFile, useSharedWithMeQuery } from '@/entities/share'
import { errorMessageKey } from '@/shared/api'
import { RouteName } from '@/shared/config/routes'
import { AppButton, EmptyState, InlineAlert } from '@/shared/ui'

const { t, locale } = useI18n()
const query = useSharedWithMeQuery()

const files = computed(() => query.data.value?.pages.flatMap((page) => page.items) ?? [])
const dateFormat = computed(() => new Intl.DateTimeFormat(locale.value, { dateStyle: 'medium' }))

const ROUTES = {
  document: RouteName.Document,
  spreadsheet: RouteName.Spreadsheet,
  presentation: RouteName.Presentation,
  pdf: RouteName.Pdf,
} as const

function route(file: SharedFile) {
  return { name: ROUTES[file.kind], params: { id: file.id } }
}
</script>

<template>
  <div class="flex flex-col gap-3">
    <InlineAlert v-if="query.isError.value">{{
      t(errorMessageKey(query.error.value))
    }}</InlineAlert>
    <EmptyState
      v-if="query.isSuccess.value && files.length === 0"
      :heading="t('files.sharedEmpty')"
      :description="t('files.sharedEmptyHint')"
    />
    <ul v-if="files.length > 0">
      <li
        v-for="file in files"
        :key="file.id"
        class="relative grid grid-cols-[1fr_auto] items-center gap-x-4 border-b border-border px-3 py-3 hover:bg-surface-muted sm:grid-cols-[1fr_7rem_8rem_8rem]"
      >
        <div class="flex min-w-0 items-center gap-3">
          <Table2
            v-if="file.kind === 'spreadsheet'"
            class="size-5 shrink-0 text-sheets"
            aria-hidden="true"
          />
          <Presentation
            v-else-if="file.kind === 'presentation'"
            class="size-5 shrink-0 text-slides"
            aria-hidden="true"
          />
          <FileType
            v-else-if="file.kind === 'pdf'"
            class="size-5 shrink-0 text-pdf"
            aria-hidden="true"
          />
          <FileText v-else class="size-5 shrink-0 text-docs" aria-hidden="true" />
          <RouterLink :to="route(file)" class="truncate font-medium after:absolute after:inset-0">
            {{ file.title }}
          </RouterLink>
        </div>
        <span class="hidden text-sm text-text-muted sm:block">{{
          t(`files.type.${file.kind}`)
        }}</span>
        <span class="hidden text-sm text-text-muted tabular-nums sm:block">
          {{ dateFormat.format(new Date(file.shared_at)) }}
        </span>
        <span class="text-sm text-text-muted">{{ t(`share.roles.${file.role}`) }}</span>
      </li>
    </ul>
    <AppButton
      v-if="query.hasNextPage.value"
      variant="ghost"
      :loading="query.isFetchingNextPage.value"
      class="self-center"
      @click="query.fetchNextPage()"
    >
      {{ t('files.loadMore') }}
    </AppButton>
  </div>
</template>
