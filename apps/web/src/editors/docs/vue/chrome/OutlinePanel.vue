<script setup lang="ts">
import type { Editor } from '@tiptap/core'
import { X } from '@lucide/vue'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import { documentOutline, jumpTo } from '../../core/commands'

const { editor } = defineProps<{ editor: Editor }>()

const emit = defineEmits<{ close: [] }>()

const { t } = useI18n()

const entries = computed(() => documentOutline(editor))
</script>

<template>
  <nav :aria-label="t('editor.outline.title')" class="flex flex-col gap-2 px-4 py-6 text-sm">
    <div class="flex items-center justify-between">
      <h2 class="font-medium text-text-muted">{{ t('editor.outline.title') }}</h2>
      <button
        type="button"
        class="rounded-full p-1 text-text-muted hover:bg-surface"
        :aria-label="t('editor.outline.close')"
        @click="emit('close')"
      >
        <X class="size-4" aria-hidden="true" />
      </button>
    </div>
    <p v-if="entries.length === 0" class="text-xs text-text-muted">
      {{ t('editor.outline.empty') }}
    </p>
    <ul v-else class="flex flex-col">
      <li v-for="entry in entries" :key="entry.pos">
        <button
          type="button"
          class="w-full truncate rounded py-1 pe-2 text-start hover:bg-surface hover:text-accent"
          :class="entry.level === 1 ? 'font-medium' : 'text-text-muted'"
          :style="{ paddingInlineStart: `${String((entry.level - 1) * 0.75 + 0.5)}rem` }"
          :title="entry.text"
          @click="jumpTo(editor, entry.pos)"
        >
          {{ entry.text }}
        </button>
      </li>
    </ul>
  </nav>
</template>
