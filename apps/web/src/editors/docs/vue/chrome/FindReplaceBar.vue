<script setup lang="ts">
import { ChevronDown, ChevronUp, X } from '@lucide/vue'
import type { Editor } from '@tiptap/core'
import { computed, nextTick, ref, useTemplateRef, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { searchKey } from '../../core/search-replace'
import { useDocsView } from '../docs-view'

const { editor, readOnly } = defineProps<{ editor: Editor; readOnly: boolean }>()

const { t } = useI18n()
const { findOpen } = useDocsView()
const term = ref('')
const replacement = ref('')
const caseSensitive = ref(false)
const field = useTemplateRef<HTMLInputElement>('field')

const state = computed(() => searchKey.getState(editor.state))
const total = computed(() => state.value?.matches.length ?? 0)
const position = computed(() => (total.value === 0 ? 0 : (state.value?.current ?? 0) + 1))

function run(): void {
  editor.commands.setSearch(term.value, caseSensitive.value)
  void nextTick(reveal)
}

function reveal(): void {
  editor.view.dom.querySelector('.search-current')?.scrollIntoView({ block: 'center' })
}

function step(direction: 1 | -1): void {
  if (direction === 1) {
    editor.commands.findNext()
  } else {
    editor.commands.findPrevious()
  }
  void nextTick(reveal)
}

function close(): void {
  findOpen.value = false
  editor.commands.clearSearch()
  editor.commands.focus()
}

watch(findOpen, (open) => {
  if (open) {
    void nextTick(() => field.value?.focus())
  }
})

const input =
  'h-8 rounded border border-border bg-surface px-2 text-sm outline-none focus:ring-2 focus:ring-focus'
const button =
  'inline-flex size-8 items-center justify-center rounded hover:bg-surface-muted disabled:opacity-40'
</script>

<template>
  <div
    v-if="findOpen"
    role="search"
    :aria-label="t('ribbon.find')"
    class="absolute end-6 top-3 z-20 flex flex-col gap-2 rounded-lg border border-border bg-surface-raised p-3 shadow-lg"
  >
    <div class="flex items-center gap-1">
      <input
        ref="field"
        v-model="term"
        :class="input"
        :placeholder="t('ribbon.find')"
        :aria-label="t('ribbon.find')"
        @input="run"
        @keydown.enter.prevent="step($event.shiftKey ? -1 : 1)"
        @keydown.esc="close"
      />
      <span class="w-14 text-center text-xs text-text-muted" aria-live="polite">
        {{ position }} / {{ total }}
      </span>
      <button
        type="button"
        :class="button"
        :aria-label="t('ribbon.findPrevious')"
        :disabled="total === 0"
        @click="step(-1)"
      >
        <ChevronUp class="size-4" aria-hidden="true" />
      </button>
      <button
        type="button"
        :class="button"
        :aria-label="t('ribbon.findNext')"
        :disabled="total === 0"
        @click="step(1)"
      >
        <ChevronDown class="size-4" aria-hidden="true" />
      </button>
      <button type="button" :class="button" :aria-label="t('editor.close')" @click="close">
        <X class="size-4" aria-hidden="true" />
      </button>
    </div>
    <div v-if="!readOnly" class="flex items-center gap-1">
      <input
        v-model="replacement"
        :class="input"
        :placeholder="t('ribbon.replace')"
        :aria-label="t('ribbon.replace')"
        @keydown.esc="close"
      />
      <button
        type="button"
        class="rounded border border-border px-2 py-1 text-sm hover:bg-surface-muted disabled:opacity-40"
        :disabled="total === 0"
        @click="editor.commands.replaceCurrent(replacement)"
      >
        {{ t('ribbon.replaceOne') }}
      </button>
      <button
        type="button"
        class="rounded border border-border px-2 py-1 text-sm hover:bg-surface-muted disabled:opacity-40"
        :disabled="total === 0"
        @click="editor.commands.replaceAll(replacement)"
      >
        {{ t('ribbon.replaceAll') }}
      </button>
    </div>
    <label class="flex items-center gap-2 text-xs text-text-muted">
      <input v-model="caseSensitive" type="checkbox" @change="run" />
      {{ t('ribbon.matchCase') }}
    </label>
  </div>
</template>
