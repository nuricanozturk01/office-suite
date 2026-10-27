<script setup lang="ts">
import type { Editor } from '@tiptap/core'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import EditorDialog from './EditorDialog.vue'

const { editor } = defineProps<{ editor: Editor }>()

const open = defineModel<boolean>('open', { required: true })

const { t, locale } = useI18n()

const stats = computed(() => {
  const format = new Intl.NumberFormat(locale.value)
  const storage = editor.storage.characterCount
  return [
    { label: t('editor.wordCount.words'), value: format.format(storage.words()) },
    { label: t('editor.wordCount.characters'), value: format.format(storage.characters()) },
  ]
})
</script>

<template>
  <EditorDialog v-model:open="open" :title="t('editor.wordCount.title')">
    <dl class="divide-y divide-border text-sm">
      <div v-for="stat in stats" :key="stat.label" class="flex justify-between py-2">
        <dt>{{ stat.label }}</dt>
        <dd class="font-medium tabular-nums">{{ stat.value }}</dd>
      </div>
    </dl>
  </EditorDialog>
</template>
