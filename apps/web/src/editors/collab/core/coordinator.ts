import type { LiveEvent, LiveState, Participant, SyncNotice, SyncTarget } from './ports'

/**
 * Keeps one user's copy of a file in step with what others save, and tracks who is here. Pure
 * application logic over the ports: no DOM, no network.
 *
 * Rules for a `saved` notification newer than the copy:
 *  - no unsaved changes here: the copy is reloaded (several notifications in a row are
 *    coalesced into one reload);
 *  - unsaved changes here: nothing is touched; the pending save is combined with the newer
 *    version by the server ([`localMerged`]), and only when it cannot be (the same part was
 *    changed on both sides, or the format cannot be merged) the user chooses
 *    ([`takeTheirs`], [`keepMine`]).
 */
export class LiveCoordinator {
  private participants = new Map<string, Participant>()
  private connected = false
  private notice: SyncNotice = { kind: 'none' }
  /** Newest version somebody reported that the copy does not have yet. */
  private newest = 0
  private refreshing = false
  private everConnected = false

  constructor(
    private readonly target: SyncTarget,
    private readonly onChange: (state: LiveState) => void,
  ) {}

  handle(event: LiveEvent): void {
    switch (event.type) {
      case 'connected':
        this.participants = new Map(event.participants.map((p) => [p.session, p]))
        this.connected = true
        // Saves made while the connection was down were never announced: look for them.
        if (this.everConnected) {
          void this.catchUp()
        }
        this.everConnected = true
        break
      case 'joined':
        this.participants.set(event.participant.session, event.participant)
        break
      case 'left':
        this.participants.delete(event.session)
        break
      case 'activity': {
        const person = this.participants.get(event.session)
        if (person !== undefined) {
          this.participants.set(event.session, { ...person, editing: event.editing })
        }
        break
      }
      case 'saved':
        this.remoteSaved(event.version, event.by)
        break
      case 'disconnected':
      case 'denied':
        this.connected = false
        this.participants.clear()
        break
    }
    this.emit()
  }

  /** The editor stored its own version (a save finished). */
  localSaved(version: number): void {
    if (this.newest <= version) {
      this.newest = 0
      if (this.notice.kind === 'conflict') {
        this.notice = { kind: 'none' }
      }
    } else {
      // Somebody saved after us.
      this.remoteSaved(this.newest, null)
    }
    this.emit()
  }

  /** A save was combined with changes others had saved; the copy shows the result. */
  localMerged(): void {
    this.notice = { kind: 'merged' }
    this.emit()
  }

  /** The store refused a save because the file changed meanwhile. */
  localSaveConflicted(): void {
    this.notice = { kind: 'conflict' }
    this.emit()
  }

  /** Conflict choice: drop this copy's unsaved changes and load what was saved. */
  async takeTheirs(): Promise<void> {
    await this.refresh(null)
  }

  /** Conflict choice: keep this copy; its next save replaces the newer version. */
  async keepMine(): Promise<void> {
    await this.target.keepMine()
    this.newest = 0
    this.notice = { kind: 'none' }
    this.emit()
  }

  dismiss(): void {
    if (this.notice.kind === 'updated' || this.notice.kind === 'merged') {
      this.notice = { kind: 'none' }
      this.emit()
    }
  }

  private remoteSaved(version: number, by: string | null): void {
    if (version <= (this.target.version ?? 0)) {
      return // this copy's own save, or an old one
    }
    this.newest = Math.max(this.newest, version)
    if (this.target.isSaving) {
      return // decided in `localSaved`, once the save's own version is known
    }
    if (this.target.hasUnsavedChanges) {
      return // the save of those changes merges, or reports the conflict
    }
    void this.refresh(by)
  }

  private async catchUp(): Promise<void> {
    const latest = await this.target.latestStoredVersion().catch(() => null)
    if (latest !== null) {
      this.remoteSaved(latest, null)
      this.emit()
    }
  }

  private async refresh(by: string | null): Promise<void> {
    if (this.refreshing) {
      return // the running reload loops again when something newer arrived
    }
    this.refreshing = true
    try {
      do {
        await this.target.reload()
      } while (this.newest > (this.target.version ?? 0))
      this.newest = 0
      this.notice = { kind: 'updated', by }
    } catch {
      this.notice = { kind: 'conflict' }
    } finally {
      this.refreshing = false
      this.emit()
    }
  }

  private emit(): void {
    this.onChange({
      connected: this.connected,
      participants: [...this.participants.values()],
      notice: this.notice,
    })
  }
}
