/**
 * Live collaboration, independent of any editor. A [`LiveChannel`] tells the product who else has
 * a file open and when somebody saved a new version; a [`SyncTarget`] is whatever holds the
 * user's copy of the file (today an editor session, tomorrow any editor). The coordinator in
 * `coordinator.ts` only knows these two interfaces, so a new editor or a new transport plugs in
 * by implementing one of them.
 */

export type ParticipantRole = 'owner' | 'editor' | 'commenter' | 'viewer'

export interface Participant {
  /** One open connection; a user with two tabs is two participants. */
  session: string
  userId: string
  displayName: string
  /** What the person may do with the file. */
  role: ParticipantRole
  /** The person has unsaved edits right now. */
  editing: boolean
}

export type LiveEvent =
  | { type: 'connected'; session: string; participants: Participant[] }
  | { type: 'joined'; participant: Participant }
  | { type: 'left'; session: string }
  | { type: 'activity'; session: string; editing: boolean }
  /** A new version of the file was saved by anybody, including this user in another tab. */
  | { type: 'saved'; version: number; by: string | null }
  | { type: 'disconnected' }
  /** The server refused or ended the connection for good (access was removed). */
  | { type: 'denied' }

/** Driven port: the live connection to one file. */
export interface LiveChannel {
  open: () => void
  close: () => void
  /** Tells the others whether this user has unsaved edits right now. */
  announce: (editing: boolean) => void
  subscribe: (listener: (event: LiveEvent) => void) => () => void
}

/** Driven port: the user's copy of the file, as the coordinator needs to see and change it. */
export interface SyncTarget {
  /** Version of the stored file the copy is based on; `null` before the first save. */
  readonly version: number | null
  readonly hasUnsavedChanges: boolean
  /** A save is in flight; its own `saved` notification must not be mistaken for somebody else's. */
  readonly isSaving: boolean
  /** Replaces the copy with the latest stored file and adopts its version. */
  reload: () => Promise<void>
  /** Keeps the copy: the next save replaces whatever was stored in the meantime. */
  keepMine: () => Promise<void>
  /** Version of the latest stored file, or `null` when there is none. */
  latestStoredVersion: () => Promise<number | null>
}

/** What the user should be told about the other people's changes. */
export type SyncNotice =
  | { kind: 'none' }
  /** The copy was refreshed with somebody's changes. */
  | { kind: 'updated'; by: string | null }
  /** This copy's save was combined with changes others saved meanwhile. */
  | { kind: 'merged' }
  /** The two sets of changes touch the same part: the user chooses. */
  | { kind: 'conflict' }

export interface LiveState {
  connected: boolean
  /** Everybody else with the file open. */
  participants: Participant[]
  notice: SyncNotice
}
