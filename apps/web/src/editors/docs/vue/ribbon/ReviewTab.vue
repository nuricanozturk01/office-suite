<script setup lang="ts">
import { Eye, Hash, Pencil, SpellCheck } from '@lucide/vue'
import type { Editor } from '@tiptap/core'
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'

import { RibbonButton, RibbonGroup } from '@/shared/ui'

import { useDocsView } from '../docs-view'

const { editor, readOnly } = defineProps<{ editor: Editor; readOnly: boolean }>()

const { t } = useI18n()
const { dialog } = useDocsView()
const spelling = ref(true)
const viewing = ref(false)

function toggleSpelling(): void {
  spelling.value = !spelling.value
  editor.view.dom.setAttribute('spellcheck', String(spelling.value))
}

function toggleViewing(): void {
  viewing.value = !viewing.value
  editor.setEditable(!viewing.value)
}
</script>

<template>
  <RibbonGroup :label="t('ribbon.group.proofing')">
    <RibbonButton
      :icon="SpellCheck"
      :label="t('ribbon.spelling')"
      size="large"
      :active="spelling"
      @click="toggleSpelling"
    />
    <RibbonButton
      :icon="Hash"
      :label="t('ribbon.wordCount')"
      size="large"
      @click="dialog = 'wordCount'"
    />
  </RibbonGroup>
  <RibbonGroup :label="t('ribbon.group.mode')">
    <RibbonButton
      :icon="Pencil"
      :label="t('ribbon.editing')"
      size="large"
      :active="!viewing"
      :disabled="readOnly"
      @click="viewing && toggleViewing()"
    />
    <RibbonButton
      :icon="Eye"
      :label="t('ribbon.viewing')"
      size="large"
      :active="viewing"
      @click="!viewing && toggleViewing()"
    />
  </RibbonGroup>
</template>
