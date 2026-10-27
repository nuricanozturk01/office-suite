<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { RouterLink, useRoute, useRouter } from 'vue-router'

import { RegisterForm } from '@/features/auth'
import { RouteName } from '@/shared/config/routes'
import { AuthShell } from '@/shared/ui'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()

/** Only same-app paths are honored to avoid open redirects. */
function redirectTarget(): string {
  const redirect = route.query.redirect
  return typeof redirect === 'string' && redirect.startsWith('/') && !redirect.startsWith('//')
    ? redirect
    : '/documents'
}

async function onSuccess(): Promise<void> {
  await router.replace(redirectTarget())
}

async function onRegistered(): Promise<void> {
  await router.replace({ name: RouteName.Login, query: { ...route.query, registered: '1' } })
}
</script>

<template>
  <AuthShell :title="t('auth.startHere')" :lead="t('auth.startLead')">
    <RegisterForm @success="onSuccess" @registered="onRegistered" />
    <p class="mt-8 text-sm text-text-muted">
      {{ t('auth.haveAccount') }}
      <RouterLink
        :to="{ name: RouteName.Login, query: route.query }"
        class="font-medium text-accent hover:underline"
      >
        {{ t('auth.signIn') }}
      </RouterLink>
    </p>
  </AuthShell>
</template>
