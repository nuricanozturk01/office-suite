<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'

import { errorMessageKey } from '@/shared/api'
import { useZodForm } from '@/shared/lib'
import { AppButton, InlineAlert, TextField } from '@/shared/ui'

import { useChangePassword } from '../api/use-profile-mutations'
import { changePasswordSchema } from '../model/schemas'

const { t } = useI18n()

const changePassword = useChangePassword()
const succeeded = ref(false)

const { values, errors, validate } = useZodForm(changePasswordSchema, {
  current_password: '',
  new_password: '',
  confirm_password: '',
})

function submit(): void {
  succeeded.value = false
  const parsed = validate()
  if (parsed === null) return
  changePassword.mutate(
    { current_password: parsed.current_password, new_password: parsed.new_password },
    {
      onSuccess: () => {
        succeeded.value = true
        values.current_password = ''
        values.new_password = ''
        values.confirm_password = ''
      },
    },
  )
}
</script>

<template>
  <form class="flex flex-col gap-6" novalidate @submit.prevent="submit">
    <InlineAlert v-if="changePassword.isError.value">
      {{ t(errorMessageKey(changePassword.error.value)) }}
    </InlineAlert>

    <InlineAlert v-if="succeeded" tone="success">
      {{ t('profile.security.success') }}
    </InlineAlert>

    <TextField
      v-model="values.current_password"
      type="password"
      autocomplete="current-password"
      :label="t('profile.security.current')"
      :error="errors.current_password && t(errors.current_password)"
    />

    <TextField
      v-model="values.new_password"
      type="password"
      autocomplete="new-password"
      :label="t('profile.security.newPassword')"
      :error="errors.new_password && t(errors.new_password)"
    />

    <TextField
      v-model="values.confirm_password"
      type="password"
      autocomplete="new-password"
      :label="t('profile.security.confirm')"
      :error="errors.confirm_password && t(errors.confirm_password)"
    />

    <div class="flex justify-end">
      <AppButton type="submit" :loading="changePassword.isPending.value">
        {{
          changePassword.isPending.value
            ? t('profile.security.submitting')
            : t('profile.security.submit')
        }}
      </AppButton>
    </div>
  </form>
</template>
