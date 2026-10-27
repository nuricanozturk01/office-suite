<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'

import { usePresentationQuery } from '@/entities/presentation'
import { useCreatePresentation } from '@/features/create-presentation'
import { RouteName } from '@/shared/config/routes'
import { EmbedWorkspace } from '@/widgets/embed-workspace'
import { SlidesWorkspace } from '@/widgets/slides-workspace'

const { id } = defineProps<{ id: string }>()

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const create = useCreatePresentation((created) => {
  void router.push({ name: RouteName.Presentation, params: { id: created.id } })
})
const query = usePresentationQuery(() => id)
// The reference app is the default editor; `?legacy` opens the built-in Vue editor.
const embedded = computed(() => route.query.legacy === undefined)
</script>

<template>
  <EmbedWorkspace
    v-if="embedded && query.data.value"
    app="deckcraft"
    :resource-id="id"
    :title="query.data.value.title"
    :untitled="t('slides.untitled')"
    @new="create.mutate()"
  />
  <SlidesWorkspace v-else-if="!embedded" :presentation-id="id" />
</template>
