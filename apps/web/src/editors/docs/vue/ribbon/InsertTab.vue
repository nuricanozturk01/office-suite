<script setup lang="ts">
import {
  CalendarDays,
  FileDown,
  Image,
  Link,
  ListChecks,
  Minus,
  PanelBottom,
  PanelTop,
  Sigma,
  SeparatorHorizontal,
  Table2,
} from '@lucide/vue'
import type { Editor } from '@tiptap/core'
import { useI18n } from 'vue-i18n'

import { RibbonButton, RibbonGroup } from '@/shared/ui'

import { insertDate } from '../../core/ribbon-commands'
import { useDocsView } from '../docs-view'

const { editor, readOnly } = defineProps<{ editor: Editor; readOnly: boolean }>()

const { t, locale } = useI18n()
const { dialog, linkOpen } = useDocsView()
</script>

<template>
  <RibbonGroup :label="t('ribbon.group.pages')">
    <RibbonButton
      :icon="SeparatorHorizontal"
      :label="t('ribbon.pageBreak')"
      size="large"
      :disabled="readOnly"
      @click="editor.chain().focus().setPageBreak().run()"
    />
  </RibbonGroup>
  <RibbonGroup :label="t('ribbon.group.tables')">
    <RibbonButton
      :icon="Table2"
      :label="t('editor.actions.table')"
      size="large"
      :disabled="readOnly"
      @click="dialog = 'table'"
    />
  </RibbonGroup>
  <RibbonGroup :label="t('ribbon.group.illustrations')">
    <RibbonButton
      :icon="Image"
      :label="t('editor.actions.image')"
      size="large"
      :disabled="readOnly"
      @click="dialog = 'image'"
    />
  </RibbonGroup>
  <RibbonGroup :label="t('ribbon.group.links')">
    <RibbonButton
      :icon="Link"
      :label="t('editor.actions.link')"
      size="large"
      :disabled="readOnly"
      @click="linkOpen = true"
    />
  </RibbonGroup>
  <RibbonGroup :label="t('ribbon.group.headerFooter')">
    <RibbonButton
      :icon="PanelTop"
      :label="t('ribbon.header')"
      size="large"
      :disabled="readOnly"
      @click="dialog = 'headerFooter'"
    />
    <RibbonButton
      :icon="PanelBottom"
      :label="t('ribbon.footer')"
      size="large"
      :disabled="readOnly"
      @click="dialog = 'headerFooter'"
    />
    <RibbonButton
      :icon="FileDown"
      :label="t('ribbon.pageNumber')"
      size="large"
      :disabled="readOnly"
      @click="dialog = 'headerFooter'"
    />
  </RibbonGroup>
  <RibbonGroup :label="t('ribbon.group.text')">
    <RibbonButton
      :icon="Minus"
      :label="t('editor.actions.horizontalRule')"
      size="large"
      :disabled="readOnly"
      @click="editor.chain().focus().setHorizontalRule().run()"
    />
    <RibbonButton
      :icon="CalendarDays"
      :label="t('ribbon.date')"
      size="large"
      :disabled="readOnly"
      @click="insertDate(editor, locale)"
    />
    <RibbonButton
      :icon="ListChecks"
      :label="t('editor.actions.taskList')"
      size="large"
      :disabled="readOnly"
      @click="editor.chain().focus().toggleTaskList().run()"
    />
  </RibbonGroup>
  <RibbonGroup :label="t('ribbon.group.symbols')">
    <RibbonButton
      :icon="Sigma"
      :label="t('ribbon.symbol')"
      size="large"
      :disabled="readOnly"
      @click="dialog = 'symbols'"
    />
  </RibbonGroup>
</template>
