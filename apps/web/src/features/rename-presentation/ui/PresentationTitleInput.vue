<script setup lang="ts">
import { useMutation, useQueryClient } from '@tanstack/vue-query'
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { presentationKeys, type PresentationDetail } from '@/entities/presentation'
import { api, errorMessageKey, unwrap } from '@/shared/api'

const { presentation } = defineProps<{ presentation: PresentationDetail }>()

const { t } = useI18n()
const queryClient = useQueryClient()
const draft = ref(presentation.title)

watch(
  () => presentation.title,
  (title) => {
    draft.value = title
  },
)

const rename = useMutation({
  mutationFn: (title: string) =>
    unwrap(
      api.PATCH('/api/v1/presentations/{id}', {
        params: { path: { id: presentation.id } },
        body: { title },
      }),
    ),
  onSuccess: async (updated) => {
    queryClient.setQueryData<PresentationDetail>(presentationKeys.detail(updated.id), (current) =>
      current === undefined
        ? current
        : { ...current, title: updated.title, updated_at: updated.updated_at },
    )
    await queryClient.invalidateQueries({ queryKey: presentationKeys.lists() })
  },
  onError: () => {
    draft.value = presentation.title
  },
})

function commit(): void {
  const title = draft.value.trim()
  const alreadySaving = rename.isPending.value && rename.variables.value === title
  if (title === '' || title === presentation.title.trim() || alreadySaving) {
    draft.value = presentation.title
    return
  }
  rename.mutate(title)
}

function cancel(event: KeyboardEvent): void {
  draft.value = presentation.title
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
      :aria-label="t('slides.titleLabel')"
      :placeholder="t('slides.untitled')"
      maxlength="255"
      class="min-w-0 rounded border border-transparent bg-transparent px-1.5 py-0.5 text-lg hover:border-border focus:border-slides focus:outline-none"
      @blur="commit"
      @keydown.enter.prevent="commitOnEnter"
      @keydown.esc.prevent="cancel"
    />
    <p v-if="rename.isError.value" role="alert" class="px-2 text-xs text-danger">
      {{ t(errorMessageKey(rename.error.value)) }}
    </p>
  </div>
</template>
