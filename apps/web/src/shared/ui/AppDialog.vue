<script setup lang="ts">
import { X } from '@lucide/vue'
import {
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogOverlay,
  DialogPortal,
  DialogRoot,
  DialogTitle,
} from 'reka-ui'

const { title, description, closeLabel } = defineProps<{
  title: string
  description?: string
  closeLabel: string
}>()

const open = defineModel<boolean>('open', { required: true })
</script>

<template>
  <DialogRoot v-model:open="open">
    <DialogPortal>
      <DialogOverlay class="fixed inset-0 z-50 bg-black/40" />
      <DialogContent
        class="fixed top-1/2 left-1/2 z-50 max-h-[90dvh] w-[min(32rem,calc(100vw-2rem))] -translate-x-1/2 -translate-y-1/2 overflow-y-auto rounded-lg bg-surface-raised p-6 text-text shadow-xl"
        v-bind="description ? {} : { 'aria-describedby': undefined }"
      >
        <DialogTitle class="text-lg font-medium">{{ title }}</DialogTitle>
        <DialogDescription v-if="description" class="mt-1 text-sm text-text-muted">
          {{ description }}
        </DialogDescription>
        <div class="mt-4"><slot /></div>
        <DialogClose
          class="absolute end-3 top-3 rounded-full p-1.5 text-text-muted hover:bg-surface-muted"
          :aria-label="closeLabel"
        >
          <X class="size-4" aria-hidden="true" />
        </DialogClose>
      </DialogContent>
    </DialogPortal>
  </DialogRoot>
</template>
