<script setup lang="ts">
import { useI18n } from 'vue-i18n'

import { AppButton, InlineAlert } from '@/shared/ui'
import { errorMessageKey } from '@/shared/api'

const { savedAt, canRestore, busy, error } = defineProps<{
  savedAt: Date
  canRestore: boolean
  busy: boolean
  error: unknown
}>()

defineEmits<{ restore: []; back: [] }>()

const { t, d } = useI18n()
</script>

<template>
  <div
    role="status"
    class="flex shrink-0 flex-wrap items-center gap-x-4 gap-y-2 border-b border-border bg-surface-muted px-4 py-2 text-sm"
  >
    <span class="min-w-0 flex-1">
      {{
        t('versionPreview.banner', {
          date: d(savedAt, { dateStyle: 'medium', timeStyle: 'short' }),
        })
      }}
    </span>
    <InlineAlert v-if="error">{{ t(errorMessageKey(error)) }}</InlineAlert>
    <AppButton variant="ghost" @click="$emit('back')">{{ t('versionPreview.back') }}</AppButton>
    <AppButton v-if="canRestore" :loading="busy" @click="$emit('restore')">
      {{ t('versionPreview.restore') }}
    </AppButton>
  </div>
</template>
