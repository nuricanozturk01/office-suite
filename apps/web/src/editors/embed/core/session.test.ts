import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import {
  type Downloader,
  type EditorEngine,
  type EditorFile,
  type EngineEvent,
  type FileStore,
  type SaveStatus,
  VersionConflictError,
} from './ports'
import { EditorSession } from './session'

const file = (name: string): EditorFile => ({ name, bytes: new ArrayBuffer(1) })

class FakeEngine implements EditorEngine {
  opened: EditorFile[] = []
  saveRequests = 0
  private listener: ((event: EngineEvent) => void) | undefined

  open(f: EditorFile): void {
    this.opened.push(f)
  }

  requestSave(): void {
    this.saveRequests += 1
  }

  subscribe(listener: (event: EngineEvent) => void): () => void {
    this.listener = listener
    return () => {
      this.listener = undefined
    }
  }

  emit(event: EngineEvent): void {
    this.listener?.(event)
  }
}

function setup(store: Partial<FileStore> = {}, autosaveMs = 1000) {
  const savedVersions: number[] = []
  const conflicts: string[] = []
  const merges: string[] = []
  const engine = new FakeEngine()
  const statuses: SaveStatus[] = []
  const commands: string[] = []
  const downloads: string[] = []
  const saved: { name: string; base: number | null }[] = []
  const downloader: Downloader = {
    download: (f) => {
      downloads.push(f.name)
    },
  }
  const fullStore: FileStore = {
    load: () => Promise.resolve(null),
    blank: () => Promise.resolve(file('Blank.docx')),
    save: (f, base) => {
      saved.push({ name: f.name, base })
      return Promise.resolve({ version: (base ?? 0) + 1, merged: false })
    },
    ...store,
  }
  const session = new EditorSession({
    engine,
    store: fullStore,
    downloader,
    isDocumentFormat: (name) => name.endsWith('.docx'),
    onStatus: (s) => statuses.push(s),
    onCommand: (c) => commands.push(c),
    autosaveMs,
    onSaved: (version) => savedVersions.push(version),
    onSaveConflict: () => conflicts.push('conflict'),
    onMerged: () => merges.push('merged'),
  })
  return { engine, session, statuses, commands, downloads, saved, savedVersions, conflicts, merges }
}

const flush = () => vi.advanceTimersByTimeAsync(0)

describe('EditorSession', () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })
  afterEach(() => {
    vi.useRealTimers()
  })

  it('opens the stored file when the editor is ready', async () => {
    const { engine, statuses } = setup({
      load: () => Promise.resolve({ file: file('Stored.docx'), version: 4 }),
    })
    engine.emit({ type: 'ready' })
    await flush()
    expect(engine.opened.map((f) => f.name)).toEqual(['Stored.docx'])
    expect(statuses.at(-1)).toBe('saved')
  })

  it('starts from the blank file when nothing is stored', async () => {
    const { engine } = setup()
    engine.emit({ type: 'ready' })
    await flush()
    expect(engine.opened.map((f) => f.name)).toEqual(['Blank.docx'])
  })

  it('reports an error when loading fails', async () => {
    const { engine, statuses } = setup({ load: () => Promise.reject(new Error('down')) })
    engine.emit({ type: 'ready' })
    await flush()
    expect(statuses.at(-1)).toBe('error')
  })

  it('stores documents with the version they were opened from', async () => {
    const { engine, saved, statuses } = setup({
      load: () => Promise.resolve({ file: file('A.docx'), version: 4 }),
    })
    engine.emit({ type: 'ready' })
    await flush()
    engine.emit({ type: 'save', file: file('A.docx') })
    await flush()
    engine.emit({ type: 'save', file: file('A.docx') })
    await flush()
    expect(saved.map((s) => s.base)).toEqual([4, 5])
    expect(statuses.at(-1)).toBe('saved')
  })

  it('downloads exports instead of storing them', async () => {
    const { engine, downloads, saved } = setup()
    engine.emit({ type: 'save', file: file('A.pdf') })
    await flush()
    expect(downloads).toEqual(['A.pdf'])
    expect(saved).toEqual([])
  })

  it('stores and downloads a copy on request', async () => {
    const { engine, session, downloads, saved } = setup()
    session.downloadCopy()
    expect(engine.saveRequests).toBe(1)
    engine.emit({ type: 'save', file: file('A.docx') })
    await flush()
    expect(downloads).toEqual(['A.docx'])
    expect(saved).toHaveLength(1)
  })

  it('saves automatically after the last change', async () => {
    const { engine, statuses } = setup()
    engine.emit({ type: 'dirty', dirty: true })
    await vi.advanceTimersByTimeAsync(600)
    engine.emit({ type: 'dirty', dirty: true })
    await vi.advanceTimersByTimeAsync(600)
    expect(engine.saveRequests).toBe(0)
    await vi.advanceTimersByTimeAsync(600)
    expect(engine.saveRequests).toBe(1)
    expect(statuses).toContain('dirty')
  })

  it('does not autosave when it is off', async () => {
    const { engine } = setup({}, 0)
    engine.emit({ type: 'dirty', dirty: true })
    await vi.advanceTimersByTimeAsync(60_000)
    expect(engine.saveRequests).toBe(0)
  })

  it('reports a conflict and leaves other failures as errors', async () => {
    const conflict = setup({ save: () => Promise.reject(new VersionConflictError()) })
    conflict.engine.emit({ type: 'save', file: file('A.docx') })
    await flush()
    expect(conflict.statuses.at(-1)).toBe('conflict')
    const broken = setup({ save: () => Promise.reject(new Error('down')) })
    broken.engine.emit({ type: 'save', file: file('A.docx') })
    await flush()
    expect(broken.statuses.at(-1)).toBe('error')
  })

  it('hands file commands to the product', async () => {
    const { engine, commands, downloads } = setup()
    engine.emit({ type: 'command', command: 'new' })
    engine.emit({ type: 'command', command: 'close' })
    expect(commands).toEqual(['new', 'close'])
    engine.emit({ type: 'command', command: 'saveAs' })
    await flush()
    expect(downloads).toEqual([])
  })

  it('stops listening after dispose', () => {
    const { engine, session, commands } = setup()
    session.dispose()
    engine.emit({ type: 'command', command: 'new' })
    expect(commands).toEqual([])
  })

  it('reports each stored version and exposes it', async () => {
    const { engine, session, savedVersions } = setup({
      load: () => Promise.resolve({ file: file('A.docx'), version: 4 }),
    })
    engine.emit({ type: 'ready' })
    await flush()
    expect(session.version).toBe(4)
    engine.emit({ type: 'save', file: file('A.docx') })
    await flush()
    expect(savedVersions).toEqual([5])
    expect(session.version).toBe(5)
    expect(session.isSaving).toBe(false)
  })

  it('reports a refused save as a conflict', async () => {
    const { engine, statuses, conflicts } = setup({
      load: () => Promise.resolve({ file: file('A.docx'), version: 4 }),
      save: () => Promise.reject(new VersionConflictError()),
    })
    engine.emit({ type: 'ready' })
    await flush()
    engine.emit({ type: 'save', file: file('A.docx') })
    await flush()
    expect(statuses.at(-1)).toBe('conflict')
    expect(conflicts).toEqual(['conflict'])
  })

  it("shows the merged file after a save that combined others' changes", async () => {
    let stored = { file: file('A.docx'), version: 4 }
    const { engine, session, merges } = setup({
      load: () => Promise.resolve(stored),
      save: () => {
        stored = { file: file('Merged.docx'), version: 6 }
        return Promise.resolve({ version: 6, merged: true })
      },
    })
    engine.emit({ type: 'ready' })
    await flush()
    engine.emit({ type: 'save', file: file('A.docx') })
    await flush()
    expect(engine.opened.at(-1)?.name).toBe('Merged.docx')
    expect(session.version).toBe(6)
    expect(merges).toEqual(['merged'])
  })

  it('shows an older version without ever saving it, until reload', async () => {
    const { engine, session, saved } = setup({
      load: () => Promise.resolve({ file: file('Current.docx'), version: 5 }),
    })
    engine.emit({ type: 'ready' })
    await flush()

    session.showVersion(file('Old.docx'), 2)
    expect(engine.opened.at(-1)?.name).toBe('Old.docx')
    expect(session.previewVersion).toBe(2)

    engine.emit({ type: 'dirty', dirty: true })
    await vi.advanceTimersByTimeAsync(5000)
    engine.emit({ type: 'save', file: file('Old.docx') })
    await flush()
    expect(engine.saveRequests).toBe(0)
    expect(saved).toEqual([])

    await session.reload()
    expect(session.previewVersion).toBeNull()
    expect(engine.opened.at(-1)?.name).toBe('Current.docx')
  })

  it('reload shows the stored file again and drops unsaved changes', async () => {
    let stored = { file: file('A.docx'), version: 4 }
    const { engine, session, statuses } = setup({ load: () => Promise.resolve(stored) })
    engine.emit({ type: 'ready' })
    await flush()
    engine.emit({ type: 'dirty', dirty: true })
    expect(session.hasUnsavedChanges).toBe(true)

    stored = { file: file('B.docx'), version: 9 }
    await session.reload()
    expect(engine.opened.at(-1)?.name).toBe('B.docx')
    expect(session.version).toBe(9)
    expect(session.hasUnsavedChanges).toBe(false)
    expect(statuses.at(-1)).toBe('saved')
  })

  it('keepMine adopts the newer version and asks the editor to save', async () => {
    let stored = { file: file('A.docx'), version: 4 }
    const { engine, session, saved } = setup({
      load: () => Promise.resolve(stored),
      save: (f, base) => {
        saved.push({ name: f.name, base })
        return Promise.resolve({ version: (base ?? 0) + 1, merged: false })
      },
    })
    engine.emit({ type: 'ready' })
    await flush()
    stored = { file: file('A.docx'), version: 9 }
    await session.keepMine()
    expect(engine.saveRequests).toBe(1)
    engine.emit({ type: 'save', file: file('A.docx') })
    await flush()
    expect(saved.at(-1)?.base).toBe(9)
  })
})
