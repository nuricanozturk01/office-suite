<script setup lang="ts">
import {
  AlignCenter,
  AlignJustify,
  AlignLeft,
  AlignRight,
  Baseline,
  Bold,
  CaseSensitive,
  ClipboardPaste,
  Copy,
  Highlighter,
  IndentDecrease,
  IndentIncrease,
  Italic,
  List,
  ListChecks,
  ListOrdered,
  MousePointer2,
  Paintbrush,
  Pilcrow,
  RemoveFormatting,
  Replace,
  Scissors,
  Search,
  Strikethrough,
  Subscript,
  Superscript,
  Underline,
  WrapText,
} from '@lucide/vue'
import type { Editor } from '@tiptap/core'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import {
  RibbonButton,
  RibbonColor,
  RibbonGroup,
  RibbonMenu,
  RibbonMenuItem,
  RibbonSelect,
} from '@/shared/ui'

import {
  currentAlignment,
  currentColor,
  currentFontFamily,
  currentFontSize,
  currentHighlight,
  indent,
  outdent,
  setAlignment,
  setFontFamily,
  setFontSize,
  setHighlight,
  setTextColor,
} from '../../core/commands'
import { COLOR_PALETTE, FONT_FAMILIES } from '../../core/formatting'
import {
  type CaseMode,
  changeCase,
  clearAllFormatting,
  copySelection,
  currentLineSpacing,
  pasteFromClipboard,
} from '../../core/ribbon-commands'
import { useDocsView } from '../docs-view'
import RibbonFontSize from './RibbonFontSize.vue'
import StylesGallery from './StylesGallery.vue'

/** `readOnly` comes from the connection state; `editor.isEditable` itself is not reactive. */
const { editor, readOnly } = defineProps<{ editor: Editor; readOnly: boolean }>()

const { t, locale } = useI18n()
const { findOpen, showMarks, painter } = useDocsView()

const fontFamily = computed(() => currentFontFamily(editor))
const fontSize = computed(() => currentFontSize(editor))
const color = computed(() => currentColor(editor))
const highlight = computed(() => currentHighlight(editor))
const alignment = computed(() => currentAlignment(editor))
const lineSpacing = computed(() => currentLineSpacing(editor))

const chain = () => editor.chain().focus()
const active = (name: string) => editor.isActive(name)

const CASES: readonly CaseMode[] = ['sentence', 'lower', 'upper', 'title', 'toggle']
const SPACINGS = [1, 1.15, 1.5, 2, 2.5, 3] as const

function togglePainter(): void {
  painter.value = painter.value === null ? editor.state.selection.$from.marks() : null
}

function indentBy(steps: 1 | -1): void {
  if (editor.isActive('listItem') || editor.isActive('taskItem')) {
    if (steps > 0) {
      indent(editor)
    } else {
      outdent(editor)
    }
  } else {
    chain().changeParagraphIndent(steps).run()
  }
}

function openFind(): void {
  findOpen.value = true
}
</script>

<template>
  <RibbonGroup :label="t('ribbon.group.clipboard')">
    <RibbonButton
      :icon="ClipboardPaste"
      :label="t('ribbon.paste')"
      size="large"
      :disabled="readOnly"
      @click="void pasteFromClipboard(editor)"
    />
    <div class="flex flex-col">
      <RibbonButton
        :icon="Scissors"
        :label="t('ribbon.cut')"
        size="small"
        :disabled="readOnly"
        @click="void copySelection(editor, true)"
      />
      <RibbonButton
        :icon="Copy"
        :label="t('ribbon.copy')"
        size="small"
        @click="void copySelection(editor, false)"
      />
      <RibbonButton
        :icon="Paintbrush"
        :label="t('ribbon.painter')"
        size="small"
        :active="painter !== null"
        :disabled="readOnly"
        @click="togglePainter"
      />
    </div>
  </RibbonGroup>

  <RibbonGroup :label="t('ribbon.group.font')">
    <div class="flex flex-col gap-1">
      <div class="flex items-center gap-1">
        <RibbonSelect
          class="w-36"
          :label="t('editor.actions.fontFamily')"
          :options="FONT_FAMILIES"
          :value="fontFamily as (typeof FONT_FAMILIES)[number]"
          :display="(font) => font"
          :option-style="(font) => ({ fontFamily: font })"
          :disabled="readOnly"
          @select="(font) => setFontFamily(editor, font)"
        />
        <RibbonFontSize :size="fontSize" @change="(points) => setFontSize(editor, points)" />
      </div>
      <div class="flex items-center gap-0.5">
        <RibbonButton
          :icon="Bold"
          :label="t('editor.actions.bold')"
          :active="active('bold')"
          :disabled="readOnly"
          @click="chain().toggleBold().run()"
        />
        <RibbonButton
          :icon="Italic"
          :label="t('editor.actions.italic')"
          :active="active('italic')"
          :disabled="readOnly"
          @click="chain().toggleItalic().run()"
        />
        <RibbonButton
          :icon="Underline"
          :label="t('editor.actions.underline')"
          :active="active('underline')"
          :disabled="readOnly"
          @click="chain().toggleUnderline().run()"
        />
        <RibbonButton
          :icon="Strikethrough"
          :label="t('editor.actions.strike')"
          :active="active('strike')"
          :disabled="readOnly"
          @click="chain().toggleStrike().run()"
        />
        <RibbonButton
          :icon="Subscript"
          :label="t('ribbon.subscript')"
          :active="active('subscript')"
          :disabled="readOnly"
          @click="chain().toggleSubscript().run()"
        />
        <RibbonButton
          :icon="Superscript"
          :label="t('ribbon.superscript')"
          :active="active('superscript')"
          :disabled="readOnly"
          @click="chain().toggleSuperscript().run()"
        />
        <RibbonMenu :icon="CaseSensitive" :label="t('ribbon.changeCase')" :disabled="readOnly">
          <RibbonMenuItem
            v-for="mode in CASES"
            :key="mode"
            :label="t(`ribbon.case.${mode}`)"
            @select="changeCase(editor, mode, locale)"
          />
        </RibbonMenu>
        <RibbonButton
          :icon="RemoveFormatting"
          :label="t('editor.actions.clearFormatting')"
          :disabled="readOnly"
          @click="clearAllFormatting(editor)"
        />
        <RibbonColor
          :icon="Baseline"
          :label="t('editor.actions.textColor')"
          :current="color"
          fallback="#000000"
          :palette="COLOR_PALETTE"
          :no-color-label="t('editor.actions.noColor')"
          :disabled="readOnly"
          @pick="(c) => setTextColor(editor, c)"
        />
        <RibbonColor
          :icon="Highlighter"
          :label="t('editor.actions.highlight')"
          :current="highlight"
          fallback="#ffff00"
          :palette="COLOR_PALETTE"
          :no-color-label="t('editor.actions.noColor')"
          :disabled="readOnly"
          @pick="(c) => setHighlight(editor, c)"
        />
      </div>
    </div>
  </RibbonGroup>

  <RibbonGroup :label="t('ribbon.group.paragraph')">
    <div class="flex flex-col gap-1">
      <div class="flex items-center gap-0.5">
        <RibbonButton
          :icon="List"
          :label="t('editor.actions.bulletList')"
          :active="active('bulletList')"
          :disabled="readOnly"
          @click="chain().toggleBulletList().run()"
        />
        <RibbonButton
          :icon="ListOrdered"
          :label="t('editor.actions.orderedList')"
          :active="active('orderedList')"
          :disabled="readOnly"
          @click="chain().toggleOrderedList().run()"
        />
        <RibbonButton
          :icon="ListChecks"
          :label="t('editor.actions.taskList')"
          :active="active('taskList')"
          :disabled="readOnly"
          @click="chain().toggleTaskList().run()"
        />
        <RibbonButton
          :icon="IndentDecrease"
          :label="t('editor.actions.outdent')"
          :disabled="readOnly"
          @click="indentBy(-1)"
        />
        <RibbonButton
          :icon="IndentIncrease"
          :label="t('editor.actions.indent')"
          :disabled="readOnly"
          @click="indentBy(1)"
        />
        <RibbonButton
          :icon="Pilcrow"
          :label="t('ribbon.showMarks')"
          :active="showMarks"
          @click="showMarks = !showMarks"
        />
      </div>
      <div class="flex items-center gap-0.5">
        <RibbonButton
          :icon="AlignLeft"
          :label="t('editor.actions.alignLeft')"
          :active="alignment === 'left'"
          :disabled="readOnly"
          @click="setAlignment(editor, 'left')"
        />
        <RibbonButton
          :icon="AlignCenter"
          :label="t('editor.actions.alignCenter')"
          :active="alignment === 'center'"
          :disabled="readOnly"
          @click="setAlignment(editor, 'center')"
        />
        <RibbonButton
          :icon="AlignRight"
          :label="t('editor.actions.alignRight')"
          :active="alignment === 'right'"
          :disabled="readOnly"
          @click="setAlignment(editor, 'right')"
        />
        <RibbonButton
          :icon="AlignJustify"
          :label="t('editor.actions.alignJustify')"
          :active="alignment === 'justify'"
          :disabled="readOnly"
          @click="setAlignment(editor, 'justify')"
        />
        <RibbonMenu :icon="WrapText" :label="t('ribbon.lineSpacing')" :disabled="readOnly">
          <RibbonMenuItem
            v-for="multiple in SPACINGS"
            :key="multiple"
            :label="String(multiple)"
            :checked="lineSpacing === multiple"
            @select="chain().setLineSpacing(multiple).run()"
          />
          <RibbonMenuItem
            :label="t('ribbon.spacingDefault')"
            @select="chain().setLineSpacing(null).run()"
          />
        </RibbonMenu>
      </div>
    </div>
  </RibbonGroup>

  <RibbonGroup :label="t('ribbon.group.styles')">
    <StylesGallery :editor="editor" />
  </RibbonGroup>

  <RibbonGroup :label="t('ribbon.group.editing')">
    <div class="flex flex-col">
      <RibbonButton :icon="Search" :label="t('ribbon.find')" size="small" @click="openFind" />
      <RibbonButton
        :icon="Replace"
        :label="t('ribbon.replace')"
        size="small"
        :disabled="readOnly"
        @click="openFind"
      />
      <RibbonButton
        :icon="MousePointer2"
        :label="t('ribbon.selectAll')"
        size="small"
        @click="chain().selectAll().run()"
      />
    </div>
  </RibbonGroup>
</template>
