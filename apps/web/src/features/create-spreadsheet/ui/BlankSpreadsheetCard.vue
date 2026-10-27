<script setup lang="ts">
import { Table2 } from '@lucide/vue'
import { useI18n } from 'vue-i18n'

import type { SpreadsheetDetail } from '@/entities/spreadsheet'
import { errorMessageKey } from '@/shared/api'
import { InlineAlert, NewFileTile } from '@/shared/ui'

import { useCreateSpreadsheet } from '../model/use-create-spreadsheet'

const emit = defineEmits<{ created: [spreadsheet: SpreadsheetDetail] }>()

const { t } = useI18n()
const create = useCreateSpreadsheet((spreadsheet) => {
  emit('created', spreadsheet)
})
</script>

<template>
  <div class="flex flex-col gap-2">
    <NewFileTile
      :icon="Table2"
      :label="t('sheets.blank')"
      :hint="t('files.hintSheets')"
      tone="sheets"
      :busy="create.isPending.value"
      @click="create.mutate()"
    />
    <InlineAlert v-if="create.isError.value" class="w-52">
      {{ t(errorMessageKey(create.error.value)) }}
    </InlineAlert>
  </div>
</template>
