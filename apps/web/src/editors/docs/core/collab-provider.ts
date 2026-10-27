import * as decoding from 'lib0/decoding'
import * as encoding from 'lib0/encoding'
import {
  Awareness,
  applyAwarenessUpdate,
  encodeAwarenessUpdate,
  removeAwarenessStates,
} from 'y-protocols/awareness'
import { messageYjsSyncStep1, messageYjsSyncStep2, messageYjsUpdate } from 'y-protocols/sync'
import * as Y from 'yjs'

const MESSAGE_SYNC = 0
const MESSAGE_AWARENESS = 1

/** Close codes sent by `modules/collaboration/core/src/api/ws.rs`. */
const CLOSE_POLICY = 1008

const MIN_RETRY_MS = 500
const MAX_RETRY_MS = 30_000

type Frame = Uint8Array<ArrayBuffer>

export type ConnectionStatus = 'connecting' | 'connected' | 'disconnected' | 'read-only'

export interface ProviderState {
  status: ConnectionStatus
  /** The server's state has been received since the last (re)connect. */
  synced: boolean
}

/** The subset of the browser `WebSocket` API the provider relies on. */
export interface SocketLike {
  binaryType: BinaryType
  readonly readyState: number
  onopen: ((event: Event) => void) | null
  onmessage: ((event: MessageEvent) => void) | null
  onclose: ((event: CloseEvent) => void) | null
  onerror: ((event: Event) => void) | null
  send(data: Frame): void
  close(code?: number, reason?: string): void
}

export interface CollabProviderOptions {
  /** Absolute `ws(s)://` URL of the document's collaboration endpoint. */
  url: string
  doc: Y.Doc
  /** Returns the current access token; sent as the `bearer, <token>` subprotocol pair. */
  getToken: () => string | null
  createSocket?: (url: string, protocols: string[]) => SocketLike
}

const OPEN = 1

/** An update that carries neither structs nor deletions. */
function isEmptyUpdate(update: Uint8Array): boolean {
  return update.length === 2 && update[0] === 0 && update[1] === 0
}

function syncFrame(type: number, payload: Uint8Array): Frame {
  const encoder = encoding.createEncoder()
  encoding.writeVarUint(encoder, MESSAGE_SYNC)
  encoding.writeVarUint(encoder, type)
  encoding.writeVarUint8Array(encoder, payload)
  return encoding.toUint8Array(encoder)
}

function awarenessFrame(update: Uint8Array): Frame {
  const encoder = encoding.createEncoder()
  encoding.writeVarUint(encoder, MESSAGE_AWARENESS)
  encoding.writeVarUint8Array(encoder, update)
  return encoding.toUint8Array(encoder)
}

/**
 * Yjs sync + awareness over the backend WebSocket protocol, with reconnects.
 * Framework-agnostic; the Vue adapter lives in `../vue`.
 */
export class CollabProvider {
  readonly awareness: Awareness
  private readonly options: Required<CollabProviderOptions>
  private socket: SocketLike | null = null
  private retryTimer: ReturnType<typeof setTimeout> | null = null
  private attempts = 0
  private destroyed = false
  private state: ProviderState = { status: 'connecting', synced: false }
  private readonly listeners = new Set<(state: ProviderState) => void>()

  constructor(options: CollabProviderOptions) {
    this.options = {
      createSocket: (url, protocols) => new WebSocket(url, protocols),
      ...options,
    }
    this.awareness = new Awareness(options.doc)
    options.doc.on('update', this.onDocUpdate)
    this.awareness.on('update', this.onAwarenessUpdate)
    this.connect()
  }

  get current(): ProviderState {
    return this.state
  }

  /** Calls `listener` on every state change; returns an unsubscribe function. */
  subscribe(listener: (state: ProviderState) => void): () => void {
    this.listeners.add(listener)
    return () => this.listeners.delete(listener)
  }

  destroy(): void {
    if (this.destroyed) {
      return
    }
    removeAwarenessStates(this.awareness, [this.options.doc.clientID], 'destroy')
    this.destroyed = true
    if (this.retryTimer !== null) {
      clearTimeout(this.retryTimer)
    }
    this.options.doc.off('update', this.onDocUpdate)
    this.awareness.off('update', this.onAwarenessUpdate)
    this.awareness.destroy()
    this.socket?.close(1000)
    this.socket = null
    this.listeners.clear()
  }

  private setState(next: Partial<ProviderState>): void {
    this.state = { ...this.state, ...next }
    for (const listener of this.listeners) {
      listener(this.state)
    }
  }

  private connect(): void {
    const token = this.options.getToken()
    if (token === null) {
      this.setState({ status: 'disconnected', synced: false })
      return
    }
    this.setState({ status: 'connecting', synced: false })
    const socket = this.options.createSocket(this.options.url, ['bearer', token])
    socket.binaryType = 'arraybuffer'
    socket.onopen = () => {
      this.attempts = 0
      this.setState({ status: 'connected' })
      this.send(syncFrame(messageYjsSyncStep1, Y.encodeStateVector(this.options.doc)))
      if (this.awareness.getLocalState() !== null) {
        this.send(
          awarenessFrame(encodeAwarenessUpdate(this.awareness, [this.options.doc.clientID])),
        )
      }
    }
    socket.onmessage = (event) => {
      if (event.data instanceof ArrayBuffer) {
        this.receive(new Uint8Array(event.data))
      }
    }
    socket.onclose = (event) => {
      this.onClose(socket, event.code)
    }
    socket.onerror = () => undefined
    this.socket = socket
  }

  private onClose(socket: SocketLike, code: number): void {
    if (this.socket !== socket) {
      return
    }
    this.socket = null
    const others = [...this.awareness.getStates().keys()].filter(
      (client) => client !== this.options.doc.clientID,
    )
    removeAwarenessStates(this.awareness, others, this)
    if (this.destroyed) {
      return
    }
    if (code === CLOSE_POLICY) {
      this.setState({ status: 'read-only', synced: false })
      return
    }
    this.setState({ status: 'disconnected', synced: false })
    const delay = Math.min(MAX_RETRY_MS, MIN_RETRY_MS * 2 ** this.attempts)
    this.attempts += 1
    this.retryTimer = setTimeout(
      () => {
        this.retryTimer = null
        this.connect()
      },
      delay * (0.5 + Math.random() / 2),
    )
  }

  private receive(frame: Uint8Array): void {
    const decoder = decoding.createDecoder(frame)
    const kind = decoding.readVarUint(decoder)
    if (kind === MESSAGE_AWARENESS) {
      applyAwarenessUpdate(this.awareness, decoding.readVarUint8Array(decoder), this)
      return
    }
    if (kind !== MESSAGE_SYNC) {
      return
    }
    const type = decoding.readVarUint(decoder)
    const payload = decoding.readVarUint8Array(decoder)
    switch (type) {
      case messageYjsSyncStep1: {
        // The server rejects updates from read-only sessions, so only send real changes.
        const missing = Y.encodeStateAsUpdate(this.options.doc, payload)
        if (!isEmptyUpdate(missing)) {
          this.send(syncFrame(messageYjsSyncStep2, missing))
        }
        break
      }
      case messageYjsSyncStep2:
        Y.applyUpdate(this.options.doc, payload, this)
        this.setState({ synced: true })
        break
      case messageYjsUpdate:
        Y.applyUpdate(this.options.doc, payload, this)
        break
      default:
        break
    }
  }

  private send(frame: Frame): void {
    if (this.socket?.readyState === OPEN) {
      this.socket.send(frame)
    }
  }

  private readonly onDocUpdate = (update: Uint8Array, origin: unknown): void => {
    if (origin !== this) {
      this.send(syncFrame(messageYjsUpdate, update))
    }
  }

  private readonly onAwarenessUpdate = (
    changes: { added: number[]; updated: number[]; removed: number[] },
    origin: unknown,
  ): void => {
    if (origin === this) {
      return
    }
    const own = this.options.doc.clientID
    const changed = [...changes.added, ...changes.updated, ...changes.removed]
    if (changed.includes(own)) {
      this.send(awarenessFrame(encodeAwarenessUpdate(this.awareness, [own])))
    }
  }
}
