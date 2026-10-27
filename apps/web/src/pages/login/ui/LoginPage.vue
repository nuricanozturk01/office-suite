<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { RouterLink, useRoute, useRouter } from 'vue-router'

import { LoginForm } from '@/features/auth'
import { RouteName } from '@/shared/config/routes'
import { AuthShell, InlineAlert } from '@/shared/ui'

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
</script>

<template>
  <AuthShell :title="t('auth.welcomeBack')" :lead="t('auth.signInLead')">
    <InlineAlert v-if="route.query.registered" tone="success" class="mb-4">
      {{ t('auth.autoLoginFailed') }}
    </InlineAlert>
    <LoginForm @success="onSuccess" />
    <p class="mt-8 text-sm text-text-muted">
      {{ t('auth.noAccount') }}
      <RouterLink
        :to="{
          name: RouteName.Register,
          query: route.query.redirect ? { redirect: route.query.redirect } : {},
        }"
        class="font-medium text-accent hover:underline"
      >
        {{ t('auth.signUp') }}
      </RouterLink>
    </p>
  </AuthShell>
</template>
