import { onScopeDispose, shallowRef } from 'vue'
import type { Awareness } from 'y-protocols/awareness'

export interface Collaborator {
  clientId: number
  name: string
  color: string
}

function toCollaborator(clientId: number, state: Record<string, unknown>): Collaborator | null {
  const user: unknown = state.user
  if (typeof user !== 'object' || user === null) {
    return null
  }
  if (!('name' in user) || !('color' in user)) {
    return null
  }
  const { name, color } = user
  return typeof name === 'string' && typeof color === 'string' ? { clientId, name, color } : null
}

/** Other people currently in the document, from Yjs awareness. */
export function usePresence(awareness: Awareness) {
  const others = shallowRef<Collaborator[]>([])

  function refresh(): void {
    const next: Collaborator[] = []
    awareness.getStates().forEach((state, clientId) => {
      const collaborator = clientId === awareness.clientID ? null : toCollaborator(clientId, state)
      if (collaborator !== null) {
        next.push(collaborator)
      }
    })
    others.value = next
  }

  awareness.on('change', refresh)
  refresh()
  onScopeDispose(() => {
    awareness.off('change', refresh)
  })

  return others
}
