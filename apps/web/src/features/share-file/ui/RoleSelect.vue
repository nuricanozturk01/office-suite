<script setup lang="ts">
import { useI18n } from 'vue-i18n'

import { type GrantableRole, grantableRoles, isGrantable } from '../model/use-share-file'

const { value, label } = defineProps<{ value: string; label: string }>()

const emit = defineEmits<{ change: [role: GrantableRole] }>()

const { t } = useI18n()

function onChange(event: Event): void {
  const target = event.target
  if (target instanceof HTMLSelectElement && isGrantable(target.value)) {
    emit('change', target.value)
  }
}
</script>

<template>
  <select
    :value="value"
    :aria-label="label"
    class="rounded-md border border-border bg-surface-raised px-2 py-1.5 text-sm"
    @change="onChange"
  >
    <option v-for="option in grantableRoles" :key="option" :value="option">
      {{ t(`share.roles.${option}`) }}
    </option>
  </select>
</template>
