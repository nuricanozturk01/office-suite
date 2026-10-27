<script setup lang="ts">
import { useI18n } from 'vue-i18n'

import { errorMessageKey } from '@/shared/api'
import { useZodForm } from '@/shared/lib'
import { AppButton, InlineAlert, TextField } from '@/shared/ui'

import { useLogin } from '../api/mutations'
import { loginSchema } from '../model/schemas'

const emit = defineEmits<{ success: [] }>()

const { t } = useI18n()
const login = useLogin()
const { values, errors, validate } = useZodForm(loginSchema, { email: '', password: '' })

function submit(): void {
  const parsed = validate()
  if (parsed === null) {
    return
  }
  login.mutate(parsed, {
    onSuccess: () => {
      emit('success')
    },
  })
}
</script>

<template>
  <form class="flex flex-col gap-4" novalidate @submit.prevent="submit">
    <InlineAlert v-if="login.isError.value">{{
      t(errorMessageKey(login.error.value))
    }}</InlineAlert>
    <TextField
      v-model="values.email"
      type="email"
      autocomplete="email"
      focus-on-mount
      :label="t('auth.email')"
      :error="errors.email && t(errors.email)"
    />
    <TextField
      v-model="values.password"
      type="password"
      autocomplete="current-password"
      :label="t('auth.password')"
      :error="errors.password && t(errors.password)"
    />
    <AppButton type="submit" :loading="login.isPending.value" class="h-11">
      {{ login.isPending.value ? t('auth.signingIn') : t('auth.signIn') }}
    </AppButton>
  </form>
</template>
