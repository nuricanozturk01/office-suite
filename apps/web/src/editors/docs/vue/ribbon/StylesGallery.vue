<script setup lang="ts">
import type { Editor } from '@tiptap/core'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import { applyStyle, currentStyle, STYLE_IDS, type StyleId } from '../../core/ribbon-commands'

const { editor } = defineProps<{ editor: Editor }>()

const { t } = useI18n()
const active = computed(() => currentStyle(editor))

/** How each style looks in its gallery tile (the page itself is styled in editor.css). */
const LOOK: Record<StyleId, Record<string, string>> = {
  normal: { fontSize: '11px' },
  noSpacing: { fontSize: '11px' },
  heading1: { fontSize: '17px', color: '#2f5496' },
  title: { fontSize: '22px' },
  heading2: { fontSize: '14px', color: '#2f5496' },
  subtitle: { fontSize: '11px', color: '#5f6368', letterSpacing: '0.06em' },
  heading3: { fontSize: '12px', color: '#1f3763' },
  quote: { fontSize: '11px', fontStyle: 'italic', color: '#5f6368' },
}
</script>

<template>
  <div
    role="listbox"
    :aria-label="t('ribbon.styles')"
    class="grid max-w-[27rem] auto-cols-[5.25rem] grid-flow-col grid-rows-1 gap-1 overflow-x-auto"
  >
    <button
      v-for="id in STYLE_IDS"
      :key="id"
      type="button"
      role="option"
      :aria-selected="active === id"
      class="flex h-[3.75rem] flex-col items-center justify-between rounded border bg-white px-1.5 py-1 text-black outline-none hover:border-accent focus-visible:ring-2 focus-visible:ring-focus"
      :class="active === id ? 'border-accent ring-1 ring-accent' : 'border-border'"
      @mousedown.prevent
      @click="applyStyle(editor, id)"
    >
      <span class="max-w-full truncate leading-tight" :style="LOOK[id]">AaBbCcDd</span>
      <span class="text-[10px] text-neutral-600">{{ t(`ribbon.style.${id}`) }}</span>
    </button>
  </div>
</template>
