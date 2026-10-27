<script setup lang="ts">
import { EllipsisVertical, FileText, Trash2 } from '@lucide/vue'
import {
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuPortal,
  DropdownMenuRoot,
  DropdownMenuTrigger,
} from 'reka-ui'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { RouterLink } from 'vue-router'

import type { DocumentSummary } from '@/entities/document'
import { RouteName } from '@/shared/config/routes'

const { document, editedAt } = defineProps<{ document: DocumentSummary; editedAt: string }>()

const emit = defineEmits<{ delete: [] }>()

const { t } = useI18n()
const title = computed(() => document.title || t('documents.untitled'))
</script>

<template>
  <li
    class="group relative flex flex-col overflow-hidden rounded-md border border-border bg-surface-raised transition hover:border-accent"
  >
    <div
      aria-hidden="true"
      class="flex h-40 flex-col gap-2 border-b border-border bg-surface px-6 pt-5"
    >
      <span class="h-2 w-2/3 rounded bg-surface-muted" />
      <span v-for="line in 6" :key="line" class="h-1.5 rounded bg-surface-muted" />
    </div>
    <div class="flex items-start gap-2 px-3 py-3">
      <div class="min-w-0 flex-1">
        <RouterLink
          :to="{ name: RouteName.Document, params: { id: document.id } }"
          class="block truncate text-sm font-medium after:absolute after:inset-0"
        >
          {{ title }}
        </RouterLink>
        <p class="mt-1 flex items-center gap-1.5 text-xs text-text-muted">
          <FileText class="size-4 shrink-0 text-accent" aria-hidden="true" />
          <span class="truncate">{{ t('documents.opened', { date: editedAt }) }}</span>
        </p>
      </div>
      <DropdownMenuRoot>
        <DropdownMenuTrigger
          class="relative z-10 -me-1 rounded-full p-1.5 text-text-muted hover:bg-surface-muted"
          :aria-label="t('documents.moreActions', { title })"
        >
          <EllipsisVertical class="size-4" aria-hidden="true" />
        </DropdownMenuTrigger>
        <DropdownMenuPortal>
          <DropdownMenuContent
            class="z-50 min-w-44 rounded-md border border-border bg-surface-raised py-1.5 text-sm shadow-lg"
            align="end"
          >
            <DropdownMenuItem
              class="flex cursor-default items-center gap-3 px-4 py-1.5 text-danger outline-none data-highlighted:bg-surface-muted"
              @select="emit('delete')"
            >
              <Trash2 class="size-4" aria-hidden="true" />
              {{ t('documents.delete') }}
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenuPortal>
      </DropdownMenuRoot>
    </div>
  </li>
</template>
