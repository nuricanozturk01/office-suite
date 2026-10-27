<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'

import { useDocumentQuery } from '@/entities/document'
import { useCreateDocument } from '@/features/create-document'
import { RouteName } from '@/shared/config/routes'
import { EmbedWorkspace } from '@/widgets/embed-workspace'
import { DocsWorkspace } from '@/widgets/docs-workspace'

const { id } = defineProps<{ id: string }>()

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const create = useCreateDocument((created) => {
  void router.push({ name: RouteName.Document, params: { id: created.id } })
})
const query = useDocumentQuery(() => id)
// The reference app is the default editor; `?legacy` opens the built-in Vue editor.
const embedded = computed(() => route.query.legacy === undefined)
</script>

<template>
  <EmbedWorkspace
    v-if="embedded && query.data.value"
    app="wordcraft"
    :resource-id="id"
    :title="query.data.value.title"
    :untitled="t('documents.untitled')"
    @new="create.mutate()"
  />
  <DocsWorkspace v-else-if="!embedded" :document-id="id" />
</template>
