<script setup lang="ts">
import {
  Download,
  EllipsisVertical,
  ExternalLink,
  FileText,
  FileType,
  History,
  Pencil,
  Presentation,
  Share2,
  Table2,
  Trash2,
} from '@lucide/vue'
import {
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuPortal,
  DropdownMenuRoot,
  DropdownMenuTrigger,
} from 'reka-ui'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { RouterLink, useRouter } from 'vue-router'

import type { FileKind } from '@/features/file-actions'
import { RouteName } from '@/shared/config/routes'

const { kind, id, title, editedAt, selected } = defineProps<{
  kind: FileKind
  id: string
  title: string
  editedAt: string
  selected: boolean
}>()

const emit = defineEmits<{
  rename: []
  share: []
  history: []
  download: []
  delete: []
  toggle: []
}>()

const { t } = useI18n()
const router = useRouter()

const ROUTES = {
  document: RouteName.Document,
  spreadsheet: RouteName.Spreadsheet,
  presentation: RouteName.Presentation,
  pdf: RouteName.Pdf,
} as const
const ICONS = {
  document: FileText,
  spreadsheet: Table2,
  presentation: Presentation,
  pdf: FileType,
} as const
const TONES = {
  document: 'text-docs',
  spreadsheet: 'text-sheets',
  presentation: 'text-slides',
  pdf: 'text-pdf',
} as const

const to = computed(() => ({ name: ROUTES[kind], params: { id } }))
const ITEM =
  'flex cursor-default items-center gap-3 px-4 py-2 outline-none data-highlighted:bg-surface-muted'
</script>

<template>
  <li
    class="relative grid grid-cols-[auto_1fr_auto] items-center gap-x-3 border-b border-border px-3 py-2.5 hover:bg-surface-muted sm:grid-cols-[auto_1fr_7rem_8rem_auto]"
    :class="selected ? 'bg-accent/8' : ''"
  >
    <input
      type="checkbox"
      class="relative z-10 size-4 accent-[var(--color-accent)]"
      :checked="selected"
      :aria-label="t('files.select', { title })"
      @change="emit('toggle')"
    />
    <div class="flex min-w-0 items-center gap-3">
      <component
        :is="ICONS[kind]"
        :class="TONES[kind]"
        class="size-5 shrink-0"
        aria-hidden="true"
      />
      <RouterLink :to="to" class="truncate font-medium after:absolute after:inset-0">
        {{ title }}
      </RouterLink>
    </div>
    <span class="hidden text-sm text-text-muted sm:block">{{ t(`files.type.${kind}`) }}</span>
    <span class="hidden text-sm text-text-muted tabular-nums sm:block">{{ editedAt }}</span>
    <DropdownMenuRoot>
      <DropdownMenuTrigger
        class="relative z-10 rounded-full p-2 text-text-muted hover:bg-border"
        :aria-label="t('files.moreActions', { title })"
      >
        <EllipsisVertical class="size-4" aria-hidden="true" />
      </DropdownMenuTrigger>
      <DropdownMenuPortal>
        <DropdownMenuContent
          class="z-50 min-w-48 rounded-md border border-border bg-surface-raised py-1.5 text-sm shadow-lg"
          align="end"
        >
          <DropdownMenuItem :class="ITEM" @select="router.push(to)">
            <ExternalLink class="size-4" aria-hidden="true" />{{ t('fileActions.open') }}
          </DropdownMenuItem>
          <DropdownMenuItem :class="ITEM" @select="emit('rename')">
            <Pencil class="size-4" aria-hidden="true" />{{ t('fileActions.rename') }}
          </DropdownMenuItem>
          <DropdownMenuItem :class="ITEM" @select="emit('download')">
            <Download class="size-4" aria-hidden="true" />{{ t('fileActions.download') }}
          </DropdownMenuItem>
          <DropdownMenuItem :class="ITEM" @select="emit('share')">
            <Share2 class="size-4" aria-hidden="true" />{{ t('files.share') }}
          </DropdownMenuItem>
          <DropdownMenuItem :class="ITEM" @select="emit('history')">
            <History class="size-4" aria-hidden="true" />{{ t('history.menu') }}
          </DropdownMenuItem>
          <DropdownMenuItem :class="`${ITEM} text-danger`" @select="emit('delete')">
            <Trash2 class="size-4" aria-hidden="true" />{{ t('files.delete') }}
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenuPortal>
    </DropdownMenuRoot>
  </li>
</template>
