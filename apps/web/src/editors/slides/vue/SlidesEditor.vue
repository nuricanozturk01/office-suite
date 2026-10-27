<script setup lang="ts">
import { Download, Play } from '@lucide/vue'
import {
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuPortal,
  DropdownMenuRoot,
  DropdownMenuTrigger,
} from 'reka-ui'
import { computed, onBeforeUnmount, onMounted, reactive, ref } from 'vue'
import { useI18n } from 'vue-i18n'

import type { DeckData } from '../core/deck'
import { downloadPptx } from '../core/pptx'
import PresentMode from './PresentMode.vue'
import SlideCanvas from './SlideCanvas.vue'
import SlideRail from './SlideRail.vue'
import SlidesToolbar from './SlidesToolbar.vue'
import { provideDeckModel, useDeckModel } from './use-deck-model'

const { presentationId, initial, version, title } = defineProps<{
  presentationId: string
  initial: DeckData
  version: number
  title: string
}>()

const emit = defineEmits<{ reload: []; newPresentation: [] }>()

defineSlots<{
  leading?: () => unknown
  title?: () => unknown
  actions?: () => unknown
}>()

const { t } = useI18n()
const model = useDeckModel(presentationId, initial, version)
provideDeckModel(model)
const ui = reactive(model)
const presenting = ref(false)
const exportError = ref(false)

const saveLabel = computed(() => {
  switch (ui.status) {
    case 'saved':
      return t('slides.saved')
    case 'saving':
      return t('slides.saving')
    case 'error':
      return t('slides.saveError')
    case 'conflict':
      return t('slides.conflict')
  }
})

async function download(): Promise<void> {
  exportError.value = false
  try {
    await downloadPptx(presentationId, title.trim() || t('slides.untitled'))
  } catch {
    exportError.value = true
  }
}

function onShortcut(event: KeyboardEvent): void {
  const target = event.target as HTMLElement // keydown target is always an element
  if (
    target.tagName === 'INPUT' ||
    target.tagName === 'TEXTAREA' ||
    !(event.ctrlKey || event.metaKey)
  ) {
    return
  }
  const key = event.key.toLowerCase()
  if (key === 'z') {
    if (event.shiftKey) {
      model.redo()
    } else {
      model.undo()
    }
  } else if (key === 'y') {
    model.redo()
  } else if (key === 'd') {
    model.duplicateSelected()
  } else {
    return
  }
  event.preventDefault()
}

onMounted(() => {
  window.addEventListener('keydown', onShortcut)
})
onBeforeUnmount(() => {
  window.removeEventListener('keydown', onShortcut)
})

const itemClass =
  'flex cursor-default items-center gap-3 px-4 py-1.5 outline-none data-highlighted:bg-surface-muted'
</script>

<template>
  <div class="flex h-dvh flex-col bg-surface-muted text-text">
    <header class="flex items-center gap-3 bg-surface px-3 pt-2 pb-1">
      <slot name="leading" />
      <div class="flex min-w-0 flex-1 items-center gap-3">
        <slot name="title" />
        <p class="truncate text-xs text-text-muted">{{ saveLabel }}</p>
        <button
          v-if="ui.status === 'error'"
          type="button"
          class="text-xs text-accent"
          @click="ui.retrySave()"
        >
          {{ t('slides.retrySave') }}
        </button>
      </div>
      <button
        type="button"
        class="inline-flex items-center gap-2 rounded bg-accent px-3 py-1 text-sm text-accent-contrast"
        @click="presenting = true"
      >
        <Play class="size-4" aria-hidden="true" />
        {{ t('slides.present') }}
      </button>
      <DropdownMenuRoot>
        <DropdownMenuTrigger
          class="rounded px-2 py-1 text-sm hover:bg-surface-muted"
          :aria-label="t('slides.fileMenu')"
        >
          {{ t('editor.menu.file') }}
        </DropdownMenuTrigger>
        <DropdownMenuPortal>
          <DropdownMenuContent
            class="z-50 min-w-48 rounded-md border border-border bg-surface-raised py-1.5 text-sm shadow-lg"
            align="end"
          >
            <DropdownMenuItem :class="itemClass" @select="emit('newPresentation')">
              {{ t('slides.new') }}
            </DropdownMenuItem>
            <DropdownMenuItem :class="itemClass" @select="download">
              <Download class="size-4" aria-hidden="true" />
              {{ t('slides.downloadPptx') }}
            </DropdownMenuItem>
            <DropdownMenuItem :class="itemClass" @select="model.duplicateSlide()">
              {{ t('slides.duplicateSlide') }}
            </DropdownMenuItem>
            <DropdownMenuItem :class="itemClass" @select="model.moveSlide(-1)">
              {{ t('slides.moveUp') }}
            </DropdownMenuItem>
            <DropdownMenuItem :class="itemClass" @select="model.moveSlide(1)">
              {{ t('slides.moveDown') }}
            </DropdownMenuItem>
            <DropdownMenuItem :class="itemClass" @select="model.deleteSlide()">
              {{ t('slides.deleteSlide') }}
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenuPortal>
      </DropdownMenuRoot>
      <slot name="actions" />
    </header>
    <p v-if="exportError" class="bg-danger-surface px-4 py-2 text-sm text-danger" role="alert">
      {{ t('slides.exportError') }}
    </p>
    <div
      v-if="ui.status === 'conflict'"
      class="flex items-center gap-3 bg-danger-surface px-4 py-2 text-sm text-danger"
      role="alert"
    >
      <p class="flex-1">{{ t('slides.conflict') }}</p>
      <button type="button" class="font-medium underline" @click="emit('reload')">
        {{ t('slides.reload') }}
      </button>
    </div>
    <div class="bg-surface px-2 pb-1">
      <SlidesToolbar />
    </div>
    <div class="flex min-h-0 flex-1">
      <SlideRail />
      <main
        id="main"
        class="flex min-w-0 flex-1 flex-col items-center gap-3 overflow-hidden p-4"
        tabindex="-1"
      >
        <div class="flex min-h-0 w-full flex-1 items-center justify-center">
          <SlideCanvas />
        </div>
        <label class="w-full max-w-5xl text-sm">
          <span class="sr-only">{{ t('slides.notes') }}</span>
          <textarea
            class="h-20 w-full resize-none rounded border border-border bg-surface p-2"
            :placeholder="t('slides.notes')"
            :value="ui.slide.notes ?? ''"
            @input="model.setNotes(($event.target as HTMLTextAreaElement).value)"
          />
        </label>
      </main>
    </div>
    <PresentMode
      v-if="presenting"
      :deck="ui.deck"
      :start="ui.activeIndex"
      @close="presenting = false"
    />
  </div>
</template>
