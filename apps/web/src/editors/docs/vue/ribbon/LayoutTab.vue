<script setup lang="ts">
import {
  Columns2,
  FileText,
  Maximize2,
  RectangleHorizontal,
  SeparatorHorizontal,
} from '@lucide/vue'
import type { Editor } from '@tiptap/core'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import { RibbonButton, RibbonGroup, RibbonMenu, RibbonMenuItem } from '@/shared/ui'

import { currentSpacing } from '../../core/ribbon-commands'
import { MARGIN_PRESETS, type PageSize, PAGE_SIZES } from '../../core/page-layout'
import { useDocsView } from '../docs-view'
import type { PageSetupModel } from '../use-page-setup'

const { editor, readOnly, page } = defineProps<{
  editor: Editor
  readOnly: boolean
  page: PageSetupModel
}>()

const { t } = useI18n()
const { dialog } = useDocsView()

const sizes = Object.keys(PAGE_SIZES) as PageSize[] // keys of PAGE_SIZES
const presets = Object.keys(MARGIN_PRESETS) as (keyof typeof MARGIN_PRESETS)[] // keys of MARGIN_PRESETS
const spacing = computed(() => currentSpacing(editor))

function setSpacing(side: 'before' | 'after', event: Event): void {
  const value = Number((event.target as HTMLInputElement).value)
  if (Number.isFinite(value) && value >= 0 && value <= 200) {
    editor
      .chain()
      .focus()
      .setParagraphSpacing({ [side]: value === 0 ? null : value })
      .run()
  }
}

function sameMargins(preset: keyof typeof MARGIN_PRESETS): boolean {
  const a = MARGIN_PRESETS[preset]
  const b = page.setup.value.margins
  return a.top === b.top && a.right === b.right && a.bottom === b.bottom && a.left === b.left
}
</script>

<template>
  <RibbonGroup :label="t('ribbon.group.pageSetup')">
    <RibbonMenu :icon="Maximize2" :label="t('ribbon.margins')" size="large" :disabled="readOnly">
      <RibbonMenuItem
        v-for="preset in presets"
        :key="preset"
        :label="t(`ribbon.marginPreset.${preset}`)"
        :checked="sameMargins(preset)"
        @select="page.setMargins(MARGIN_PRESETS[preset])"
      />
      <RibbonMenuItem :label="t('ribbon.customMargins')" @select="dialog = 'margins'" />
    </RibbonMenu>
    <RibbonMenu
      :icon="RectangleHorizontal"
      :label="t('ribbon.orientation')"
      size="large"
      :disabled="readOnly"
    >
      <RibbonMenuItem
        :label="t('ribbon.portrait')"
        :checked="page.setup.value.orientation === 'portrait'"
        @select="page.setOrientation('portrait')"
      />
      <RibbonMenuItem
        :label="t('ribbon.landscape')"
        :checked="page.setup.value.orientation === 'landscape'"
        @select="page.setOrientation('landscape')"
      />
    </RibbonMenu>
    <RibbonMenu :icon="FileText" :label="t('ribbon.size')" size="large" :disabled="readOnly">
      <RibbonMenuItem
        v-for="size in sizes"
        :key="size"
        :label="t(`ribbon.paper.${size}`)"
        :hint="`${String(PAGE_SIZES[size].width)} × ${String(PAGE_SIZES[size].height)} mm`"
        :checked="page.setup.value.size === size"
        @select="page.setSize(size)"
      />
    </RibbonMenu>
    <RibbonButton
      :icon="SeparatorHorizontal"
      :label="t('ribbon.pageBreak')"
      size="large"
      :disabled="readOnly"
      @click="editor.chain().focus().setPageBreak().run()"
    />
    <RibbonButton :icon="Columns2" :label="t('ribbon.columns')" size="large" disabled />
  </RibbonGroup>
  <RibbonGroup :label="t('ribbon.group.paragraph')">
    <div class="flex flex-col gap-1 text-sm">
      <label class="flex items-center justify-between gap-2">
        {{ t('ribbon.spaceBefore') }}
        <input
          type="number"
          min="0"
          max="200"
          step="3"
          class="h-7 w-16 rounded border border-border bg-surface px-1"
          :disabled="readOnly"
          :value="spacing.before"
          @change="setSpacing('before', $event)"
        />
      </label>
      <label class="flex items-center justify-between gap-2">
        {{ t('ribbon.spaceAfter') }}
        <input
          type="number"
          min="0"
          max="200"
          step="3"
          class="h-7 w-16 rounded border border-border bg-surface px-1"
          :disabled="readOnly"
          :value="spacing.after"
          @change="setSpacing('after', $event)"
        />
      </label>
    </div>
  </RibbonGroup>
</template>
