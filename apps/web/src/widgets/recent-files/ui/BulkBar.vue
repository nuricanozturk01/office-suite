<script setup lang="ts">
import { Download, Trash2, X } from '@lucide/vue'
import { useI18n } from 'vue-i18n'

import { AppButton, InlineAlert } from '@/shared/ui'

defineProps<{ count: number; busy: boolean; failed: number }>()

const emit = defineEmits<{ download: []; delete: []; clear: [] }>()

const { t } = useI18n()
</script>

<template>
  <div class="flex flex-col gap-2">
    <div
      role="toolbar"
      :aria-label="t('files.bulk.label')"
      class="flex flex-wrap items-center gap-2 rounded-md bg-ink px-3 py-2 text-sm text-white"
    >
      <span class="me-auto font-medium" aria-live="polite">
        {{ t('files.bulk.selected', { count }) }}
      </span>
      <AppButton
        variant="ghost"
        :loading="busy"
        class="text-white hover:bg-white/10"
        @click="emit('download')"
      >
        <Download class="size-4" aria-hidden="true" />
        {{ t('fileActions.download') }}
      </AppButton>
      <AppButton
        variant="ghost"
        :loading="busy"
        class="text-white hover:bg-white/10"
        @click="emit('delete')"
      >
        <Trash2 class="size-4" aria-hidden="true" />
        {{ t('files.delete') }}
      </AppButton>
      <AppButton variant="ghost" class="text-white hover:bg-white/10" @click="emit('clear')">
        <X class="size-4" aria-hidden="true" />
        {{ t('files.bulk.clear') }}
      </AppButton>
    </div>
    <InlineAlert v-if="failed > 0">{{ t('files.bulk.failed', { count: failed }) }}</InlineAlert>
  </div>
</template>
