<script setup lang="ts">
import { FileText, FileType, Printer } from '@lucide/vue'
import type { Editor } from '@tiptap/core'
import {
  DropdownMenuContent,
  DropdownMenuPortal,
  DropdownMenuRoot,
  DropdownMenuTrigger,
} from 'reka-ui'
import { onBeforeUnmount, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'

import { RibbonMenuItem, RibbonShell } from '@/shared/ui'

import type { PageSetupModel } from '../use-page-setup'
import { type RibbonTab, useDocsView } from '../docs-view'
import HomeTab from './HomeTab.vue'
import InsertTab from './InsertTab.vue'
import LayoutTab from './LayoutTab.vue'
import ReviewTab from './ReviewTab.vue'
import ViewTab from './ViewTab.vue'

const { editor, readOnly, page } = defineProps<{
  editor: Editor
  readOnly: boolean
  page: PageSetupModel
}>()

const emit = defineEmits<{ newDocument: []; downloadDocx: []; downloadHtml: []; print: [] }>()

const { t } = useI18n()
const { ribbonTab, painter } = useDocsView()

const tabs: { id: RibbonTab; label: string }[] = [
  { id: 'home', label: t('ribbon.tab.home') },
  { id: 'insert', label: t('ribbon.tab.insert') },
  { id: 'layout', label: t('ribbon.tab.layout') },
  { id: 'review', label: t('ribbon.tab.review') },
  { id: 'view', label: t('ribbon.tab.view') },
]

/** Format painter: the next selection takes the copied marks. */
function applyPainter(): void {
  const marks = painter.value
  if (marks === null || editor.state.selection.empty) {
    return
  }
  painter.value = null
  const chain = editor.chain().focus().unsetAllMarks()
  for (const mark of marks) {
    chain.setMark(mark.type.name, mark.attrs)
  }
  chain.run()
}

onMounted(() => {
  editor.on('selectionUpdate', applyPainter)
})
onBeforeUnmount(() => {
  editor.off('selectionUpdate', applyPainter)
})
</script>

<template>
  <RibbonShell v-model="ribbonTab" :tabs="tabs" :label="t('ribbon.label')">
    <template #leading>
      <DropdownMenuRoot :modal="false">
        <DropdownMenuTrigger
          class="rounded bg-accent px-3 py-1 text-sm font-medium text-accent-contrast outline-none hover:bg-accent-hover focus-visible:ring-2 focus-visible:ring-focus"
        >
          {{ t('editor.menu.file') }}
        </DropdownMenuTrigger>
        <DropdownMenuPortal>
          <DropdownMenuContent
            class="z-50 min-w-56 rounded-md border border-border bg-surface-raised py-1.5 text-sm shadow-lg"
            align="start"
            :side-offset="4"
          >
            <RibbonMenuItem
              :icon="FileText"
              :label="t('editor.actions.newDocument')"
              @select="emit('newDocument')"
            />
            <RibbonMenuItem
              :icon="FileType"
              :label="t('editor.actions.downloadDocx')"
              @select="emit('downloadDocx')"
            />
            <RibbonMenuItem
              :icon="FileType"
              :label="t('editor.actions.downloadHtml')"
              @select="emit('downloadHtml')"
            />
            <RibbonMenuItem
              :icon="Printer"
              :label="t('editor.actions.print')"
              @select="emit('print')"
            />
          </DropdownMenuContent>
        </DropdownMenuPortal>
      </DropdownMenuRoot>
    </template>
    <template #panel-home><HomeTab :editor="editor" :read-only="readOnly" /></template>
    <template #panel-insert><InsertTab :editor="editor" :read-only="readOnly" /></template>
    <template #panel-layout
      ><LayoutTab :editor="editor" :read-only="readOnly" :page="page"
    /></template>
    <template #panel-review><ReviewTab :editor="editor" :read-only="readOnly" /></template>
    <template #panel-view><ViewTab /></template>
  </RibbonShell>
</template>
