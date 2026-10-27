<script setup lang="ts">
import { reactive, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import type { HeaderFooter } from '../../core/page-layout'
import type { PageSetupModel } from '../use-page-setup'
import DialogActions from './DialogActions.vue'
import EditorDialog from './EditorDialog.vue'

const { page } = defineProps<{ page: PageSetupModel }>()
const open = defineModel<boolean>('open', { required: true })

const { t } = useI18n()
const draft = reactive<HeaderFooter>({ ...page.setup.value })
const FIELDS = ['headerLeft', 'headerRight', 'footerLeft', 'footerCenter', 'footerRight'] as const

watch(open, (isOpen) => {
  if (isOpen) {
    Object.assign(draft, page.setup.value)
  }
})

function submit(): void {
  open.value = false
  page.setTexts({ ...draft })
}
</script>

<template>
  <EditorDialog
    v-model:open="open"
    :title="t('ribbon.headerFooterTitle')"
    :description="t('ribbon.headerFooterHint')"
  >
    <form @submit.prevent="submit">
      <div class="grid gap-3">
        <label v-for="field in FIELDS" :key="field" class="text-sm font-medium">
          {{ t(`ribbon.hf.${field}`) }}
          <input
            v-model="draft[field]"
            maxlength="120"
            class="mt-1 block w-full rounded border border-border bg-surface px-3 py-2 font-normal outline-none focus:ring-2 focus:ring-focus"
          />
        </label>
      </div>
      <DialogActions :submit-label="t('ribbon.apply')" />
    </form>
  </EditorDialog>
</template>
