<script setup lang="ts">
import { Cloud, CloudOff, Eye, LoaderCircle } from '@lucide/vue'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import type { ProviderState } from '../../core/collab-provider'

const { state } = defineProps<{ state: ProviderState }>()

const { t } = useI18n()

const view = computed(() => {
  switch (state.status) {
    case 'connected':
      return state.synced
        ? { label: t('editor.status.connected'), icon: Cloud, spin: false }
        : { label: t('editor.status.syncing'), icon: LoaderCircle, spin: true }
    case 'connecting':
      return { label: t('editor.status.connecting'), icon: LoaderCircle, spin: true }
    case 'read-only':
      return { label: t('editor.status.readOnly'), icon: Eye, spin: false }
    case 'disconnected':
      return { label: t('editor.status.disconnected'), icon: CloudOff, spin: false }
  }
})
</script>

<template>
  <p
    role="status"
    :title="view.label"
    class="flex items-center gap-1.5 text-xs text-text-muted"
    :class="{ 'text-danger': state.status === 'disconnected' }"
  >
    <component
      :is="view.icon"
      class="size-4"
      :class="{ 'animate-spin': view.spin }"
      aria-hidden="true"
    />
    <span class="sr-only lg:not-sr-only">{{ view.label }}</span>
  </p>
</template>
