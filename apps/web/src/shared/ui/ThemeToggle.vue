<script setup lang="ts">
import { Check, Monitor, Moon, Sun } from '@lucide/vue'
import {
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuPortal,
  DropdownMenuRoot,
  DropdownMenuTrigger,
} from 'reka-ui'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import { useTheme } from '@/shared/lib'

const { t } = useI18n()
const theme = useTheme()

const ICONS = { system: Monitor, light: Sun, dark: Moon } as const
const current = computed(() => ICONS[theme.preference.value])
</script>

<template>
  <DropdownMenuRoot>
    <DropdownMenuTrigger
      class="grid size-9 place-items-center rounded-full text-text-muted hover:bg-surface-muted hover:text-text"
      :aria-label="t('theme.label')"
      :title="t('theme.label')"
    >
      <component :is="current" class="size-4" aria-hidden="true" />
    </DropdownMenuTrigger>
    <DropdownMenuPortal>
      <DropdownMenuContent
        class="z-50 min-w-40 rounded-md border border-border bg-surface-raised py-1.5 text-sm text-text shadow-lg"
        align="end"
        :side-offset="6"
      >
        <DropdownMenuItem
          v-for="option in theme.preferences"
          :key="option"
          class="flex cursor-default items-center gap-3 px-3 py-2 outline-none data-highlighted:bg-surface-muted"
          @select="theme.set(option)"
        >
          <component :is="ICONS[option]" class="size-4" aria-hidden="true" />
          <span class="flex-1">{{ t(`theme.${option}`) }}</span>
          <Check v-if="theme.preference.value === option" class="size-4" aria-hidden="true" />
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenuPortal>
  </DropdownMenuRoot>
</template>
