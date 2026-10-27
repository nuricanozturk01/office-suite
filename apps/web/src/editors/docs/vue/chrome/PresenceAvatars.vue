<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import type { Collaborator } from '../use-presence'

const { collaborators } = defineProps<{ collaborators: readonly Collaborator[] }>()

const MAX_VISIBLE = 4

const { t } = useI18n()
const visible = computed(() => collaborators.slice(0, MAX_VISIBLE))
const hidden = computed(() => collaborators.length - visible.value.length)

function initial(name: string): string {
  return name.trim().charAt(0).toLocaleUpperCase()
}
</script>

<template>
  <ul
    v-if="collaborators.length > 0"
    :aria-label="t('editor.presence.label')"
    class="flex items-center -space-x-1.5"
  >
    <li
      v-for="person in visible"
      :key="person.clientId"
      :title="person.name"
      :style="{ backgroundColor: person.color }"
      class="grid size-8 place-items-center rounded-full text-sm font-medium text-white ring-2 ring-surface"
    >
      <span aria-hidden="true">{{ initial(person.name) }}</span>
      <span class="sr-only">{{ person.name }}</span>
    </li>
    <li
      v-if="hidden > 0"
      class="grid size-8 place-items-center rounded-full bg-surface-muted text-xs font-medium ring-2 ring-surface"
    >
      {{ t('editor.presence.more', { count: hidden }) }}
    </li>
  </ul>
</template>
