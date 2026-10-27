<script setup lang="ts">
import type { JSONContent } from '@tiptap/core'
import { EditorContent } from '@tiptap/vue-3'
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { downloadDocx } from '../core/docx'
import { fileName, toHtmlDocument } from '../core/export'
import { pageAt } from '../core/pagination'
import ConnectionBadge from './chrome/ConnectionBadge.vue'
import FindReplaceBar from './chrome/FindReplaceBar.vue'
import OutlinePanel from './chrome/OutlinePanel.vue'
import PageRuler from './chrome/PageRuler.vue'
import PresenceAvatars from './chrome/PresenceAvatars.vue'
import StatusBar from './chrome/StatusBar.vue'
import HeaderFooterDialog from './dialogs/HeaderFooterDialog.vue'
import InsertImageDialog from './dialogs/InsertImageDialog.vue'
import InsertTableDialog from './dialogs/InsertTableDialog.vue'
import LinkDialog from './dialogs/LinkDialog.vue'
import MarginsDialog from './dialogs/MarginsDialog.vue'
import SymbolDialog from './dialogs/SymbolDialog.vue'
import WordCountDialog from './dialogs/WordCountDialog.vue'
import { provideDocsView } from './docs-view'
import DocsRibbon from './ribbon/DocsRibbon.vue'
import { type DocsEditorUser, useDocsEditor } from './use-docs-editor'
import './editor.css'

const { documentId, title, user, getToken, initialContent } = defineProps<{
  documentId: string
  /** Used for downloads; the editable title itself is rendered through the `title` slot. */
  title: string
  user: DocsEditorUser
  getToken: () => string | null
  /** Content converted from an uploaded file, applied once the server state has synced. */
  initialContent?: JSONContent | undefined
}>()

const emit = defineEmits<{ newDocument: [] }>()

defineSlots<{
  leading?: () => unknown
  title?: () => unknown
  actions?: () => unknown
}>()

const { t, locale } = useI18n()
const { editor, state, collaborators, page, pages } = useDocsEditor(
  documentId,
  user,
  getToken,
  initialContent,
)
const view = provideDocsView()
const { zoom, outlineOpen, linkOpen, dialog, rulerOpen, showMarks } = view

function print(): void {
  window.print()
}

function downloadHtml(): void {
  if (!editor.value) {
    return
  }
  const html = toHtmlDocument(title, editor.value.getHTML(), locale.value)
  const url = URL.createObjectURL(new Blob([html], { type: 'text/html;charset=utf-8' }))
  const link = document.createElement('a')
  link.href = url
  link.download = fileName(title, 'html', t('documents.untitled'))
  link.click()
  URL.revokeObjectURL(url)
}

const exportError = ref(false)

async function downloadWord(): Promise<void> {
  if (!editor.value) {
    return
  }
  exportError.value = false
  try {
    await downloadDocx(
      title,
      editor.value.getJSON(),
      fileName(title, 'docx', t('documents.untitled')),
    )
  } catch {
    exportError.value = true
  }
}

const currentPage = ref(1)

function updateCurrentPage(): void {
  const current = editor.value
  if (current) {
    currentPage.value = Math.min(
      pages.value,
      pageAt(current.view, current.state.selection.head, page.geometry.value),
    )
  }
}

watch(editor, (current, previous) => {
  previous?.off('selectionUpdate', updateCurrentPage)
  current?.on('selectionUpdate', updateCurrentPage)
})

const pageStyle = computed(() => {
  const g = page.geometry.value
  return {
    '--page-width': `${String(g.width)}px`,
    '--page-mt': `${String(g.marginTop)}px`,
    '--page-mr': `${String(g.marginRight)}px`,
    '--page-mb': `${String(g.marginBottom)}px`,
    '--page-ml': `${String(g.marginLeft)}px`,
  }
})

function dialogModel(name: NonNullable<typeof dialog.value>) {
  return computed({
    get: () => dialog.value === name,
    set: (open: boolean) => {
      dialog.value = open ? name : null
    },
  })
}
const imageOpen = dialogModel('image')
const tableOpen = dialogModel('table')
const wordCountOpen = dialogModel('wordCount')
const headerFooterOpen = dialogModel('headerFooter')
const symbolsOpen = dialogModel('symbols')
const marginsOpen = dialogModel('margins')

function onShortcut(event: KeyboardEvent): void {
  if (!(event.ctrlKey || event.metaKey) || event.altKey) {
    return
  }
  const key = event.key.toLowerCase()
  if (key === 'k' && !event.shiftKey && editor.value?.isEditable === true) {
    event.preventDefault()
    linkOpen.value = true
  } else if (key === 'c' && event.shiftKey) {
    event.preventDefault()
    dialog.value = 'wordCount'
  } else if ((key === 'f' || key === 'h') && !event.shiftKey) {
    event.preventDefault()
    view.findOpen.value = true
  }
}

onMounted(() => {
  window.addEventListener('keydown', onShortcut)
})
onBeforeUnmount(() => {
  window.removeEventListener('keydown', onShortcut)
})
</script>

<template>
  <div class="docs-editor flex h-dvh flex-col bg-surface-muted text-text">
    <header class="docs-chrome flex items-center gap-3 bg-surface px-3 pt-2">
      <slot name="leading" />
      <div class="flex min-w-0 flex-1 items-center gap-3">
        <slot name="title" />
        <ConnectionBadge :state="state" />
      </div>
      <PresenceAvatars :collaborators="collaborators" />
      <slot name="actions" />
    </header>
    <p v-if="exportError" class="bg-danger-surface px-4 py-2 text-sm text-danger" role="alert">
      {{ t('documents.exportError') }}
    </p>
    <div class="docs-chrome">
      <DocsRibbon
        v-if="editor"
        :editor="editor"
        :read-only="state.status === 'read-only'"
        :page="page"
        @new-document="emit('newDocument')"
        @download-docx="downloadWord"
        @download-html="downloadHtml"
        @print="print"
      />
    </div>
    <div class="docs-body flex min-h-0 flex-1" :class="{ 'docs-show-marks': showMarks }">
      <aside
        v-if="outlineOpen && editor"
        class="docs-chrome hidden w-60 shrink-0 overflow-y-auto lg:block"
      >
        <OutlinePanel :editor="editor" @close="outlineOpen = false" />
      </aside>
      <div class="relative flex min-w-0 flex-1 flex-col">
        <PageRuler
          v-if="rulerOpen"
          :geometry="page.geometry.value"
          class="mt-1"
          :style="{ zoom: zoom / 100 }"
        />
        <main id="main" class="docs-canvas min-h-0 flex-1 overflow-auto" tabindex="-1">
          <div class="docs-zoom py-4" :style="{ zoom: zoom / 100 }">
            <EditorContent v-if="editor" :editor="editor" class="docs-page" :style="pageStyle" />
          </div>
        </main>
        <FindReplaceBar v-if="editor" :editor="editor" :read-only="state.status === 'read-only'" />
      </div>
    </div>
    <StatusBar v-if="editor" :editor="editor" :pages="pages" :current-page="currentPage" />
    <template v-if="editor">
      <InsertImageDialog v-model:open="imageOpen" :editor="editor" />
      <InsertTableDialog v-model:open="tableOpen" :editor="editor" />
      <WordCountDialog v-model:open="wordCountOpen" :editor="editor" />
      <HeaderFooterDialog v-model:open="headerFooterOpen" :page="page" />
      <SymbolDialog v-model:open="symbolsOpen" :editor="editor" />
      <MarginsDialog v-model:open="marginsOpen" :page="page" />
      <LinkDialog v-model:open="linkOpen" :editor="editor" />
    </template>
  </div>
</template>
