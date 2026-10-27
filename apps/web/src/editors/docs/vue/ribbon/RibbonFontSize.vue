<script setup lang="ts">
import { Minus, Plus } from '@lucide/vue'
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { FONT_SIZES, MAX_FONT_SIZE, MIN_FONT_SIZE } from '../../core/formatting'

const { size } = defineProps<{ size: number }>()
const emit = defineEmits<{ change: [points: number] }>()

const { t } = useI18n()
const draft = ref(String(size))
watch(
  () => size,
  (next) => {
    draft.value = String(next)
  },
)

function step(direction: 1 | -1): void {
  const next =
    direction === 1
      ? FONT_SIZES.find((candidate) => candidate > size)
      : [...FONT_SIZES].reverse().find((candidate) => candidate < size)
  emit('change', next ?? size + direction)
}

function commit(): void {
  const parsed = Number.parseFloat(draft.value)
  if (Number.isFinite(parsed) && parsed !== size) {
    emit('change', parsed)
  } else {
    draft.value = String(size)
  }
}

const button =
  'inline-flex size-7 items-center justify-center rounded outline-none hover:bg-surface-muted focus-visible:ring-2 focus-visible:ring-focus disabled:opacity-40'
</script>

<template>
  <div class="flex items-center">
    <button
      type="button"
      :class="button"
      :aria-label="t('editor.actions.decreaseFontSize')"
      :disabled="size <= MIN_FONT_SIZE"
      @mousedown.prevent
      @click="step(-1)"
    >
      <Minus class="size-4" aria-hidden="true" />
    </button>
    <input
      v-model="draft"
      class="h-7 w-11 rounded border border-border bg-surface text-center text-sm outline-none focus:ring-2 focus:ring-focus"
      inputmode="decimal"
      :aria-label="t('editor.actions.fontSize')"
      :title="t('editor.actions.fontSize')"
      @keydown.enter.prevent="commit"
      @blur="commit"
    />
    <button
      type="button"
      :class="button"
      :aria-label="t('editor.actions.increaseFontSize')"
      :disabled="size >= MAX_FONT_SIZE"
      @mousedown.prevent
      @click="step(1)"
    >
      <Plus class="size-4" aria-hidden="true" />
    </button>
  </div>
</template>
