<script setup lang="ts">
import { DialogClose } from 'reka-ui'
import { useI18n } from 'vue-i18n'

const { submitLabel } = defineProps<{ submitLabel: string }>()

const { t } = useI18n()
</script>

<template>
  <div class="mt-6 flex justify-end gap-2">
    <DialogClose class="rounded px-4 py-1.5 text-sm text-accent hover:bg-surface-muted">
      {{ t('editor.cancel') }}
    </DialogClose>
    <button
      type="submit"
      class="rounded bg-accent px-4 py-1.5 text-sm text-accent-contrast hover:bg-accent-hover"
    >
      {{ submitLabel }}
    </button>
  </div>
</template>
