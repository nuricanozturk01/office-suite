<script setup lang="ts">
import { FileText } from '@lucide/vue'
import { useI18n } from 'vue-i18n'

import type { DocumentSummary } from '@/entities/document'
import { errorMessageKey } from '@/shared/api'
import { InlineAlert, NewFileTile } from '@/shared/ui'

import { useCreateDocument } from '../model/use-create-document'

const emit = defineEmits<{ created: [document: DocumentSummary] }>()

const { t } = useI18n()
const create = useCreateDocument((document) => {
  emit('created', document)
})
</script>

<template>
  <div class="flex flex-col gap-2">
    <NewFileTile
      :icon="FileText"
      :label="t('documents.blank')"
      :hint="t('files.hintDocs')"
      tone="docs"
      :busy="create.isPending.value"
      @click="create.mutate()"
    />
    <InlineAlert v-if="create.isError.value" class="w-52">
      {{ t(errorMessageKey(create.error.value)) }}
    </InlineAlert>
  </div>
</template>
