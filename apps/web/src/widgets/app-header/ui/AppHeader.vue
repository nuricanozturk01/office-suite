<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { RouterLink, useRouter } from 'vue-router'

import { UserMenu } from '@/features/auth'
import { RouteName } from '@/shared/config/routes'
import { ThemeToggle } from '@/shared/ui'

const { t } = useI18n()
const router = useRouter()

async function toLanding(): Promise<void> {
  await router.push({ name: RouteName.Landing })
}
</script>

<template>
  <header
    class="sticky top-0 z-10 flex items-center gap-4 border-b border-border bg-surface px-4 py-2.5"
  >
    <RouterLink
      :to="{ name: RouteName.Documents }"
      class="w-48 shrink-0 rounded-md px-1 font-display text-xl font-semibold"
    >
      {{ t('app.name') }}
    </RouterLink>
    <div class="flex min-w-0 flex-1 justify-start"><slot /></div>
    <ThemeToggle />
    <UserMenu @signed-out="toLanding" />
  </header>
</template>
