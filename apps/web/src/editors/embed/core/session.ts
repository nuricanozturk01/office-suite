import {
  type Downloader,
  type EditorCommand,
  type EditorEngine,
  type EditorFile,
  type EngineEvent,
  type FileStore,
  type SaveStatus,
  VersionConflictError,
} from './ports'

export interface SessionOptions {
  engine: EditorEngine
  store: FileStore
  downloader: Downloader
  /** Whether a file the editor sends is the document itself (true) or an export copy (false). */
  isDocumentFormat: (name: string) => boolean
  /** Called when the status changes. */
  onStatus: (status: SaveStatus) => void
  /** Called for commands the product handles (`new`, `close`). */
  onCommand: (command: Exclude<EditorCommand, 'saveAs'>) => void
  /** Milliseconds after the last change before the document is saved. `0` turns autosave off. */
  autosaveMs: number
  /** Called after a save stored `version`. */
  onSaved?: (version: number) => void
  /** Called after a save that also merged in changes saved by others (the editor reloads). */
  onMerged?: () => void
  /** Called when the store refused a save because the file changed meanwhile. */
  onSaveConflict?: () => void
  /** Called when an older version is shown (its number) or the preview ends (`null`). */
  onPreview?: (version: number | null) => void
  /** Called when the user renamed the document inside the editor. */
  onRenamed?: (name: string) => void
}

/**
 * One editing session: loads the document into the editor, keeps it in the store, and routes
 * the editor's file-level commands. Pure application logic over the ports; no DOM, no HTTP.
 */
export class EditorSession {
  private storedVersion: number | null = null
  private dirty = false
  private saving = false
  private previewing: number | null = null
  private copyRequested = false
  private timer: ReturnType<typeof setTimeout> | undefined
  private readonly unsubscribe: () => void

  constructor(private readonly options: SessionOptions) {
    this.options.onStatus('loading')
    this.unsubscribe = options.engine.subscribe((event) => {
      void this.handle(event)
    })
  }

  /** Stores the document in the cloud now. */
  saveToCloud(): void {
    this.options.engine.requestSave()
  }

  /** Saves, then also gives the same file to the user's computer. */
  downloadCopy(): void {
    this.copyRequested = true
    this.options.engine.requestSave()
  }

  dispose(): void {
    clearTimeout(this.timer)
    this.unsubscribe()
  }

  /** Whether leaving the page would lose changes. */
  get hasUnsavedChanges(): boolean {
    return this.dirty
  }

  /** Version of the stored file this session is based on; `null` before the first save. */
  get version(): number | null {
    return this.storedVersion
  }

  /** Number of the older version on screen, or `null` when the current file is shown. */
  get previewVersion(): number | null {
    return this.previewing
  }

  /**
   * Shows an older version. Nothing typed while it is on screen is saved; [`reload`] returns to
   * the current file.
   */
  showVersion(file: EditorFile, version: number): void {
    clearTimeout(this.timer)
    this.dirty = false
    this.setPreview(version)
    this.options.engine.open(file)
    this.options.onStatus('saved')
  }

  get isSaving(): boolean {
    return this.saving
  }

  /** Replaces what the editor shows with the latest stored file (unsaved changes are dropped). */
  async reload(): Promise<void> {
    const stored = await this.options.store.load()
    if (stored === null) {
      return
    }
    clearTimeout(this.timer)
    this.dirty = false
    this.setPreview(null)
    this.storedVersion = stored.version
    this.options.engine.open(stored.file)
    this.options.onStatus('saved')
  }

  /** Keeps what the editor shows: the next save replaces the newer stored version. */
  async keepMine(): Promise<void> {
    const stored = await this.options.store.load()
    if (stored !== null) {
      this.storedVersion = stored.version
    }
    this.options.engine.requestSave()
  }

  async latestStoredVersion(): Promise<number | null> {
    return (await this.options.store.load())?.version ?? null
  }

  private async handle(event: EngineEvent): Promise<void> {
    switch (event.type) {
      case 'ready':
        await this.open()
        break
      case 'save':
        await this.save(event.file)
        break
      case 'dirty':
        this.markDirty(event.dirty)
        break
      case 'command':
        this.command(event.command)
        break
      case 'renamed':
        this.options.onRenamed?.(event.name)
        break
      case 'failed':
        this.options.onStatus('failed')
        break
    }
  }

  private async open(): Promise<void> {
    try {
      const stored = await this.options.store.load()
      const file = stored?.file ?? (await this.options.store.blank())
      this.storedVersion = stored?.version ?? null
      if (file !== null) {
        this.options.engine.open(file)
      }
      this.options.onStatus('saved')
    } catch {
      this.options.onStatus('error')
    }
  }

  private setPreview(version: number | null): void {
    if (this.previewing !== version) {
      this.previewing = version
      this.options.onPreview?.(version)
    }
  }

  private async save(file: EditorFile): Promise<void> {
    if (this.previewing !== null && this.options.isDocumentFormat(file.name)) {
      return // an older version is on screen: it never replaces the current file
    }
    if (!this.options.isDocumentFormat(file.name)) {
      this.options.downloader.download(file)
      return
    }
    this.options.onStatus('saving')
    if (this.copyRequested) {
      this.copyRequested = false
      this.options.downloader.download(file)
    }
    this.saving = true
    try {
      const saved = await this.options.store.save(file, this.storedVersion)
      this.storedVersion = saved.version
      this.saving = false
      this.options.onStatus(this.dirty ? 'dirty' : 'saved')
      this.options.onSaved?.(saved.version)
      if (saved.merged) {
        // The stored file now holds both sets of changes; show it (the view stays put).
        await this.reload()
        this.options.onMerged?.()
      }
    } catch (error) {
      this.saving = false
      const conflict = error instanceof VersionConflictError
      this.options.onStatus(conflict ? 'conflict' : 'error')
      if (conflict) {
        this.options.onSaveConflict?.()
      }
    }
  }

  private markDirty(dirty: boolean): void {
    if (this.previewing !== null) {
      return
    }
    this.dirty = dirty
    clearTimeout(this.timer)
    this.options.onStatus(dirty ? 'dirty' : 'saved')
    if (dirty && this.options.autosaveMs > 0) {
      this.timer = setTimeout(() => {
        this.options.engine.requestSave()
      }, this.options.autosaveMs)
    }
  }

  private command(command: EditorCommand): void {
    if (command === 'saveAs') {
      this.downloadCopy()
    } else {
      this.options.onCommand(command)
    }
  }
}
