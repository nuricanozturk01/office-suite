<script setup lang="ts">
import { watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { errorMessageKey } from '@/shared/api'
import { useZodForm } from '@/shared/lib'
import { AppButton, InlineAlert, TextField } from '@/shared/ui'

import { useProfile } from '../api/use-profile-query'
import { useUpdateProfile } from '../api/use-profile-mutations'
import { updateProfileSchema } from '../model/schemas'

const { t } = useI18n()

const profile = useProfile()
const update = useUpdateProfile()

const { values, errors, validate } = useZodForm(updateProfileSchema, {
  display_name: '',
  avatar_url: '',
  bio: '',
})

watch(
  () => profile.data.value,
  (data) => {
    if (data === undefined) return
    values.display_name = data.display_name
    values.avatar_url = data.avatar_url ?? ''
    values.bio = data.bio ?? ''
  },
  { immediate: true },
)

function submit(): void {
  const parsed = validate()
  if (parsed === null) return
  update.mutate({
    display_name: parsed.display_name,
    bio: parsed.bio === '' ? null : parsed.bio,
    avatar_url: parsed.avatar_url === '' ? null : parsed.avatar_url,
  })
}
</script>

<template>
  <form class="flex flex-col gap-6" novalidate @submit.prevent="submit">
    <div v-if="profile.isLoading.value" class="h-48 animate-pulse rounded-lg bg-surface-muted" />

    <template v-else>
      <InlineAlert v-if="update.isError.value">
        {{ t(errorMessageKey(update.error.value)) }}
      </InlineAlert>

      <InlineAlert v-if="update.isSuccess.value" tone="success">
        {{ t('profile.saved') }}
      </InlineAlert>

      <TextField
        v-model="values.display_name"
        :label="t('profile.displayName')"
        autocomplete="name"
        :error="errors.display_name && t(errors.display_name)"
      />

      <div class="flex flex-col gap-1.5">
        <label for="profile-email" class="text-sm font-medium">{{ t('profile.email') }}</label>
        <input
          id="profile-email"
          :value="profile.data.value?.email ?? ''"
          type="email"
          disabled
          class="h-11 w-full cursor-not-allowed rounded-md border border-border bg-surface px-3 text-sm text-text-muted opacity-60"
          aria-readonly="true"
        />
      </div>

      <TextField
        v-model="values.avatar_url"
        :label="t('profile.avatarUrl')"
        autocomplete="url"
        :hint="t('profile.avatarUrlHint')"
        :error="errors.avatar_url && t(errors.avatar_url)"
      />

      <div class="flex flex-col gap-1.5">
        <label for="profile-bio" class="text-sm font-medium">{{ t('profile.bio') }}</label>
        <textarea
          id="profile-bio"
          v-model="values.bio"
          rows="3"
          maxlength="300"
          :aria-invalid="errors.bio !== undefined"
          class="w-full resize-none rounded-md border border-border bg-surface-raised px-3 py-2 text-sm transition-colors outline-none focus:border-accent aria-invalid:border-danger"
        />
        <p v-if="errors.bio !== undefined" class="text-sm text-danger">{{ t(errors.bio) }}</p>
        <p v-else class="text-right text-sm text-text-muted">{{ values.bio?.length ?? 0 }}/300</p>
      </div>

      <div class="flex justify-end gap-3">
        <AppButton type="submit" :loading="update.isPending.value">
          {{ update.isPending.value ? t('profile.saving') : t('profile.save') }}
        </AppButton>
      </div>
    </template>
  </form>
</template>
