import { nextTick, ref, watch, type Ref } from 'vue'

import type { SheetModel } from './use-sheet-model'

/** Selection drag, fill handle, column resize, clipboard and the in-cell editor. */
export function useGridPointer(
  model: SheetModel,
  scroller: Ref<HTMLElement | null>,
  editor: Ref<HTMLInputElement | null>,
  cellFrom: (event: MouseEvent) => { row: number; col: number } | null,
) {
  const selecting = ref(false)

  function onPointerDown(event: PointerEvent): void {
    if (event.button !== 0) {
      return
    }
    if (event.target instanceof HTMLInputElement || event.target instanceof HTMLButtonElement) {
      return
    }
    const pos = cellFrom(event)
    if (pos === null) {
      return
    }
    model.select(pos, event.shiftKey)
    if (event.shiftKey) {
      return
    }
    selecting.value = true
    scroller.value?.setPointerCapture(event.pointerId)
  }

  function onPointerMove(event: PointerEvent): void {
    if (!selecting.value) {
      return
    }
    const pos = cellFrom(event)
    if (pos !== null) {
      model.select(pos, true)
    }
  }

  function onPointerUp(): void {
    selecting.value = false
  }

  function onDoubleClick(event: MouseEvent): void {
    const pos = cellFrom(event)
    if (pos !== null) {
      model.select(pos, false)
      model.startEdit('cell')
    }
  }

  function startFill(event: PointerEvent): void {
    const handle = event.currentTarget
    if (!(handle instanceof HTMLElement)) {
      return
    }
    let target: { row: number; col: number } | null = null
    handle.setPointerCapture(event.pointerId)
    const move = (pointer: PointerEvent) => {
      target = cellFrom(pointer)
    }
    const end = () => {
      handle.removeEventListener('pointermove', move)
      handle.removeEventListener('pointerup', end)
      if (target !== null) {
        model.fillTo(target)
      }
    }
    handle.addEventListener('pointermove', move)
    handle.addEventListener('pointerup', end)
  }

  function startResize(event: PointerEvent, col: number): void {
    const handle = event.currentTarget
    if (!(handle instanceof HTMLElement)) {
      return
    }
    const startX = event.clientX
    const startWidth = model.colWidth(col)
    model.armColumnResize()
    handle.setPointerCapture(event.pointerId)
    const move = (pointer: PointerEvent) => {
      model.setColWidth(col, startWidth + pointer.clientX - startX)
    }
    const end = () => {
      handle.removeEventListener('pointermove', move)
      handle.removeEventListener('pointerup', end)
    }
    handle.addEventListener('pointermove', move)
    handle.addEventListener('pointerup', end)
  }

  function onCopy(event: ClipboardEvent): void {
    if (model.editing.value !== null) {
      return
    }
    event.preventDefault()
    event.clipboardData?.setData('text/plain', model.copyText())
  }

  function onPaste(event: ClipboardEvent): void {
    if (model.editing.value !== null) {
      return
    }
    const text = event.clipboardData?.getData('text/plain')
    if (text === undefined || text === '') {
      return
    }
    event.preventDefault()
    model.paste(text)
  }

  function onCut(event: ClipboardEvent): void {
    onCopy(event)
    if (model.editing.value === null) {
      model.clearSelection()
    }
  }

  function onDraft(event: Event): void {
    if (event.target instanceof HTMLInputElement) {
      model.setDraft(event.target.value)
    }
  }

  function commitMove(dRow: number, dCol: number): void {
    model.commit({ row: model.focus.value.row + dRow, col: model.focus.value.col + dCol })
  }

  function onEditorKeydown(event: KeyboardEvent): void {
    if (event.key === 'Enter') {
      commitMove(event.shiftKey ? -1 : 1, 0)
    } else if (event.key === 'Tab') {
      commitMove(0, event.shiftKey ? -1 : 1)
    } else if (event.key === 'Escape') {
      model.cancelEdit()
    } else {
      return
    }
    event.preventDefault()
    event.stopPropagation()
  }

  watch(
    () => model.editing.value,
    (current, previous) => {
      if (current?.source === 'cell') {
        void nextTick(() => {
          editor.value?.focus()
        })
      } else if (previous?.source === 'cell') {
        scroller.value?.focus()
      }
    },
  )

  return {
    onPointerDown,
    onPointerMove,
    onPointerUp,
    onDoubleClick,
    startFill,
    startResize,
    onCopy,
    onPaste,
    onCut,
    onDraft,
    onEditorKeydown,
  }
}
