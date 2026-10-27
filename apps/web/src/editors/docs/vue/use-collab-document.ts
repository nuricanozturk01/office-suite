import { markRaw, onScopeDispose, shallowRef } from 'vue'
import * as Y from 'yjs'

import { apiWebSocketOrigin } from '@/shared/api/origin'

import { CollabProvider } from '../core/collab-provider'

function collabUrl(documentId: string): string {
  return `${apiWebSocketOrigin()}/api/v1/collab/${encodeURIComponent(documentId)}`
}

/** One Yjs document bound to the backend for the lifetime of the calling scope. */
export function useCollabDocument(documentId: string, getToken: () => string | null) {
  const doc = markRaw(new Y.Doc())
  const provider = markRaw(new CollabProvider({ url: collabUrl(documentId), doc, getToken }))
  const state = shallowRef(provider.current)
  const unsubscribe = provider.subscribe((next) => {
    state.value = next
  })

  onScopeDispose(() => {
    unsubscribe()
    provider.destroy()
    doc.destroy()
  })

  return { doc, provider, state }
}
