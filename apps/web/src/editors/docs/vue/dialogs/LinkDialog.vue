<script setup lang="ts">
import type { Editor } from '@tiptap/core'
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { currentLink, normalizeUrl, setLink } from '../../core/commands'
import DialogActions from './DialogActions.vue'
import EditorDialog from './EditorDialog.vue'

const { editor } = defineProps<{ editor: Editor }>()
const open = defineModel<boolean>('open', { required: true })

const { t } = useI18n()
const href = ref('')
const invalid = ref(false)

watch(open, (isOpen) => {
  if (isOpen) {
    href.value = currentLink(editor) ?? ''
    invalid.value = false
  }
})

function submit(): void {
  const url = normalizeUrl(href.value)
  if (url === null) {
    invalid.value = true
    return
  }
  setLink(editor, url)
  open.value = false
}

function remove(): void {
  setLink(editor, null)
  open.value = false
}
</script>

<template>
  <EditorDialog v-model:open="open" :title="t('editor.actions.link')">
    <form @submit.prevent="submit">
      <label class="text-sm font-medium">
        {{ t('editor.link.label') }}
        <input
          v-model="href"
          type="url"
          :aria-invalid="invalid"
          class="mt-1 block w-full rounded border border-border bg-surface px-3 py-2 font-normal outline-none focus:ring-2 focus:ring-focus"
          placeholder="https://"
        />
      </label>
      <p v-if="invalid" class="mt-2 text-sm text-danger" role="alert">
        {{ t('editor.link.invalid') }}
      </p>
      <DialogActions :submit-label="t('editor.link.apply')" />
      <button
        v-if="editor.isActive('link')"
        type="button"
        class="mt-2 text-sm text-danger hover:underline"
        @click="remove"
      >
        {{ t('editor.link.remove') }}
      </button>
    </form>
  </EditorDialog>
</template>
