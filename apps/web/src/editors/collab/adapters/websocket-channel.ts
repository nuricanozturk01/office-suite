import type { LiveChannel, LiveEvent, Participant, ParticipantRole } from '../core/ports'

/** The part of `WebSocket` the adapter uses, so tests can bring their own. */
export interface SocketLike {
  onopen: ((event: Event) => void) | null
  onmessage: ((event: MessageEvent) => void) | null
  onclose: ((event: CloseEvent) => void) | null
  onerror: ((event: Event) => void) | null
  close: () => void
  send: (data: string) => void
}

export interface WebSocketChannelOptions {
  /** Full `ws://` or `wss://` URL of the file's live channel. */
  url: string
  /** Access token; asked again on every (re)connect so a refreshed token is used. */
  token: () => string | null
  /** Creates the socket; the browser's `WebSocket` by default. */
  connect?: (url: string, protocols: string[]) => SocketLike
  /** Waits before a reconnect; grows with every failed attempt. */
  retryDelayMs?: (attempt: number) => number
}

/** Close code the server uses when the user's access ended. */
const POLICY_VIOLATION = 1008
const MAX_DELAY_MS = 15_000

function defaultDelay(attempt: number): number {
  return Math.min(MAX_DELAY_MS, 500 * 2 ** attempt)
}

function text(value: unknown): string | undefined {
  return typeof value === 'string' ? value : undefined
}

function asRecord(value: unknown): Record<string, unknown> | undefined {
  return typeof value === 'object' && value !== null
    ? (value as Record<string, unknown>)
    : undefined // narrowed to an object just above
}

const ROLES: readonly ParticipantRole[] = ['owner', 'editor', 'commenter', 'viewer']

function toRole(value: unknown): ParticipantRole {
  return ROLES.find((role) => role === value) ?? 'viewer'
}

function toParticipant(value: unknown): Participant | undefined {
  const record = asRecord(value)
  const session = text(record?.session)
  const userId = text(record?.user_id)
  const displayName = text(record?.display_name)
  return session !== undefined && userId !== undefined && displayName !== undefined
    ? {
        session,
        userId,
        displayName,
        role: toRole(record?.role),
        editing: record?.editing === true,
      }
    : undefined
}

/** Reads one JSON frame of the server's protocol; `undefined` for anything unknown. */
export function parseServerMessage(raw: unknown): LiveEvent | undefined {
  if (typeof raw !== 'string') {
    return undefined
  }
  let json: unknown
  try {
    json = JSON.parse(raw)
  } catch {
    return undefined
  }
  const message = asRecord(json)
  switch (message?.type) {
    case 'welcome': {
      const session = text(message.session)
      const list = Array.isArray(message.participants) ? (message.participants as unknown[]) : [] // checked by Array.isArray
      return session === undefined
        ? undefined
        : {
            type: 'connected',
            session,
            participants: list.flatMap((item) => toParticipant(item) ?? []),
          }
    }
    case 'joined': {
      const participant = toParticipant(message.participant)
      return participant === undefined ? undefined : { type: 'joined', participant }
    }
    case 'left': {
      const session = text(message.session)
      return session === undefined ? undefined : { type: 'left', session }
    }
    case 'activity': {
      const session = text(message.session)
      return session === undefined
        ? undefined
        : { type: 'activity', session, editing: message.editing === true }
    }
    case 'saved':
      return typeof message.version === 'number'
        ? { type: 'saved', version: message.version, by: text(message.by) ?? null }
        : undefined
    default:
      return undefined
  }
}

/**
 * Adapter: the server's live channel over a WebSocket. Reconnects with a growing delay until
 * closed, except when the server ends the connection because access was removed.
 */
export class WebSocketLiveChannel implements LiveChannel {
  private listeners = new Set<(event: LiveEvent) => void>()
  private socket: SocketLike | null = null
  private timer: ReturnType<typeof setTimeout> | undefined
  private attempt = 0
  private wanted = false
  private editing = false

  constructor(private readonly options: WebSocketChannelOptions) {}

  open(): void {
    this.wanted = true
    this.connect()
  }

  announce(editing: boolean): void {
    this.editing = editing
    this.socket?.send(JSON.stringify({ type: 'activity', editing }))
  }

  close(): void {
    this.wanted = false
    clearTimeout(this.timer)
    this.detach()
  }

  subscribe(listener: (event: LiveEvent) => void): () => void {
    this.listeners.add(listener)
    return () => {
      this.listeners.delete(listener)
    }
  }

  private emit(event: LiveEvent): void {
    for (const listener of this.listeners) {
      listener(event)
    }
  }

  private connect(): void {
    const token = this.options.token()
    if (!this.wanted || this.socket !== null || token === null) {
      return
    }
    const create =
      this.options.connect ??
      ((url: string, protocols: string[]): SocketLike => new WebSocket(url, protocols))
    const socket = create(this.options.url, ['bearer', token])
    this.socket = socket
    socket.onopen = () => {
      this.attempt = 0
      if (this.editing) {
        socket.send(JSON.stringify({ type: 'activity', editing: true }))
      }
    }
    socket.onmessage = (event) => {
      const parsed = parseServerMessage(event.data)
      if (parsed !== undefined) {
        this.emit(parsed)
      }
    }
    socket.onerror = () => undefined // `onclose` follows and handles it
    socket.onclose = (event) => {
      if (this.socket === socket) {
        this.socket = null
      }
      this.emit(event.code === POLICY_VIOLATION ? { type: 'denied' } : { type: 'disconnected' })
      if (this.wanted && event.code !== POLICY_VIOLATION) {
        const delay = (this.options.retryDelayMs ?? defaultDelay)(this.attempt)
        this.attempt += 1
        this.timer = setTimeout(() => {
          this.connect()
        }, delay)
      }
    }
  }

  private detach(): void {
    const socket = this.socket
    this.socket = null
    if (socket !== null) {
      socket.onopen = null
      socket.onmessage = null
      socket.onclose = null
      socket.onerror = null
      socket.close()
    }
  }
}
