import type { JSONContent } from '@tiptap/core'
import { useEditor } from '@tiptap/vue-3'
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { userColor } from '@/shared/lib'

import { docsExtensions } from '../core/extensions'
import { useCollabDocument } from './use-collab-document'
import { usePageSetup } from './use-page-setup'
import { usePresence } from './use-presence'

export interface DocsEditorUser {
  id: string
  name: string
}

/** Collaborative Tiptap editor plus connection and presence state for one document. */
export function useDocsEditor(
  documentId: string,
  user: DocsEditorUser,
  getToken: () => string | null,
  initialContent?: JSONContent,
) {
  const { t } = useI18n()
  const { doc, provider, state } = useCollabDocument(documentId, getToken)
  const collaborators = usePresence(provider.awareness)
  const page = usePageSetup(doc)
  const pages = ref(1)

  const editor = useEditor({
    extensions: docsExtensions({
      doc,
      provider,
      user: { name: user.name, color: userColor(user.id) },
      placeholder: t('editor.placeholder'),
      geometry: () => page.geometry.value,
      headerFooter: () => page.setup.value,
      onPages: (count) => {
        pages.value = count
      },
    }),
    editorProps: {
      attributes: {
        role: 'textbox',
        'aria-multiline': 'true',
        'aria-label': t('editor.label'),
        class: 'docs-content',
        spellcheck: 'true',
      },
    },
  })

  watch(
    () => state.value.status === 'read-only',
    (readOnly) => {
      editor.value?.setEditable(!readOnly)
    },
  )

  // Applied once, and only to a document that has no content on the server yet.
  let pendingContent = initialContent
  watch(
    () => [state.value.synced, editor.value] as const,
    ([synced, current]) => {
      if (pendingContent !== undefined && synced && current?.isEmpty === true) {
        current.commands.setContent(pendingContent)
        pendingContent = undefined
      }
    },
    { immediate: true },
  )

  // Header, footer and page geometry changes need a fresh layout pass.
  watch(page.setup, () => {
    const view = editor.value?.view
    view?.dispatch(view.state.tr)
  })

  return { editor, state, collaborators, page, pages }
}
