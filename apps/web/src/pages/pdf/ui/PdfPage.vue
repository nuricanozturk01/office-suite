<script setup lang="ts">
import { useI18n } from 'vue-i18n'

import { usePdfQuery } from '@/entities/pdf'
import { EmbedWorkspace } from '@/widgets/embed-workspace'

const { id } = defineProps<{ id: string }>()

const { t } = useI18n()
const query = usePdfQuery(() => id)
</script>

<template>
  <EmbedWorkspace
    v-if="query.data.value"
    app="pdfcraft"
    :resource-id="id"
    :title="query.data.value.title"
    :untitled="t('pdf.untitled')"
  />
</template>
