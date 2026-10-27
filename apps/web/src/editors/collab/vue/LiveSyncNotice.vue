<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import type { Participant, SyncNotice } from '../core/ports'

const { notice, participants } = defineProps<{
  notice: SyncNotice
  /** Used to name who saved. */
  participants: Participant[]
}>()

defineEmits<{ takeTheirs: []; keepMine: []; dismiss: [] }>()

const { t } = useI18n()
const who = computed(() => {
  if (notice.kind !== 'updated' || notice.by === null) {
    return undefined
  }
  const by = notice.by
  return participants.find((participant) => participant.userId === by)?.displayName
})
</script>

<template>
  <p v-if="notice.kind === 'updated'" role="status" class="flex items-center gap-3 text-sm">
    <span>{{ who ? t('live.updatedBy', { name: who }) : t('live.updated') }}</span>
    <button
      type="button"
      class="rounded px-2 py-0.5 text-text-muted hover:bg-surface-muted"
      @click="$emit('dismiss')"
    >
      {{ t('live.dismiss') }}
    </button>
  </p>
  <p v-else-if="notice.kind === 'merged'" role="status" class="flex items-center gap-3 text-sm">
    <span>{{ t('live.merged') }}</span>
    <button
      type="button"
      class="rounded px-2 py-0.5 text-text-muted hover:bg-surface-muted"
      @click="$emit('dismiss')"
    >
      {{ t('live.dismiss') }}
    </button>
  </p>
  <div
    v-else-if="notice.kind === 'conflict'"
    role="alert"
    class="flex flex-wrap items-center gap-3 text-sm"
  >
    <span class="font-medium">{{ t('live.conflict') }}</span>
    <button
      type="button"
      class="rounded-md border border-border px-3 py-1 hover:bg-surface-muted"
      @click="$emit('takeTheirs')"
    >
      {{ t('live.takeTheirs') }}
    </button>
    <button
      type="button"
      class="rounded-md bg-accent px-3 py-1 text-accent-contrast hover:bg-accent-hover"
      @click="$emit('keepMine')"
    >
      {{ t('live.keepMine') }}
    </button>
  </div>
</template>
