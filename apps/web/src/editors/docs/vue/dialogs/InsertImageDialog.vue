<script setup lang="ts">
import type { Editor } from '@tiptap/core'
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { insertImage, normalizeUrl } from '../../core/commands'
import DialogActions from './DialogActions.vue'
import EditorDialog from './EditorDialog.vue'

const { editor } = defineProps<{ editor: Editor }>()

const open = defineModel<boolean>('open', { required: true })

const { t } = useI18n()

const url = ref('')
const invalid = ref(false)

watch(open, () => {
  url.value = ''
  invalid.value = false
})

function submit(): void {
  const src = normalizeUrl(url.value)
  if (src === null) {
    invalid.value = true
    return
  }
  open.value = false
  insertImage(editor, src)
}
</script>

<template>
  <EditorDialog
    v-model:open="open"
    :title="t('editor.image.title')"
    :description="t('editor.image.description')"
  >
    <form @submit.prevent="submit">
      <label class="block text-sm font-medium">
        {{ t('editor.image.label') }}
        <input
          v-model="url"
          type="url"
          class="mt-1 block w-full rounded border border-border bg-surface px-3 py-2 font-normal outline-none focus:ring-2 focus:ring-focus"
          placeholder="https://"
          :aria-invalid="invalid"
          :aria-describedby="invalid ? 'image-error' : undefined"
        />
      </label>
      <p v-if="invalid" id="image-error" class="mt-1 text-xs text-danger">
        {{ t('editor.image.invalid') }}
      </p>
      <DialogActions :submit-label="t('editor.image.insert')" />
    </form>
  </EditorDialog>
</template>
