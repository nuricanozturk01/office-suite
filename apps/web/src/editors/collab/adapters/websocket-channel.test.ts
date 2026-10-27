import { describe, expect, it } from 'vitest'

import type { LiveEvent } from '../core/ports'

import { parseServerMessage, type SocketLike, WebSocketLiveChannel } from './websocket-channel'

class FakeSocket implements SocketLike {
  onopen: ((event: Event) => void) | null = null
  onmessage: ((event: MessageEvent) => void) | null = null
  onclose: ((event: CloseEvent) => void) | null = null
  onerror: ((event: Event) => void) | null = null
  closed = false
  sent: string[] = []
  constructor(
    readonly url: string,
    readonly protocols: string[],
  ) {}
  close(): void {
    this.closed = true
  }
  send(data: string): void {
    this.sent.push(data)
  }
  receive(data: unknown): void {
    this.onmessage?.(new MessageEvent('message', { data }))
  }
  end(code: number): void {
    this.onclose?.(new CloseEvent('close', { code }))
  }
}

function setup(token: string | null = 'tok') {
  const sockets: FakeSocket[] = []
  const events: LiveEvent[] = []
  const channel = new WebSocketLiveChannel({
    url: 'ws://host/api/v1/livesync/r1',
    token: () => token,
    connect: (url, protocols) => {
      const socket = new FakeSocket(url, protocols)
      sockets.push(socket)
      return socket
    },
    retryDelayMs: () => 0,
  })
  channel.subscribe((event) => events.push(event))
  return { channel, sockets, events }
}

const wait = (): Promise<void> =>
  new Promise((resolve) => {
    setTimeout(resolve, 5)
  })

describe('parseServerMessage', () => {
  it('reads every message of the protocol', () => {
    const person = {
      session: 's1',
      user_id: 'u1',
      display_name: 'Ada',
      role: 'editor',
      editing: true,
    }
    expect(
      parseServerMessage(
        JSON.stringify({ type: 'welcome', session: 's0', participants: [person] }),
      ),
    ).toEqual({
      type: 'connected',
      session: 's0',
      participants: [
        { session: 's1', userId: 'u1', displayName: 'Ada', role: 'editor', editing: true },
      ],
    })
    expect(parseServerMessage(JSON.stringify({ type: 'joined', participant: person }))).toEqual({
      type: 'joined',
      participant: {
        session: 's1',
        userId: 'u1',
        displayName: 'Ada',
        role: 'editor',
        editing: true,
      },
    })
    expect(
      parseServerMessage(JSON.stringify({ type: 'activity', session: 's1', editing: true })),
    ).toEqual({ type: 'activity', session: 's1', editing: true })
    expect(parseServerMessage(JSON.stringify({ type: 'left', session: 's1' }))).toEqual({
      type: 'left',
      session: 's1',
    })
    expect(
      parseServerMessage(JSON.stringify({ type: 'saved', version: 4, by: 'u1', at: 'x' })),
    ).toEqual({ type: 'saved', version: 4, by: 'u1' })
  })

  it('ignores what it does not understand', () => {
    expect(parseServerMessage('not json')).toBeUndefined()
    expect(parseServerMessage(new ArrayBuffer(2))).toBeUndefined()
    expect(parseServerMessage(JSON.stringify({ type: 'nope' }))).toBeUndefined()
    expect(parseServerMessage(JSON.stringify({ type: 'saved' }))).toBeUndefined()
  })
})

describe('WebSocketLiveChannel', () => {
  it('connects with the bearer subprotocol and forwards messages', () => {
    const { channel, sockets, events } = setup()
    channel.open()
    expect(sockets[0]?.protocols).toEqual(['bearer', 'tok'])
    sockets[0]?.receive(JSON.stringify({ type: 'left', session: 's9' }))
    expect(events).toEqual([{ type: 'left', session: 's9' }])
  })

  it('tells the server when the user starts editing, also after a reconnect', async () => {
    const { channel, sockets } = setup()
    channel.open()
    channel.announce(true)
    expect(sockets[0]?.sent).toEqual([JSON.stringify({ type: 'activity', editing: true })])
    sockets[0]?.end(1006)
    await wait()
    sockets[1]?.onopen?.(new Event('open'))
    expect(sockets[1]?.sent).toEqual([JSON.stringify({ type: 'activity', editing: true })])
  })

  it('does not connect without a token', () => {
    const { channel, sockets } = setup(null)
    channel.open()
    expect(sockets).toHaveLength(0)
  })

  it('reconnects after the connection drops', async () => {
    const { channel, sockets, events } = setup()
    channel.open()
    sockets[0]?.end(1006)
    await wait()
    expect(events).toContainEqual({ type: 'disconnected' })
    expect(sockets).toHaveLength(2)
  })

  it('gives up when the server ends the connection over access', async () => {
    const { channel, sockets, events } = setup()
    channel.open()
    sockets[0]?.end(1008)
    await wait()
    expect(events).toContainEqual({ type: 'denied' })
    expect(sockets).toHaveLength(1)
  })

  it('stops for good once closed', async () => {
    const { channel, sockets } = setup()
    channel.open()
    channel.close()
    await wait()
    expect(sockets[0]?.closed).toBe(true)
    expect(sockets).toHaveLength(1)
  })
})
