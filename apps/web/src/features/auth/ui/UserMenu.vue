<script setup lang="ts">
import { LogOut, Settings } from '@lucide/vue'
import { useQueryClient } from '@tanstack/vue-query'
import {
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuPortal,
  DropdownMenuRoot,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from 'reka-ui'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'

import { useSessionStore } from '@/entities/session'
import { RouteName } from '@/shared/config/routes'
import { userColor } from '@/shared/lib'

const emit = defineEmits<{ signedOut: [] }>()

const { t } = useI18n()
const router = useRouter()
const session = useSessionStore()
const queryClient = useQueryClient()

function signOut(): void {
  void session.logout()
  queryClient.clear()
  emit('signedOut')
}

async function goToProfile(): Promise<void> {
  await router.push({ name: RouteName.Profile })
}
</script>

<template>
  <DropdownMenuRoot v-if="session.user">
    <DropdownMenuTrigger
      class="shrink-0 rounded-full hover:ring-4 hover:ring-surface-muted focus-visible:ring-2 focus-visible:ring-focus"
      :aria-label="t('auth.accountMenu', { name: session.user.display_name })"
      :title="t('auth.accountMenu', { name: session.user.display_name })"
    >
      <img
        v-if="session.user.avatar_url"
        :src="session.user.avatar_url"
        :alt="session.user.display_name"
        class="size-9 rounded-full object-cover"
        referrerpolicy="no-referrer"
      />
      <span
        v-else
        class="grid size-9 place-items-center rounded-full text-sm font-medium text-white"
        :style="{ backgroundColor: userColor(session.user.id) }"
        aria-hidden="true"
        >{{ session.user.display_name.trim().charAt(0).toLocaleUpperCase() }}</span
      >
    </DropdownMenuTrigger>
    <DropdownMenuPortal>
      <DropdownMenuContent
        class="z-50 min-w-64 rounded-xl border border-border bg-surface-raised p-2 text-sm text-text shadow-lg"
        align="end"
        :side-offset="6"
      >
        <DropdownMenuLabel class="flex items-center gap-3 px-3 py-2">
          <img
            v-if="session.user.avatar_url"
            :src="session.user.avatar_url"
            :alt="session.user.display_name"
            class="size-8 rounded-full object-cover"
            referrerpolicy="no-referrer"
          />
          <span
            v-else
            class="grid size-8 shrink-0 place-items-center rounded-full text-xs font-medium text-white"
            :style="{ backgroundColor: userColor(session.user.id) }"
            aria-hidden="true"
            >{{ session.user.display_name.trim().charAt(0).toLocaleUpperCase() }}</span
          >
          <span class="flex min-w-0 flex-col">
            <span class="truncate font-medium">{{ session.user.display_name }}</span>
            <span class="truncate text-text-muted">{{ session.user.email }}</span>
          </span>
        </DropdownMenuLabel>
        <DropdownMenuSeparator class="my-1 h-px bg-border" />
        <DropdownMenuItem
          class="flex cursor-default items-center gap-3 rounded-md px-3 py-2 outline-none data-highlighted:bg-surface-muted"
          @select="goToProfile"
        >
          <Settings class="size-4" aria-hidden="true" />
          {{ t('auth.profile') }}
        </DropdownMenuItem>
        <DropdownMenuSeparator class="my-1 h-px bg-border" />
        <DropdownMenuItem
          class="flex cursor-default items-center gap-3 rounded-md px-3 py-2 outline-none data-highlighted:bg-surface-muted"
          @select="signOut"
        >
          <LogOut class="size-4" aria-hidden="true" />
          {{ t('auth.signOut') }}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenuPortal>
  </DropdownMenuRoot>
</template>
