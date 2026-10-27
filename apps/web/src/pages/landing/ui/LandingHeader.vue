<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { RouterLink } from 'vue-router'

import { RouteName } from '@/shared/config/routes'
import { ThemeToggle } from '@/shared/ui'

const { t } = useI18n()
</script>

<template>
  <header class="mx-auto flex max-w-6xl items-center gap-6 px-5 py-5">
    <RouterLink :to="{ name: RouteName.Landing }" class="font-display text-xl font-semibold">
      {{ t('app.name') }}
    </RouterLink>
    <nav :aria-label="t('landing.nav.label')" class="hidden gap-6 text-sm text-text-muted md:flex">
      <a href="#tools" class="hover:text-text">{{ t('landing.nav.tools') }}</a>
      <a href="#formats" class="hover:text-text">{{ t('landing.nav.formats') }}</a>
      <a href="#sharing" class="hover:text-text">{{ t('landing.nav.sharing') }}</a>
    </nav>
    <div class="ms-auto flex items-center gap-2 text-sm">
      <ThemeToggle />
      <RouterLink
        :to="{ name: RouteName.Login }"
        class="rounded-md px-3 py-2 font-medium hover:bg-surface-muted"
      >
        {{ t('landing.signIn') }}
      </RouterLink>
      <RouterLink
        :to="{ name: RouteName.Register }"
        class="hidden rounded-md bg-ink px-3 py-2 font-medium text-white hover:opacity-90 sm:block"
      >
        {{ t('landing.signUp') }}
      </RouterLink>
    </div>
  </header>
</template>
