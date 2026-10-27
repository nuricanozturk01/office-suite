<script setup lang="ts">
import { Presentation } from '@lucide/vue'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { RouterLink, useRouter } from 'vue-router'

import { usePresentationQuery } from '@/entities/presentation'
import { parseDeck, SlidesEditor } from '@/editors/slides'
import { UserMenu } from '@/features/auth'
import { useCreatePresentation } from '@/features/create-presentation'
import { PresentationTitleInput } from '@/features/rename-presentation'
import { errorMessageKey } from '@/shared/api'
import { RouteName } from '@/shared/config/routes'
import { InlineAlert } from '@/shared/ui'

const { presentationId } = defineProps<{ presentationId: string }>()

const { t } = useI18n()
const router = useRouter()
const query = usePresentationQuery(() => presentationId)
const generation = ref(0)

const deck = computed(() => {
  const content = query.data.value?.content
  return content === undefined ? null : parseDeck(content)
})

const create = useCreatePresentation((presentation) => {
  void router.push({ name: RouteName.Presentation, params: { id: presentation.id } })
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
  <InlineAlert v-else-if="query.isSuccess.value && deck === null" class="m-4">
    {{ t('errors.unknown') }}
  </InlineAlert>
  <SlidesEditor
    v-else-if="query.data.value && deck"
    :key="`${presentationId}-${String(generation)}`"
    :presentation-id="presentationId"
    :initial="deck"
    :version="query.data.value.version"
    :title="query.data.value.title"
    @reload="reload"
    @new-presentation="create.mutate()"
  >
    <template #leading>
      <RouterLink
        :to="{ name: RouteName.Documents }"
        :aria-label="t('slides.home')"
        :title="t('slides.home')"
        class="shrink-0 rounded p-1 hover:bg-surface-muted"
      >
        <Presentation class="size-8 text-slides" aria-hidden="true" />
      </RouterLink>
    </template>
    <template #title>
      <PresentationTitleInput :presentation="query.data.value" />
    </template>
    <template #actions>
      <UserMenu @signed-out="toLogin" />
    </template>
  </SlidesEditor>
</template>
