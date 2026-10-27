import { describe, expect, it } from 'vitest'

import { deckContent, newElement, parseDeck } from './deck'

const stored = {
  width: 960,
  height: 540,
  slides: [
    {
      id: 's1',
      background: '#112233',
      notes: 'n',
      elements: [
        { id: 'e1', kind: 'text', x: 1, y: 2, w: 3, h: 4, text: 'hi', fontSize: 20, bold: true },
        { id: 'e2', kind: 'shape', shape: 'ellipse', x: 0, y: 0, w: 5, h: 5, fill: '#ff0000' },
      ],
    },
  ],
}

describe('parseDeck', () => {
  it('reads a stored deck and writes it back unchanged', () => {
    const deck = parseDeck(stored)
    expect(deck).not.toBeNull()
    expect(deck && deckContent(deck)).toEqual(stored)
  })

  it('rejects a deck without slides and drops invalid colors and shapes', () => {
    expect(parseDeck({ slides: [] })).toBeNull()
    expect(parseDeck('x')).toBeNull()
    const deck = parseDeck({
      slides: [{ id: 'a', elements: [{ id: 'b', kind: 'shape', shape: 'star', fill: 'red' }] }],
    })
    expect(deck?.slides[0]?.elements[0]).toMatchObject({ id: 'b', kind: 'shape' })
    expect(deck?.slides[0]?.elements[0]?.fill).toBeUndefined()
    expect(deck?.slides[0]?.elements[0]?.shape).toBeUndefined()
  })
})

describe('newElement', () => {
  it('centers a new text box on the slide', () => {
    const deck = parseDeck(stored)
    if (deck === null) {
      throw new Error('fixture must parse')
    }
    const element = newElement('text', deck)
    expect(element.kind).toBe('text')
    expect(element.x + element.w / 2).toBe(deck.width / 2)
  })
})
