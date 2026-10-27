import * as decoding from 'lib0/decoding'
import * as encoding from 'lib0/encoding'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { messageYjsSyncStep1, messageYjsSyncStep2, messageYjsUpdate } from 'y-protocols/sync'
import * as Y from 'yjs'

import { CollabProvider, type SocketLike } from './collab-provider'

function frame(type: number, payload: Uint8Array): ArrayBuffer {
  const encoder = encoding.createEncoder()
  encoding.writeVarUint(encoder, 0)
  encoding.writeVarUint(encoder, type)
  encoding.writeVarUint8Array(encoder, payload)
  return encoding.toUint8Array(encoder).slice().buffer
}

/** In-memory stand-in for the backend room: hello = step 1, replies step 2, applies updates. */
class FakeServer {
  readonly doc = new Y.Doc()
  readonly sockets: FakeSocket[] = []
  readonly received: { kind: number; type: number }[] = []
  readOnly = false

  handle(socket: FakeSocket, bytes: Uint8Array): void {
    const decoder = decoding.createDecoder(bytes)
    const kind = decoding.readVarUint(decoder)
    if (kind !== 0) {
      this.received.push({ kind, type: -1 })
      return
    }
    const type = decoding.readVarUint(decoder)
    const payload = decoding.readVarUint8Array(decoder)
    this.received.push({ kind, type })
    if (type === messageYjsSyncStep1) {
      socket.deliver(frame(messageYjsSyncStep2, Y.encodeStateAsUpdate(this.doc, payload)))
      return
    }
    if (this.readOnly) {
      socket.serverClose(1008)
      return
    }
    Y.applyUpdate(this.doc, payload)
  }
}

class FakeSocket implements SocketLike {
  binaryType: BinaryType = 'blob'
  readyState = 0
  onopen: ((event: Event) => void) | null = null
  onmessage: ((event: MessageEvent) => void) | null = null
  onclose: ((event: CloseEvent) => void) | null = null
  onerror: ((event: Event) => void) | null = null

  constructor(
    private readonly server: FakeServer,
    readonly protocols: string[],
  ) {}

  open(): void {
    this.readyState = 1
    this.onopen?.(new Event('open'))
    this.deliver(frame(messageYjsSyncStep1, Y.encodeStateVector(this.server.doc)))
  }

  deliver(data: ArrayBuffer): void {
    this.onmessage?.(new MessageEvent('message', { data }))
  }

  send(data: Uint8Array): void {
    this.server.handle(this, data)
  }

  serverClose(code: number): void {
    this.readyState = 3
    this.onclose?.(new CloseEvent('close', { code }))
  }

  close(): void {
    this.readyState = 3
  }
}

describe('CollabProvider', () => {
  let server: FakeServer
  let doc: Y.Doc
  let provider: CollabProvider | null

  function connect(): FakeSocket {
    provider = new CollabProvider({
      url: 'ws://test/api/v1/collab/doc',
      doc,
      getToken: () => 'tok',
      createSocket: (_url, protocols) => {
        const socket = new FakeSocket(server, protocols)
        server.sockets.push(socket)
        return socket
      },
    })
    const socket = server.sockets.at(-1)
    if (socket === undefined) {
      throw new Error('no socket created')
    }
    return socket
  }

  beforeEach(() => {
    vi.useFakeTimers()
    server = new FakeServer()
    doc = new Y.Doc()
    provider = null
  })

  afterEach(() => {
    provider?.destroy()
    vi.useRealTimers()
  })

  it('authenticates with the bearer subprotocol pair', () => {
    expect(connect().protocols).toEqual(['bearer', 'tok'])
  })

  it('pulls server content on connect and reports synced', () => {
    server.doc.getText('t').insert(0, 'hello')
    connect().open()

    expect(doc.getText('t').toJSON()).toBe('hello')
    expect(provider?.current).toEqual({ status: 'connected', synced: true })
  })

  it('does not answer the server step 1 when it has nothing new', () => {
    connect().open()

    const syncFrames = server.received.filter((message) => message.kind === 0)
    expect(syncFrames).toEqual([{ kind: 0, type: messageYjsSyncStep1 }])
  })

  it('streams local edits to the server', () => {
    connect().open()
    doc.getText('t').insert(0, 'abc')

    expect(server.doc.getText('t').toJSON()).toBe('abc')
    expect(server.received.at(-1)).toEqual({ kind: 0, type: messageYjsUpdate })
  })

  it('flushes offline edits after reconnecting', () => {
    const first = connect()
    first.open()
    first.serverClose(1006)
    expect(provider?.current.status).toBe('disconnected')

    doc.getText('t').insert(0, 'offline')
    vi.advanceTimersByTime(30_000)
    server.sockets.at(-1)?.open()

    expect(server.sockets).toHaveLength(2)
    expect(server.doc.getText('t').toJSON()).toBe('offline')
  })

  it('stops reconnecting when the server rejects writes', () => {
    server.readOnly = true
    connect().open()
    doc.getText('t').insert(0, 'x')

    expect(provider?.current.status).toBe('read-only')
    vi.advanceTimersByTime(60_000)
    expect(server.sockets).toHaveLength(1)
  })
})
