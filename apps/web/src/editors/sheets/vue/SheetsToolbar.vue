<script setup lang="ts">
import {
  AlignCenter,
  AlignLeft,
  AlignRight,
  Baseline,
  Bold,
  Italic,
  PaintBucket,
  Redo2,
  Underline,
  Undo2,
} from '@lucide/vue'
import { ToolbarButton, ToolbarRoot, ToolbarSeparator } from 'reka-ui'
import { reactive } from 'vue'
import { useI18n } from 'vue-i18n'

import ColorMenu from './ColorMenu.vue'
import { useSheet } from './use-sheet-model'

const { t } = useI18n()
const model = reactive(useSheet())

const buttonClass =
  'inline-flex h-7 min-w-7 items-center justify-center rounded px-1 text-text outline-none hover:bg-surface-muted focus-visible:ring-2 focus-visible:ring-focus disabled:opacity-40 data-[state=on]:bg-accent/15 data-[state=on]:text-accent'

function onFormat(event: Event): void {
  if (!(event.target instanceof HTMLSelectElement)) {
    return
  }
  const value = event.target.value
  if (value === 'number' || value === 'percent' || value === 'currency') {
    model.setFormat(value)
    return
  }
  model.setFormat(null)
}

const formats = ['general', 'number', 'percent', 'currency'] as const
</script>

<template>
  <ToolbarRoot class="flex items-center gap-0.5" :aria-label="t('sheets.toolbar')">
    <ToolbarButton
      :class="buttonClass"
      :aria-label="t('editor.actions.undo')"
      :disabled="!model.canUndo"
      @mousedown.prevent
      @click="model.undo()"
    >
      <Undo2 class="size-4" aria-hidden="true" />
    </ToolbarButton>
    <ToolbarButton
      :class="buttonClass"
      :aria-label="t('editor.actions.redo')"
      :disabled="!model.canRedo"
      @mousedown.prevent
      @click="model.redo()"
    >
      <Redo2 class="size-4" aria-hidden="true" />
    </ToolbarButton>
    <ToolbarSeparator class="mx-1 h-5 w-px bg-border" />
    <ToolbarButton
      :class="buttonClass"
      :aria-label="t('editor.actions.bold')"
      :aria-pressed="model.focusCell?.bold === true"
      :data-state="model.focusCell?.bold === true ? 'on' : 'off'"
      @mousedown.prevent
      @click="model.toggleFlag('bold')"
    >
      <Bold class="size-4" aria-hidden="true" />
    </ToolbarButton>
    <ToolbarButton
      :class="buttonClass"
      :aria-label="t('editor.actions.italic')"
      :aria-pressed="model.focusCell?.italic === true"
      :data-state="model.focusCell?.italic === true ? 'on' : 'off'"
      @mousedown.prevent
      @click="model.toggleFlag('italic')"
    >
      <Italic class="size-4" aria-hidden="true" />
    </ToolbarButton>
    <ToolbarButton
      :class="buttonClass"
      :aria-label="t('editor.actions.underline')"
      :aria-pressed="model.focusCell?.underline === true"
      :data-state="model.focusCell?.underline === true ? 'on' : 'off'"
      @mousedown.prevent
      @click="model.toggleFlag('underline')"
    >
      <Underline class="size-4" aria-hidden="true" />
    </ToolbarButton>
    <ToolbarSeparator class="mx-1 h-5 w-px bg-border" />
    <ToolbarButton
      :class="buttonClass"
      :aria-label="t('editor.actions.alignLeft')"
      :aria-pressed="model.focusCell?.align === 'left'"
      :data-state="model.focusCell?.align === 'left' ? 'on' : 'off'"
      @mousedown.prevent
      @click="model.setAlign('left')"
    >
      <AlignLeft class="size-4" aria-hidden="true" />
    </ToolbarButton>
    <ToolbarButton
      :class="buttonClass"
      :aria-label="t('editor.actions.alignCenter')"
      :aria-pressed="model.focusCell?.align === 'center'"
      :data-state="model.focusCell?.align === 'center' ? 'on' : 'off'"
      @mousedown.prevent
      @click="model.setAlign('center')"
    >
      <AlignCenter class="size-4" aria-hidden="true" />
    </ToolbarButton>
    <ToolbarButton
      :class="buttonClass"
      :aria-label="t('editor.actions.alignRight')"
      :aria-pressed="model.focusCell?.align === 'right'"
      :data-state="model.focusCell?.align === 'right' ? 'on' : 'off'"
      @mousedown.prevent
      @click="model.setAlign('right')"
    >
      <AlignRight class="size-4" aria-hidden="true" />
    </ToolbarButton>
    <ColorMenu
      :label="t('editor.actions.textColor')"
      :icon="Baseline"
      :current="model.focusCell?.color"
      fallback="#202124"
      @pick="model.setPaint('color', $event)"
    />
    <ColorMenu
      :label="t('sheets.fillColor')"
      :icon="PaintBucket"
      :current="model.focusCell?.fill"
      fallback="#ffffff"
      @pick="model.setPaint('fill', $event)"
    />
    <ToolbarSeparator class="mx-1 h-5 w-px bg-border" />
    <label class="ms-1 flex items-center gap-2 text-sm">
      <span class="sr-only">{{ t('sheets.formats.label') }}</span>
      <select
        class="h-7 rounded border border-border bg-surface px-2"
        :value="model.focusCell?.format ?? ''"
        @change="onFormat"
      >
        <option v-for="format in formats" :key="format" :value="format === 'general' ? '' : format">
          {{ t(`sheets.formats.${format}`) }}
        </option>
      </select>
    </label>
  </ToolbarRoot>
</template>
