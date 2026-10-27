<script setup lang="ts">
import { Presentation as PresentationIcon } from '@lucide/vue'
import { useI18n } from 'vue-i18n'

import type { PresentationDetail } from '@/entities/presentation'
import { errorMessageKey } from '@/shared/api'
import { InlineAlert, NewFileTile } from '@/shared/ui'

import { useCreatePresentation } from '../model/use-create-presentation'

const emit = defineEmits<{ created: [presentation: PresentationDetail] }>()

const { t } = useI18n()
const create = useCreatePresentation((presentation) => {
  emit('created', presentation)
})
</script>

<template>
  <div class="flex flex-col gap-2">
    <NewFileTile
      :icon="PresentationIcon"
      :label="t('slides.blank')"
      :hint="t('files.hintSlides')"
      tone="slides"
      :busy="create.isPending.value"
      @click="create.mutate()"
    />
    <InlineAlert v-if="create.isError.value" class="w-52">
      {{ t(errorMessageKey(create.error.value)) }}
    </InlineAlert>
  </div>
</template>
