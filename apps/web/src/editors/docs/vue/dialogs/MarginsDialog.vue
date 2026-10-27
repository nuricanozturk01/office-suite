<script setup lang="ts">
import { reactive, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import type { Margins } from '../../core/page-layout'
import type { PageSetupModel } from '../use-page-setup'
import DialogActions from './DialogActions.vue'
import EditorDialog from './EditorDialog.vue'

const { page } = defineProps<{ page: PageSetupModel }>()
const open = defineModel<boolean>('open', { required: true })

const { t } = useI18n()
const SIDES = ['top', 'bottom', 'left', 'right'] as const
const MAX_MM = 100
const draft = reactive<Margins>({ ...page.setup.value.margins })

watch(open, (isOpen) => {
  if (isOpen) {
    Object.assign(draft, page.setup.value.margins)
  }
})

const clamp = (value: number) => Math.min(MAX_MM, Math.max(0, Number.isFinite(value) ? value : 0))

function submit(): void {
  open.value = false
  page.setMargins({
    top: clamp(draft.top),
    right: clamp(draft.right),
    bottom: clamp(draft.bottom),
    left: clamp(draft.left),
  })
}
</script>

<template>
  <EditorDialog v-model:open="open" :title="t('ribbon.customMargins')">
    <form @submit.prevent="submit">
      <div class="grid grid-cols-2 gap-4">
        <label v-for="side in SIDES" :key="side" class="text-sm font-medium">
          {{ t(`ribbon.side.${side}`) }} (mm)
          <input
            v-model.number="draft[side]"
            type="number"
            min="0"
            :max="MAX_MM"
            step="0.1"
            class="mt-1 block w-full rounded border border-border bg-surface px-3 py-2 font-normal outline-none focus:ring-2 focus:ring-focus"
          />
        </label>
      </div>
      <DialogActions :submit-label="t('ribbon.apply')" />
    </form>
  </EditorDialog>
</template>
