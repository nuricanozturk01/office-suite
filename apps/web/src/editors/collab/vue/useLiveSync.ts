import { onScopeDispose, ref } from 'vue'

import { LiveCoordinator } from '../core/coordinator'
import type { LiveChannel, LiveState, SyncTarget } from '../core/ports'

/** How long "the file was updated" stays up. */
const UPDATED_NOTICE_MS = 7000

const IDLE: LiveState = { connected: false, participants: [], notice: { kind: 'none' } }

/**
 * Connects one file's live channel to the user's copy of it. Editor-agnostic: it takes any
 * [`LiveChannel`] and [`SyncTarget`]; the caller says when the target exists ([`start`]) and
 * forwards the target's save results ([`localSaved`], [`localSaveConflicted`]).
 */
export function useLiveSync(createChannel: () => LiveChannel) {
  const state = ref<LiveState>(IDLE)
  let channel: LiveChannel | undefined
  let coordinator: LiveCoordinator | undefined
  let unsubscribe: (() => void) | undefined
  let timer: ReturnType<typeof setTimeout> | undefined

  function show(next: LiveState): void {
    state.value = next
    clearTimeout(timer)
    if (next.notice.kind === 'updated' || next.notice.kind === 'merged') {
      timer = setTimeout(() => coordinator?.dismiss(), UPDATED_NOTICE_MS)
    }
  }

  function start(target: SyncTarget): void {
    stop()
    coordinator = new LiveCoordinator(target, show)
    channel = createChannel()
    const live = coordinator
    unsubscribe = channel.subscribe((event) => {
      live.handle(event)
    })
    channel.open()
  }

  function stop(): void {
    clearTimeout(timer)
    unsubscribe?.()
    channel?.close()
    channel = undefined
    coordinator = undefined
    state.value = IDLE
  }

  onScopeDispose(stop)

  return {
    state,
    start,
    stop,
    localSaved: (version: number): void => {
      coordinator?.localSaved(version)
    },
    localMerged: (): void => {
      coordinator?.localMerged()
    },
    localSaveConflicted: (): void => {
      coordinator?.localSaveConflicted()
    },
    takeTheirs: async (): Promise<void> => {
      await coordinator?.takeTheirs()
    },
    keepMine: async (): Promise<void> => {
      await coordinator?.keepMine()
    },
    announce: (editing: boolean): void => {
      channel?.announce(editing)
    },
    dismiss: (): void => {
      coordinator?.dismiss()
    },
  }
}
