<script setup lang="ts">
import { useMutation, useQueryClient } from '@tanstack/vue-query'
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { pdfKeys, type PdfSummary } from '@/entities/pdf'
import { api, errorMessageKey, unwrap } from '@/shared/api'

const { pdf } = defineProps<{ pdf: PdfSummary }>()

const { t } = useI18n()
const queryClient = useQueryClient()
const draft = ref(pdf.title)

watch(
  () => pdf.title,
  (title) => {
    draft.value = title
  },
)

const rename = useMutation({
  mutationFn: (title: string) =>
    unwrap(
      api.PATCH('/api/v1/pdfs/{id}', {
        params: { path: { id: pdf.id } },
        body: { title },
      }),
    ),
  onSuccess: async (updated) => {
    queryClient.setQueryData(pdfKeys.detail(updated.id), updated)
    await queryClient.invalidateQueries({ queryKey: pdfKeys.lists() })
  },
  onError: () => {
    draft.value = pdf.title
  },
})

function commit(): void {
  const title = draft.value.trim()
  const alreadySaving = rename.isPending.value && rename.variables.value === title
  if (title === pdf.title.trim() || alreadySaving) {
    return
  }
  rename.mutate(title)
}

function cancel(event: KeyboardEvent): void {
  draft.value = pdf.title
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
      :aria-label="t('pdf.titleLabel')"
      :placeholder="t('pdf.untitled')"
      maxlength="255"
      class="min-w-0 rounded border border-transparent bg-transparent px-1.5 py-0.5 text-lg hover:border-border focus:border-accent focus:outline-none"
      @blur="commit"
      @keydown.enter.prevent="commitOnEnter"
      @keydown.esc.prevent="cancel"
    />
    <p v-if="rename.isError.value" role="alert" class="px-2 text-xs text-danger">
      {{ t(errorMessageKey(rename.error.value)) }}
    </p>
  </div>
</template>
