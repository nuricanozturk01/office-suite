import { useQueryClient } from '@tanstack/vue-query'
import { computed, type InjectionKey, inject, onBeforeUnmount, provide, ref } from 'vue'

import { presentationKeys, type PresentationDetail } from '@/entities/presentation'
import { api, ApiError, unwrap } from '@/shared/api'

import {
  blankSlide,
  deckContent,
  type DeckData,
  type DeckElement,
  type DeckSlide,
  MAX_ELEMENTS,
  MAX_SLIDES,
  newElement,
  newId,
  parseDeck,
  type ShapeKind,
} from '../core/deck'

export type SaveStatus = 'saved' | 'saving' | 'error' | 'conflict'

const HISTORY_LIMIT = 100

function clone(deck: DeckData): DeckData {
  return parseDeck(deckContent(deck)) ?? deck
}

/**
 * Deck state for one presentation: selection, undo and debounced saves. The parent remounts
 * this after a reload, so `initial` is whatever the server last stored.
 */
export function useDeckModel(id: string, initial: DeckData, initialVersion: number) {
  const queryClient = useQueryClient()
  const deck = ref<DeckData>(clone(initial))
  const activeIndex = ref(0)
  const selectedId = ref<string | null>(null)
  const version = ref(initialVersion)
  const status = ref<SaveStatus>('saved')
  const past = ref<DeckData[]>([])
  const future = ref<DeckData[]>([])
  const dirty = ref(false)
  let timer = 0
  let flushing: Promise<unknown> | null = null

  const slide = computed<DeckSlide>(() => deck.value.slides[activeIndex.value] ?? emptySlide())
  const selected = computed(() => slide.value.elements.find((e) => e.id === selectedId.value))

  function emptySlide(): DeckSlide {
    return { id: 'none', elements: [] }
  }

  function scheduleSave(): void {
    dirty.value = true
    if (status.value !== 'conflict') {
      status.value = 'saving'
    }
    window.clearTimeout(timer)
    timer = window.setTimeout(() => {
      void flush()
    }, 600)
  }

  async function flush(): Promise<void> {
    if (flushing) {
      await flushing
      if (dirty.value) {
        await flush()
      }
      return
    }
    if (!dirty.value || status.value === 'conflict') {
      return
    }
    dirty.value = false
    const run = send(deckContent(deck.value)).finally(() => {
      flushing = null
    })
    flushing = run
    if (await run) {
      await flush()
    }
  }

  async function send(content: Record<string, unknown>): Promise<boolean> {
    try {
      const saved = await unwrap(
        api.PUT('/api/v1/presentations/{id}/content', {
          params: { path: { id } },
          body: { version: version.value, content },
        }),
      )
      version.value = saved.version
      queryClient.setQueryData<PresentationDetail>(presentationKeys.detail(id), (current) =>
        current === undefined ? current : { ...current, version: saved.version, content },
      )
      status.value = dirty.value ? 'saving' : 'saved'
      return dirty.value
    } catch (error) {
      if (error instanceof ApiError && error.code === 'version_conflict') {
        status.value = 'conflict'
        return false
      }
      dirty.value = true
      status.value = 'error'
      return false
    }
  }

  onBeforeUnmount(() => {
    window.clearTimeout(timer)
    if (dirty.value) {
      void flush()
    }
  })

  /** Remembers the current deck for undo; call once before a gesture that changes it. */
  function snapshot(): void {
    past.value.push(clone(deck.value))
    if (past.value.length > HISTORY_LIMIT) {
      past.value.shift()
    }
    future.value = []
  }

  function replaceSlide(next: DeckSlide): void {
    deck.value = {
      ...deck.value,
      slides: deck.value.slides.map((s, i) => (i === activeIndex.value ? next : s)),
    }
    scheduleSave()
  }

  function addElement(kind: 'text' | ShapeKind): void {
    if (slide.value.elements.length >= MAX_ELEMENTS) {
      return
    }
    snapshot()
    const element = newElement(kind, deck.value, slide.value.elements.length)
    replaceSlide({ ...slide.value, elements: [...slide.value.elements, element] })
    selectedId.value = element.id
  }

  /** Patches one element. Pass `record: false` while a drag is already snapshotted. */
  function updateElement(elementId: string, patch: Partial<DeckElement>, record = true): void {
    if (record) {
      snapshot()
    }
    replaceSlide({
      ...slide.value,
      elements: slide.value.elements.map((e) => (e.id === elementId ? { ...e, ...patch } : e)),
    })
  }

  function deleteSelected(): void {
    if (selectedId.value === null) {
      return
    }
    snapshot()
    const removed = selectedId.value
    replaceSlide({ ...slide.value, elements: slide.value.elements.filter((e) => e.id !== removed) })
    selectedId.value = null
  }

  function duplicateSelected(): void {
    const source = selected.value
    if (source === undefined || slide.value.elements.length >= MAX_ELEMENTS) {
      return
    }
    snapshot()
    const copy = { ...source, id: newId(), x: source.x + 20, y: source.y + 20 }
    replaceSlide({ ...slide.value, elements: [...slide.value.elements, copy] })
    selectedId.value = copy.id
  }

  function reorder(toFront: boolean): void {
    const source = selected.value
    if (source === undefined) {
      return
    }
    snapshot()
    const others = slide.value.elements.filter((e) => e.id !== source.id)
    replaceSlide({
      ...slide.value,
      elements: toFront ? [...others, source] : [source, ...others],
    })
  }

  function setBackground(background: string | undefined): void {
    snapshot()
    const next = { ...slide.value }
    delete next.background
    replaceSlide(background === undefined ? next : { ...next, background })
  }

  function setNotes(notes: string): void {
    const next = { ...slide.value }
    delete next.notes
    replaceSlide(notes === '' ? next : { ...next, notes })
  }

  function addSlide(): void {
    if (deck.value.slides.length >= MAX_SLIDES) {
      return
    }
    snapshot()
    const slides = [...deck.value.slides]
    slides.splice(activeIndex.value + 1, 0, blankSlide(deck.value))
    deck.value = { ...deck.value, slides }
    activeIndex.value += 1
    selectedId.value = null
    scheduleSave()
  }

  function duplicateSlide(): void {
    if (deck.value.slides.length >= MAX_SLIDES) {
      return
    }
    snapshot()
    const copy: DeckSlide = {
      ...slide.value,
      id: newId(),
      elements: slide.value.elements.map((e) => ({ ...e, id: newId() })),
    }
    const slides = [...deck.value.slides]
    slides.splice(activeIndex.value + 1, 0, copy)
    deck.value = { ...deck.value, slides }
    activeIndex.value += 1
    selectedId.value = null
    scheduleSave()
  }

  function deleteSlide(): void {
    if (deck.value.slides.length < 2) {
      return
    }
    snapshot()
    deck.value = {
      ...deck.value,
      slides: deck.value.slides.filter((_, i) => i !== activeIndex.value),
    }
    activeIndex.value = Math.min(activeIndex.value, deck.value.slides.length - 1)
    selectedId.value = null
    scheduleSave()
  }

  function moveSlide(delta: -1 | 1): void {
    const from = activeIndex.value
    const to = from + delta
    if (to < 0 || to >= deck.value.slides.length) {
      return
    }
    snapshot()
    const slides = [...deck.value.slides]
    const [moved] = slides.splice(from, 1)
    if (moved !== undefined) {
      slides.splice(to, 0, moved)
    }
    deck.value = { ...deck.value, slides }
    activeIndex.value = to
    scheduleSave()
  }

  function chooseSlide(index: number): void {
    activeIndex.value = Math.min(deck.value.slides.length - 1, Math.max(0, index))
    selectedId.value = null
  }

  function undo(): void {
    const previous = past.value.pop()
    if (previous === undefined) {
      return
    }
    future.value.push(clone(deck.value))
    deck.value = previous
    activeIndex.value = Math.min(activeIndex.value, previous.slides.length - 1)
    selectedId.value = null
    scheduleSave()
  }

  function redo(): void {
    const next = future.value.pop()
    if (next === undefined) {
      return
    }
    past.value.push(clone(deck.value))
    deck.value = next
    activeIndex.value = Math.min(activeIndex.value, next.slides.length - 1)
    selectedId.value = null
    scheduleSave()
  }

  return {
    deck,
    slide,
    activeIndex,
    selectedId,
    selected,
    status,
    snapshot,
    addElement,
    updateElement,
    deleteSelected,
    duplicateSelected,
    bringToFront: () => {
      reorder(true)
    },
    sendToBack: () => {
      reorder(false)
    },
    setBackground,
    setNotes,
    addSlide,
    duplicateSlide,
    deleteSlide,
    moveSlide,
    chooseSlide,
    undo,
    redo,
    canUndo: computed(() => past.value.length > 0),
    canRedo: computed(() => future.value.length > 0),
    retrySave: () => {
      if (status.value === 'error') {
        scheduleSave()
      }
    },
  }
}

export type DeckModel = ReturnType<typeof useDeckModel>

const DECK_MODEL: InjectionKey<DeckModel> = Symbol('deck-model')

export function provideDeckModel(model: DeckModel): void {
  provide(DECK_MODEL, model)
}

export function useDeck(): DeckModel {
  const model = inject(DECK_MODEL)
  if (model === undefined) {
    throw new Error('useDeck() must be used inside SlidesEditor')
  }
  return model
}
