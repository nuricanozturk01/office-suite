<script setup lang="ts">
import { Table2 } from '@lucide/vue'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { RouterLink, useRouter } from 'vue-router'

import { useSpreadsheetQuery } from '@/entities/spreadsheet'
import { parseWorkbook, SheetsEditor, useEngineReady } from '@/editors/sheets'
import { UserMenu } from '@/features/auth'
import { useCreateSpreadsheet } from '@/features/create-spreadsheet'
import { SpreadsheetTitleInput } from '@/features/rename-spreadsheet'
import { errorMessageKey } from '@/shared/api'
import { RouteName } from '@/shared/config/routes'
import { InlineAlert } from '@/shared/ui'

const { spreadsheetId } = defineProps<{ spreadsheetId: string }>()

const { t } = useI18n()
const router = useRouter()
const query = useSpreadsheetQuery(() => spreadsheetId)
const generation = ref(0)
const engine = useEngineReady()

const workbook = computed(() => {
  const content = query.data.value?.content
  return content === undefined ? null : parseWorkbook(content)
})

const create = useCreateSpreadsheet((spreadsheet) => {
  void router.push({ name: RouteName.Spreadsheet, params: { id: spreadsheet.id } })
})

async function reload(): Promise<void> {
  await query.refetch()
  generation.value += 1
}

async function toLogin(): Promise<void> {
  await router.push({ name: RouteName.Login })
}
</script>

<template>
  <InlineAlert v-if="query.isError.value" class="m-4">
    {{ t(errorMessageKey(query.error.value)) }}
  </InlineAlert>
  <InlineAlert v-else-if="query.isSuccess.value && workbook === null" class="m-4">
    {{ t('errors.unknown') }}
  </InlineAlert>
  <InlineAlert v-else-if="engine.failed.value" class="m-4">
    {{ t('sheets.engineError') }}
  </InlineAlert>
  <SheetsEditor
    v-else-if="engine.ready.value && query.data.value && workbook"
    :key="`${spreadsheetId}-${String(generation)}`"
    :spreadsheet-id="spreadsheetId"
    :initial="workbook"
    :version="query.data.value.version"
    :title="query.data.value.title"
    @reload="reload"
    @new-spreadsheet="create.mutate()"
  >
    <template #leading>
      <RouterLink
        :to="{ name: RouteName.Documents }"
        :aria-label="t('sheets.home')"
        :title="t('sheets.home')"
        class="shrink-0 rounded p-1 hover:bg-surface-muted"
      >
        <Table2 class="size-8 text-sheets" aria-hidden="true" />
      </RouterLink>
    </template>
    <template #title>
      <SpreadsheetTitleInput :spreadsheet="query.data.value" />
    </template>
    <template #actions>
      <UserMenu @signed-out="toLogin" />
    </template>
  </SheetsEditor>
</template>
