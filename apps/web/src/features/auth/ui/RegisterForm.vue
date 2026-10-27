<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import { errorMessageKey } from '@/shared/api'
import { useZodForm } from '@/shared/lib'
import { AppButton, InlineAlert, TextField } from '@/shared/ui'

import { useLogin, useRegister } from '../api/mutations'
import { PASSWORD_MIN_LENGTH, registerSchema } from '../model/schemas'

/** `success`: registered and signed in. `registered`: account exists but the automatic sign-in failed. */
const emit = defineEmits<{ success: []; registered: [] }>()

const { t } = useI18n()
const register = useRegister()
const login = useLogin()
const busy = computed(() => register.isPending.value || login.isPending.value)
const { values, errors, validate } = useZodForm(registerSchema, {
  displayName: '',
  email: '',
  password: '',
})

function submit(): void {
  const parsed = validate()
  if (parsed === null) {
    return
  }
  register.mutate(
    { email: parsed.email, display_name: parsed.displayName, password: parsed.password },
    {
      onSuccess: () => {
        login.mutate(
          { email: parsed.email, password: parsed.password },
          {
            onSuccess: () => {
              emit('success')
            },
            onError: () => {
              emit('registered')
            },
          },
        )
      },
    },
  )
}
</script>

<template>
  <form class="flex flex-col gap-4" novalidate @submit.prevent="submit">
    <InlineAlert v-if="register.isError.value">
      {{ t(errorMessageKey(register.error.value)) }}
    </InlineAlert>
    <TextField
      v-model="values.displayName"
      autocomplete="name"
      focus-on-mount
      :label="t('auth.displayName')"
      :error="errors.displayName && t(errors.displayName)"
    />
    <TextField
      v-model="values.email"
      type="email"
      autocomplete="email"
      :label="t('auth.email')"
      :error="errors.email && t(errors.email)"
    />
    <TextField
      v-model="values.password"
      type="password"
      autocomplete="new-password"
      :label="t('auth.password')"
      :hint="t('auth.passwordHint', { min: PASSWORD_MIN_LENGTH })"
      :error="errors.password && t(errors.password, { min: PASSWORD_MIN_LENGTH })"
    />
    <AppButton type="submit" :loading="busy" class="h-11">
      {{ busy ? t('auth.creating') : t('auth.createAccount') }}
    </AppButton>
  </form>
</template>
