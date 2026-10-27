import { describe, expect, it } from 'vitest'

import { LiveCoordinator } from './coordinator'
import type { LiveState, Participant, SyncTarget } from './ports'

class FakeTarget implements SyncTarget {
  version: number | null = 1
  hasUnsavedChanges = false
  isSaving = false
  reloads = 0
  kept = 0
  stored: number | null = 1

  async reload(): Promise<void> {
    this.reloads += 1
    this.version = this.stored
    await Promise.resolve()
  }

  async keepMine(): Promise<void> {
    this.kept += 1
    this.version = this.stored
    await Promise.resolve()
  }

  async latestStoredVersion(): Promise<number | null> {
    return await Promise.resolve(this.stored)
  }
}

const ada: Participant = {
  session: 's-ada',
  userId: 'u-ada',
  displayName: 'Ada',
  role: 'editor',
  editing: false,
}

function setup() {
  const target = new FakeTarget()
  const states: LiveState[] = []
  const coordinator = new LiveCoordinator(target, (state) => states.push(state))
  const last = (): LiveState => {
    const state = states.at(-1)
    if (state === undefined) {
      throw new Error('no state yet')
    }
    return state
  }
  return { target, coordinator, last }
}

describe('LiveCoordinator', () => {
  it('tracks who is here', () => {
    const { coordinator, last } = setup()
    coordinator.handle({ type: 'connected', session: 's-me', participants: [] })
    coordinator.handle({ type: 'joined', participant: ada })
    expect(last().participants).toEqual([ada])
    coordinator.handle({ type: 'left', session: ada.session })
    expect(last().participants).toEqual([])
    coordinator.handle({ type: 'disconnected' })
    expect(last().connected).toBe(false)
  })

  it('shows who is editing', () => {
    const { coordinator, last } = setup()
    coordinator.handle({ type: 'joined', participant: ada })
    coordinator.handle({ type: 'activity', session: ada.session, editing: true })
    expect(last().participants[0]?.editing).toBe(true)
    coordinator.handle({ type: 'activity', session: 'unknown', editing: true })
    expect(last().participants).toHaveLength(1)
  })

  it('reloads a clean copy when somebody else saves, and says so', async () => {
    const { target, coordinator, last } = setup()
    target.stored = 2
    coordinator.handle({ type: 'saved', version: 2, by: 'u-ada' })
    await Promise.resolve()
    await Promise.resolve()
    expect(target.reloads).toBe(1)
    expect(last().notice).toEqual({ kind: 'updated', by: 'u-ada' })
  })

  it('ignores a notification for the version it already has', () => {
    const { target, coordinator } = setup()
    coordinator.handle({ type: 'saved', version: 1, by: null })
    expect(target.reloads).toBe(0)
  })

  it('does not touch unsaved work and lets the user choose', async () => {
    const { target, coordinator, last } = setup()
    target.hasUnsavedChanges = true
    target.stored = 3
    coordinator.handle({ type: 'saved', version: 3, by: 'u-ada' })
    expect(target.reloads).toBe(0)
    expect(last().notice).toEqual({ kind: 'none' })

    // The pending save could not be merged: now the user chooses.
    coordinator.localSaveConflicted()
    expect(last().notice).toEqual({ kind: 'conflict' })

    await coordinator.keepMine()
    expect(target.kept).toBe(1)
    expect(last().notice).toEqual({ kind: 'none' })
  })

  it('takes their version when the user chooses so', async () => {
    const { target, coordinator, last } = setup()
    target.hasUnsavedChanges = true
    target.stored = 3
    coordinator.handle({ type: 'saved', version: 3, by: null })
    await coordinator.takeTheirs()
    expect(target.reloads).toBe(1)
    expect(last().notice.kind).toBe('updated')
  })

  it('waits for its own save before judging a notification that raced it', () => {
    const { target, coordinator, last } = setup()
    target.isSaving = true
    coordinator.handle({ type: 'saved', version: 2, by: 'u-me' })
    expect(target.reloads).toBe(0)

    // The save finishes with exactly that version: it was this copy's own.
    target.isSaving = false
    target.version = 2
    coordinator.localSaved(2)
    expect(target.reloads).toBe(0)
    expect(last().notice).toEqual({ kind: 'none' })
  })

  it("treats a newer notification that arrived during a save as somebody else's", async () => {
    const { target, coordinator } = setup()
    target.isSaving = true
    target.stored = 3
    coordinator.handle({ type: 'saved', version: 3, by: 'u-ada' })

    target.isSaving = false
    target.version = 2
    coordinator.localSaved(2)
    await Promise.resolve()
    await Promise.resolve()
    expect(target.reloads).toBe(1)
  })

  it('looks for missed saves after a reconnect', async () => {
    const { target, coordinator } = setup()
    coordinator.handle({ type: 'connected', session: 'a', participants: [] })
    coordinator.handle({ type: 'disconnected' })
    target.stored = 5
    coordinator.handle({ type: 'connected', session: 'b', participants: [] })
    await new Promise((resolve) => {
      setTimeout(resolve, 0)
    })
    expect(target.reloads).toBe(1)
  })

  it('reports a refused save as a conflict', () => {
    const { coordinator, last } = setup()
    coordinator.localSaveConflicted()
    expect(last().notice).toEqual({ kind: 'conflict' })
  })
})
