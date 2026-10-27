<script setup lang="ts">
import { PopoverContent, PopoverPortal, PopoverRoot, PopoverTrigger } from 'reka-ui'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import { userColor } from '@/shared/lib'

import type { Participant } from '../core/ports'

const { participants } = defineProps<{ participants: Participant[] }>()

const { t } = useI18n()
const MAX_SHOWN = 4

interface Person {
  userId: string
  displayName: string
  role: Participant['role']
  editing: boolean
}

/** One entry per person: a user with two tabs open is shown once, editing if either tab is. */
const people = computed<Person[]>(() => {
  const byUser = new Map<string, Person>()
  for (const participant of participants) {
    const known = byUser.get(participant.userId)
    byUser.set(participant.userId, {
      userId: participant.userId,
      displayName: participant.displayName,
      role: known?.role ?? participant.role,
      editing: (known?.editing ?? false) || participant.editing,
    })
  }
  return [...byUser.values()].sort((a, b) => Number(b.editing) - Number(a.editing))
})
const shown = computed(() => people.value.slice(0, MAX_SHOWN))
const hidden = computed(() => Math.max(0, people.value.length - MAX_SHOWN))
const editors = computed(() => people.value.filter((person) => person.editing))
const summary = computed(() => {
  const first = editors.value[0]
  if (first === undefined) {
    return t('live.here', { count: people.value.length })
  }
  return editors.value.length === 1
    ? t('live.editing', { name: first.displayName })
    : t('live.editingMany', { count: editors.value.length })
})

function initial(name: string): string {
  return name.trim().charAt(0).toLocaleUpperCase()
}
</script>

<template>
  <PopoverRoot v-if="people.length > 0">
    <PopoverTrigger
      class="flex items-center gap-2 rounded-full border border-border bg-surface-raised py-0.5 ps-1 pe-3 text-xs shadow-sm hover:bg-surface-muted"
      :aria-label="t('live.open')"
    >
      <span class="flex -space-x-1.5" aria-hidden="true">
        <span
          v-for="person in shown"
          :key="person.userId"
          class="relative grid size-5 place-items-center rounded-full border-2 border-surface-raised text-[0.625rem] font-medium text-white"
          :class="person.editing ? 'ring-2 ring-accent motion-safe:animate-pulse' : ''"
          :style="{ backgroundColor: userColor(person.userId) }"
        >
          {{ initial(person.displayName) }}
        </span>
      </span>
      <span v-if="hidden > 0" class="text-text-muted">{{ t('live.more', { count: hidden }) }}</span>
      <span class="max-w-56 truncate" role="status">{{ summary }}</span>
    </PopoverTrigger>
    <PopoverPortal>
      <PopoverContent
        side="top"
        :side-offset="8"
        class="z-50 w-72 rounded-lg border border-border bg-surface-raised p-2 text-sm text-text shadow-lg"
      >
        <p class="px-2 pt-1 pb-2 font-medium">{{ t('live.title') }}</p>
        <ul class="flex flex-col">
          <li
            v-for="person in people"
            :key="person.userId"
            class="flex items-center gap-3 rounded-md px-2 py-1.5"
          >
            <span
              class="grid size-7 shrink-0 place-items-center rounded-full text-xs font-medium text-white"
              :style="{ backgroundColor: userColor(person.userId) }"
              aria-hidden="true"
            >
              {{ initial(person.displayName) }}
            </span>
            <span class="min-w-0 flex-1">
              <span class="block truncate">{{ person.displayName }}</span>
              <span class="block text-xs text-text-muted">{{
                t(`share.roles.${person.role}`)
              }}</span>
            </span>
            <span
              v-if="person.editing"
              class="rounded-full bg-accent/12 px-2 py-0.5 text-xs text-accent"
            >
              {{ t('live.editingNow') }}
            </span>
          </li>
        </ul>
      </PopoverContent>
    </PopoverPortal>
  </PopoverRoot>
</template>
