<script setup lang="ts">
import type { Editor } from '@tiptap/core'
import { useI18n } from 'vue-i18n'

import { insertSymbol } from '../../core/ribbon-commands'
import EditorDialog from './EditorDialog.vue'

const { editor } = defineProps<{ editor: Editor }>()
const open = defineModel<boolean>('open', { required: true })

const { t } = useI18n()
const SYMBOLS = Array.from(
  '©®™°±×÷≠≈≤≥∞√∑∏∫∂∆πΩµαβγδεθλσφψ←↑→↓↔⇒⇔€£¥¢§¶†‡•…–—«»‹›“”‘’¼½¾²³¹✓✗★☆♠♣♥♦☎✉',
)
</script>

<template>
  <EditorDialog v-model:open="open" :title="t('ribbon.symbol')">
    <div class="grid grid-cols-10 gap-1" role="group" :aria-label="t('ribbon.symbol')">
      <button
        v-for="symbol in SYMBOLS"
        :key="symbol"
        type="button"
        class="flex size-9 items-center justify-center rounded border border-border text-lg hover:bg-surface-muted focus-visible:ring-2 focus-visible:ring-focus"
        :aria-label="symbol"
        @click="insertSymbol(editor, symbol)"
      >
        {{ symbol }}
      </button>
    </div>
  </EditorDialog>
</template>
