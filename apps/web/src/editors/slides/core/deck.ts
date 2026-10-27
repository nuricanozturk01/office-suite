export type ShapeKind = 'rect' | 'roundRect' | 'ellipse' | 'triangle' | 'line' | 'arrow'
export type TextAlign = 'left' | 'center' | 'right'

export interface DeckElement {
  id: string
  kind: 'text' | 'shape'
  x: number
  y: number
  w: number
  h: number
  rotation?: number
  text?: string
  fontSize?: number
  bold?: boolean
  italic?: boolean
  underline?: boolean
  color?: string
  fill?: string
  stroke?: string
  strokeWidth?: number
  align?: TextAlign
  shape?: ShapeKind
}

export interface DeckSlide {
  id: string
  background?: string
  notes?: string
  elements: DeckElement[]
}

export interface DeckData {
  width: number
  height: number
  slides: DeckSlide[]
}

export const MAX_SLIDES = 200
export const MAX_ELEMENTS = 500
export const DEFAULT_FONT_SIZE = 24
export const DEFAULT_TEXT_COLOR = '#202124'
export const DEFAULT_FILL = '#4285f4'

const SHAPES: readonly string[] = ['rect', 'roundRect', 'ellipse', 'triangle', 'line', 'arrow']

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function num(value: unknown, fallback: number): number {
  return typeof value === 'number' && Number.isFinite(value) ? value : fallback
}

function color(value: unknown): string | undefined {
  return typeof value === 'string' && /^#[0-9a-f]{6}$/i.test(value) ? value : undefined
}

function parseElement(value: unknown): DeckElement | null {
  if (!isRecord(value) || typeof value.id !== 'string') {
    return null
  }
  const align = value.align
  return {
    id: value.id,
    kind: value.kind === 'text' ? 'text' : 'shape',
    x: num(value.x, 0),
    y: num(value.y, 0),
    w: Math.max(0, num(value.w, 100)),
    h: Math.max(0, num(value.h, 50)),
    ...(typeof value.rotation === 'number' ? { rotation: value.rotation } : {}),
    ...(typeof value.text === 'string' ? { text: value.text } : {}),
    ...(typeof value.fontSize === 'number' ? { fontSize: value.fontSize } : {}),
    ...(value.bold === true ? { bold: true } : {}),
    ...(value.italic === true ? { italic: true } : {}),
    ...(value.underline === true ? { underline: true } : {}),
    ...paint('color', value.color),
    ...paint('fill', value.fill),
    ...paint('stroke', value.stroke),
    ...(typeof value.strokeWidth === 'number' ? { strokeWidth: value.strokeWidth } : {}),
    ...(align === 'center' || align === 'right' || align === 'left' ? { align } : {}),
    ...(typeof value.shape === 'string' && SHAPES.includes(value.shape)
      ? { shape: value.shape as ShapeKind } // narrowed by the SHAPES membership check
      : {}),
  }
}

function paint(key: 'color' | 'fill' | 'stroke', value: unknown): Partial<DeckElement> {
  const parsed = color(value)
  return parsed === undefined ? {} : { [key]: parsed }
}

/** Reads a stored deck defensively; `null` when it has no usable slide list. */
export function parseDeck(content: unknown): DeckData | null {
  if (!isRecord(content) || !Array.isArray(content.slides) || content.slides.length === 0) {
    return null
  }
  const slides: DeckSlide[] = []
  for (const raw of content.slides as unknown[]) {
    if (!isRecord(raw) || typeof raw.id !== 'string' || !Array.isArray(raw.elements)) {
      return null
    }
    const slide: DeckSlide = {
      id: raw.id,
      elements: (raw.elements as unknown[])
        .map(parseElement)
        .filter((element): element is DeckElement => element !== null),
    }
    const background = color(raw.background)
    if (background !== undefined) {
      slide.background = background
    }
    if (typeof raw.notes === 'string' && raw.notes !== '') {
      slide.notes = raw.notes
    }
    slides.push(slide)
  }
  return { width: num(content.width, 960), height: num(content.height, 540), slides }
}

/** The JSON the server stores; the deck already has that shape. */
export function deckContent(deck: DeckData): Record<string, unknown> {
  return JSON.parse(JSON.stringify(deck)) as Record<string, unknown> // deep copy of plain data
}

export function newId(): string {
  return crypto.randomUUID()
}

/** A slide with an empty title and subtitle, like the first slide of a new deck. */
export function blankSlide(deck: DeckData): DeckSlide {
  const id = newId()
  return {
    id,
    elements: [
      {
        id: newId(),
        kind: 'text',
        x: deck.width * 0.08,
        y: deck.height * 0.31,
        w: deck.width * 0.84,
        h: 100,
        text: '',
        fontSize: 48,
        bold: true,
        align: 'center',
        color: DEFAULT_TEXT_COLOR,
      },
      {
        id: newId(),
        kind: 'text',
        x: deck.width * 0.17,
        y: deck.height * 0.54,
        w: deck.width * 0.66,
        h: 70,
        text: '',
        fontSize: 24,
        align: 'center',
        color: '#5f6368',
      },
    ],
  }
}

export function newElement(kind: 'text' | ShapeKind, deck: DeckData, offset = 0): DeckElement {
  const nudge = (offset % 8) * 24
  const cx = deck.width / 2 + nudge
  const cy = deck.height / 2 + nudge
  if (kind === 'text') {
    return {
      id: newId(),
      kind: 'text',
      x: cx - 200,
      y: cy - 30,
      w: 400,
      h: 60,
      text: '',
      fontSize: DEFAULT_FONT_SIZE,
      color: DEFAULT_TEXT_COLOR,
    }
  }
  const isLine = kind === 'line' || kind === 'arrow'
  return {
    id: newId(),
    kind: 'shape',
    shape: kind,
    x: cx - 100,
    y: cy - (isLine ? 0 : 60),
    w: 200,
    h: isLine ? 0 : 120,
    ...(isLine ? { stroke: '#202124', strokeWidth: 3 } : { fill: DEFAULT_FILL }),
  }
}
