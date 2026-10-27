<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import { useDocumentsQuery } from '@/entities/document'
import { useDeleteDocument } from '@/features/delete-document'
import { errorMessageKey } from '@/shared/api'
import { AppButton, InlineAlert } from '@/shared/ui'

import DocumentCard from './DocumentCard.vue'

/** Filters the loaded pages by title; server-side search arrives with the search module. */
const { filter = '' } = defineProps<{ filter?: string }>()

const { t, locale } = useI18n()
const query = useDocumentsQuery()
const remove = useDeleteDocument()

const documents = computed(() => query.data.value?.pages.flatMap((page) => page.items) ?? [])
const needle = computed(() => filter.trim().toLocaleLowerCase(locale.value))
const visible = computed(() =>
  needle.value === ''
    ? documents.value
    : documents.value.filter((document) =>
        document.title.toLocaleLowerCase(locale.value).includes(needle.value),
      ),
)
const dateFormat = computed(() => new Intl.DateTimeFormat(locale.value, { dateStyle: 'medium' }))
</script>

<template>
  <div class="flex flex-col gap-4">
    <InlineAlert v-if="query.isError.value">
      {{ t(errorMessageKey(query.error.value)) }}
    </InlineAlert>
    <InlineAlert v-if="remove.mutation.isError.value">
      {{ t(errorMessageKey(remove.mutation.error.value)) }}
    </InlineAlert>
    <p v-if="query.isSuccess.value && documents.length === 0" class="text-text-muted">
      {{ t('documents.empty') }}
    </p>
    <p v-else-if="documents.length > 0 && visible.length === 0" class="text-text-muted">
      {{ t('documents.noMatch') }}
    </p>
    <ul
      v-if="visible.length > 0"
      class="grid grid-cols-[repeat(auto-fill,minmax(14rem,1fr))] gap-5"
    >
      <DocumentCard
        v-for="document in visible"
        :key="document.id"
        :document="document"
        :edited-at="dateFormat.format(new Date(document.updated_at))"
        @delete="remove.confirmAndDelete(document)"
      />
    </ul>
    <AppButton
      v-if="query.hasNextPage.value"
      variant="ghost"
      :loading="query.isFetchingNextPage.value"
      class="self-center"
      @click="query.fetchNextPage()"
    >
      {{ t('documents.loadMore') }}
    </AppButton>
  </div>
</template>
