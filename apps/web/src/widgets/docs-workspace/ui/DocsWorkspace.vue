<script setup lang="ts">
import { FileText } from '@lucide/vue'
import { useI18n } from 'vue-i18n'
import { RouterLink, useRouter } from 'vue-router'

import { takePendingImport, useDocumentQuery } from '@/entities/document'
import { useSessionStore } from '@/entities/session'
import { DocsEditor } from '@/editors/docs'
import { UserMenu } from '@/features/auth'
import { useCreateDocument } from '@/features/create-document'
import { DocumentTitleInput } from '@/features/rename-document'
import { errorMessageKey } from '@/shared/api'
import { RouteName } from '@/shared/config/routes'
import { InlineAlert } from '@/shared/ui'

const { documentId } = defineProps<{ documentId: string }>()

const { t } = useI18n()
const router = useRouter()
const session = useSessionStore()
const query = useDocumentQuery(() => documentId)

const initialContent = takePendingImport(documentId)

const create = useCreateDocument((document) => {
  void router.push({ name: RouteName.Document, params: { id: document.id } })
})

async function toLogin(): Promise<void> {
  await router.push({ name: RouteName.Login })
}
</script>

<template>
  <InlineAlert v-if="query.isError.value" class="m-4">
    {{ t(errorMessageKey(query.error.value)) }}
  </InlineAlert>
  <DocsEditor
    v-else-if="query.data.value && session.user"
    :key="documentId"
    :document-id="documentId"
    :title="query.data.value.title"
    :user="{ id: session.user.id, name: session.user.display_name }"
    :get-token="session.token"
    :initial-content="initialContent"
    @new-document="create.mutate()"
  >
    <template #leading>
      <RouterLink
        :to="{ name: RouteName.Documents }"
        :aria-label="t('documents.home')"
        :title="t('documents.home')"
        class="shrink-0 rounded p-1 hover:bg-surface-muted"
      >
        <FileText class="size-8 text-accent" aria-hidden="true" />
      </RouterLink>
    </template>
    <template #title>
      <DocumentTitleInput :document="query.data.value" />
    </template>
    <template #actions>
      <UserMenu @signed-out="toLogin" />
    </template>
  </DocsEditor>
</template>
