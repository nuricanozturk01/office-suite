<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'

import { useSpreadsheetQuery } from '@/entities/spreadsheet'
import { useCreateSpreadsheet } from '@/features/create-spreadsheet'
import { RouteName } from '@/shared/config/routes'
import { EmbedWorkspace } from '@/widgets/embed-workspace'
import { SheetsWorkspace } from '@/widgets/sheets-workspace'

const { id } = defineProps<{ id: string }>()

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const create = useCreateSpreadsheet((created) => {
  void router.push({ name: RouteName.Spreadsheet, params: { id: created.id } })
})
const query = useSpreadsheetQuery(() => id)
// The reference app is the default editor; `?legacy` opens the built-in Vue editor.
const embedded = computed(() => route.query.legacy === undefined)
</script>

<template>
  <EmbedWorkspace
    v-if="embedded && query.data.value"
    app="gridcraft"
    :resource-id="id"
    :title="query.data.value.title"
    :untitled="t('sheets.untitled')"
    @new="create.mutate()"
  />
  <SheetsWorkspace v-else-if="!embedded" :spreadsheet-id="id" />
</template>
