<script setup lang="ts">
import { useMutation, useQueryClient } from '@tanstack/vue-query'
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { spreadsheetKeys, type SpreadsheetDetail } from '@/entities/spreadsheet'
import { api, errorMessageKey, unwrap } from '@/shared/api'

const { spreadsheet } = defineProps<{ spreadsheet: SpreadsheetDetail }>()

const { t } = useI18n()
const queryClient = useQueryClient()
const draft = ref(spreadsheet.title)

watch(
  () => spreadsheet.title,
  (title) => {
    draft.value = title
  },
)

const rename = useMutation({
  mutationFn: (title: string) =>
    unwrap(
      api.PATCH('/api/v1/spreadsheets/{id}', {
        params: { path: { id: spreadsheet.id } },
        body: { title },
      }),
    ),
  onSuccess: async (updated) => {
    queryClient.setQueryData<SpreadsheetDetail>(spreadsheetKeys.detail(updated.id), (current) =>
      current === undefined
        ? current
        : { ...current, title: updated.title, updated_at: updated.updated_at },
    )
    await queryClient.invalidateQueries({ queryKey: spreadsheetKeys.lists() })
  },
  onError: () => {
    draft.value = spreadsheet.title
  },
})

function commit(): void {
  const title = draft.value.trim()
  const alreadySaving = rename.isPending.value && rename.variables.value === title
  if (title === '' || title === spreadsheet.title.trim() || alreadySaving) {
    draft.value = spreadsheet.title
    return
  }
  rename.mutate(title)
}

function cancel(event: KeyboardEvent): void {
  draft.value = spreadsheet.title
  if (event.target instanceof HTMLInputElement) {
    event.target.blur()
  }
}

function commitOnEnter(event: KeyboardEvent): void {
  commit()
  if (event.target instanceof HTMLInputElement) {
    event.target.blur()
  }
}
</script>

<template>
  <div class="flex min-w-0 flex-col">
    <input
      v-model="draft"
      :aria-label="t('sheets.titleLabel')"
      :placeholder="t('sheets.untitled')"
      maxlength="255"
      class="min-w-0 rounded border border-transparent bg-transparent px-1.5 py-0.5 text-lg hover:border-border focus:border-sheets focus:outline-none"
      @blur="commit"
      @keydown.enter.prevent="commitOnEnter"
      @keydown.esc.prevent="cancel"
    />
    <p v-if="rename.isError.value" role="alert" class="px-2 text-xs text-danger">
      {{ t(errorMessageKey(rename.error.value)) }}
    </p>
  </div>
</template>
