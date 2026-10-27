<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { errorMessageKey } from '@/shared/api'
import { AppButton, AppDialog, InlineAlert, TextField } from '@/shared/ui'

import type { FileKind } from '../model/file-kind'
import { useRenameFile } from '../model/use-rename-file'

const { kind, id, title } = defineProps<{ kind: FileKind; id: string; title: string }>()

const open = defineModel<boolean>('open', { required: true })

const { t } = useI18n()
const rename = useRenameFile()
const draft = ref(title)
const error = ref<string>()

watch(open, (isOpen) => {
  if (isOpen) {
    draft.value = title
    error.value = undefined
    rename.reset()
  }
})

function submit(): void {
  const next = draft.value.trim()
  if (next === '') {
    error.value = t('validation.required')
    return
  }
  if (next === title.trim()) {
    open.value = false
    return
  }
  error.value = undefined
  rename.mutate(
    { kind, id, title: next },
    {
      onSuccess: () => {
        open.value = false
      },
    },
  )
}
</script>

<template>
  <AppDialog
    v-model:open="open"
    :title="t('fileActions.renameTitle')"
    :close-label="t('share.close')"
  >
    <form class="flex flex-col gap-4" novalidate @submit.prevent="submit">
      <TextField v-model="draft" :label="t('fileActions.name')" :error="error" />
      <InlineAlert v-if="rename.isError.value">
        {{ t(errorMessageKey(rename.error.value)) }}
      </InlineAlert>
      <div class="flex justify-end gap-2">
        <AppButton variant="ghost" @click="open = false">{{ t('fileActions.cancel') }}</AppButton>
        <AppButton type="submit" :loading="rename.isPending.value">{{
          t('fileActions.save')
        }}</AppButton>
      </div>
    </form>
  </AppDialog>
</template>
