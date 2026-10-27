import type { EditorCommand, EditorEngine, EditorFile, EngineEvent } from '../core/ports'

export interface PostMessageEngineOptions {
  /** The iframe that hosts the editor (same origin as this page). */
  frame: () => HTMLIFrameElement | null
  /** Message type prefix of the editor's `?host=parent` bridge (`<prefix>:ready`, ...). */
  prefix: string
  /** Id of the editor's canvas element, where Ctrl+S is sent. */
  canvasId: string
  /** The origin messages are accepted from and sent to. */
  origin: string
  /** Where messages from the iframe arrive (the page's window). */
  target: Pick<Window, 'addEventListener' | 'removeEventListener'>
}

/** Editor command ids (as sent by the editors' bridges) to the product's commands. */
const COMMANDS: Record<string, EditorCommand | undefined> = {
  'file.new': 'new',
  'file.open': 'open',
  'file.close': 'close',
  'file.saveAs': 'saveAs',
  'file.share': 'share',
}

interface BridgeMessage {
  type?: unknown
  name?: unknown
  bytes?: unknown
  dirty?: unknown
  command?: unknown
}

/** Adapter: talks to an editor library running in an iframe through its `postMessage` bridge. */
export class PostMessageEngine implements EditorEngine {
  constructor(private readonly options: PostMessageEngineOptions) {}

  open(file: EditorFile): void {
    this.options
      .frame()
      ?.contentWindow?.postMessage(
        { type: `${this.options.prefix}:open`, name: file.name, bytes: file.bytes },
        this.options.origin,
        [file.bytes],
      )
  }

  /** Presses Ctrl+S inside the editor (same origin), which makes it send its file. */
  requestSave(): void {
    const canvas = this.options.frame()?.contentDocument?.getElementById(this.options.canvasId)
    if (!canvas) {
      return
    }
    canvas.focus()
    for (const type of ['keydown', 'keyup']) {
      canvas.dispatchEvent(
        new KeyboardEvent(type, { key: 's', code: 'KeyS', ctrlKey: true, bubbles: true }),
      )
    }
  }

  subscribe(listener: (event: EngineEvent) => void): () => void {
    const onMessage = (event: Event): void => {
      if (!(event instanceof MessageEvent)) {
        return
      }
      const frameWindow = this.options.frame()?.contentWindow
      if (event.origin !== this.options.origin || event.source !== frameWindow) {
        return
      }
      const translated = this.translate(event.data)
      if (translated !== null) {
        listener(translated)
      }
    }
    this.options.target.addEventListener('message', onMessage)
    return () => {
      this.options.target.removeEventListener('message', onMessage)
    }
  }

  private translate(data: unknown): EngineEvent | null {
    if (typeof data !== 'object' || data === null || !('type' in data)) {
      return null
    }
    const message = data as BridgeMessage // shape checked field by field below
    const prefix = `${this.options.prefix}:`
    if (typeof message.type !== 'string' || !message.type.startsWith(prefix)) {
      return null
    }
    switch (message.type.slice(prefix.length)) {
      case 'ready':
        return { type: 'ready' }
      case 'save':
        return typeof message.name === 'string' && message.bytes instanceof ArrayBuffer
          ? { type: 'save', file: { name: message.name, bytes: message.bytes } }
          : null
      case 'dirty':
        return { type: 'dirty', dirty: message.dirty === true }
      case 'command': {
        const command = typeof message.command === 'string' ? COMMANDS[message.command] : undefined
        return command === undefined ? null : { type: 'command', command }
      }
      case 'renamed':
        return typeof message.name === 'string' && message.name.trim() !== ''
          ? { type: 'renamed', name: message.name.trim() }
          : null
      case 'failed':
        return { type: 'failed' }
      default:
        return null
    }
  }
}
