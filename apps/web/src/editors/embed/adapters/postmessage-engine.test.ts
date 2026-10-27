import { describe, expect, it } from 'vitest'

import type { EngineEvent } from '../core/ports'
import { PostMessageEngine } from './postmessage-engine'

function setup() {
  const frameWindow = {} as Window // identity only
  const frame = { contentWindow: frameWindow } as HTMLIFrameElement
  const target = new EventTarget()
  const engine = new PostMessageEngine({
    frame: () => frame,
    prefix: 'wordcraft',
    canvasId: 'wordcraft_canvas',
    origin: 'http://app.test',
    target,
  })
  const events: EngineEvent[] = []
  const stop = engine.subscribe((e) => events.push(e))
  const send = (data: unknown, source: unknown = frameWindow, origin = 'http://app.test') => {
    const event = new MessageEvent('message', { data, origin })
    Object.defineProperty(event, 'source', { value: source })
    target.dispatchEvent(event)
  }
  return { events, send, stop }
}

describe('PostMessageEngine', () => {
  it('translates the editor bridge messages', () => {
    const { events, send } = setup()
    send({ type: 'wordcraft:ready' })
    send({ type: 'wordcraft:dirty', dirty: true })
    send({ type: 'wordcraft:save', name: 'A.docx', bytes: new ArrayBuffer(2) })
    send({ type: 'wordcraft:command', command: 'file.new' })
    send({ type: 'wordcraft:command', command: 'file.saveAs' })
    send({ type: 'wordcraft:command', command: 'file.open' })
    send({ type: 'wordcraft:failed' })
    expect(events.map((e) => e.type)).toEqual([
      'ready',
      'dirty',
      'save',
      'command',
      'command',
      'command',
      'failed',
    ])
    expect(events[3]).toEqual({ type: 'command', command: 'new' })
  })

  it('ignores other origins, other windows, other apps and malformed messages', () => {
    const { events, send } = setup()
    send({ type: 'wordcraft:ready' }, {}, 'http://app.test')
    send({ type: 'wordcraft:ready' }, undefined, 'http://evil.test')
    send({ type: 'gridcraft:ready' })
    send({ type: 'wordcraft:save', name: 'A.docx' })
    send({ type: 'wordcraft:command', command: 'file.format' })
    send('text')
    expect(events).toEqual([])
  })

  it('stops after unsubscribe', () => {
    const { events, send, stop } = setup()
    stop()
    send({ type: 'wordcraft:ready' })
    expect(events).toEqual([])
  })
})
