<script setup lang="ts">
import { Ban } from '@lucide/vue'
import {
  PopoverClose,
  PopoverContent,
  PopoverPortal,
  PopoverRoot,
  PopoverTrigger,
  ToolbarButton,
} from 'reka-ui'
import type { Component } from 'vue'
import { useI18n } from 'vue-i18n'

import { COLOR_PALETTE } from './palette'

const { label, icon, current, fallback } = defineProps<{
  label: string
  icon: Component
  current: string | undefined
  fallback: string
}>()

const emit = defineEmits<{ pick: [color: string | null] }>()

const { t } = useI18n()
const buttonClass =
  'inline-flex h-7 min-w-7 flex-col items-center justify-center gap-0 rounded px-1 text-text outline-none hover:bg-surface-muted focus-visible:ring-2 focus-visible:ring-focus'
</script>

<template>
  <PopoverRoot>
    <PopoverTrigger as-child>
      <ToolbarButton :class="buttonClass" :aria-label="label" :title="label" @mousedown.prevent>
        <component :is="icon" class="size-4" aria-hidden="true" />
        <span
          class="h-1 w-4 rounded-sm border border-border"
          :style="{ backgroundColor: current ?? fallback }"
        />
      </ToolbarButton>
    </PopoverTrigger>
    <PopoverPortal>
      <PopoverContent
        class="z-50 rounded-md border border-border bg-surface-raised p-3 shadow-lg"
        :side-offset="4"
        align="start"
        @close-auto-focus.prevent
      >
        <PopoverClose
          class="mb-2 flex w-full items-center gap-2 rounded px-2 py-1 text-sm hover:bg-surface-muted"
          @click="emit('pick', null)"
        >
          <Ban class="size-4" aria-hidden="true" />
          {{ t('editor.actions.noColor') }}
        </PopoverClose>
        <div class="grid grid-cols-8 gap-1" role="group" :aria-label="label">
          <template v-for="row in COLOR_PALETTE" :key="row[0]">
            <PopoverClose
              v-for="color in row"
              :key="color"
              class="size-5 rounded-full border border-border hover:scale-110 focus-visible:ring-2 focus-visible:ring-focus"
              :class="{ 'ring-2 ring-accent': current === color }"
              :style="{ backgroundColor: color }"
              :aria-label="color"
              :aria-pressed="current === color"
              :title="color"
              @click="emit('pick', color)"
            />
          </template>
        </div>
      </PopoverContent>
    </PopoverPortal>
  </PopoverRoot>
</template>
