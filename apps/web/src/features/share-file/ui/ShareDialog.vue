<script setup lang="ts">
import { X } from '@lucide/vue'
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'

import { useSharesQuery } from '@/entities/share'
import { errorMessageKey } from '@/shared/api'
import { AppButton, AppDialog, InlineAlert, TextField } from '@/shared/ui'

import { type GrantableRole, useShareFile } from '../model/use-share-file'

import RoleSelect from './RoleSelect.vue'

const { fileId, title } = defineProps<{ fileId: string; title: string }>()

const open = defineModel<boolean>('open', { required: true })

const { t } = useI18n()
const shares = useSharesQuery(
  () => fileId,
  () => open.value,
)
const { share, revoke } = useShareFile(() => fileId)

const email = ref('')
const role = ref<GrantableRole>('editor')
const emailError = ref<string>()

function submit(): void {
  const address = email.value.trim()
  if (address === '') {
    emailError.value = t('validation.required')
    return
  }
  emailError.value = undefined
  share.mutate(
    { email: address, role: role.value },
    {
      onSuccess: () => {
        email.value = ''
      },
    },
  )
}

function changeRole(address: string, next: GrantableRole): void {
  share.mutate({ email: address, role: next })
}
</script>

<template>
  <AppDialog
    v-model:open="open"
    :title="t('share.title', { title })"
    :description="t('share.description')"
    :close-label="t('share.close')"
  >
    <form class="flex flex-col gap-3" novalidate @submit.prevent="submit">
      <TextField
        v-model="email"
        type="email"
        autocomplete="off"
        :label="t('share.email')"
        :error="emailError"
      />
      <div class="flex items-end gap-2">
        <div class="flex flex-1 flex-col gap-1">
          <span class="text-sm font-medium" aria-hidden="true">{{ t('share.role') }}</span>
          <RoleSelect :value="role" :label="t('share.role')" @change="role = $event" />
        </div>
        <AppButton type="submit" :loading="share.isPending.value">{{ t('share.add') }}</AppButton>
      </div>
      <p v-if="share.isPending.value && share.failureCount.value > 0" role="status" class="text-sm">
        {{ t('share.retrying') }}
      </p>
      <InlineAlert v-if="share.isError.value">
        {{ t(errorMessageKey(share.error.value)) }}
      </InlineAlert>
    </form>

    <h3 class="mt-6 text-sm font-medium">{{ t('share.people') }}</h3>
    <InlineAlert v-if="shares.isError.value" class="mt-2">
      {{ t(errorMessageKey(shares.error.value)) }}
    </InlineAlert>
    <InlineAlert v-if="revoke.isError.value" class="mt-2">
      {{ t(errorMessageKey(revoke.error.value)) }}
    </InlineAlert>
    <p v-if="shares.data.value?.items.length === 0" class="mt-2 text-sm text-text-muted">
      {{ t('share.empty') }}
    </p>
    <ul v-if="shares.data.value" class="mt-2 flex flex-col divide-y divide-border">
      <li
        v-for="person in shares.data.value.items"
        :key="person.user_id"
        class="flex items-center gap-2 py-2"
      >
        <span class="min-w-0 flex-1 truncate text-sm">{{ person.email || person.user_id }}</span>
        <RoleSelect
          :value="person.role"
          :label="t('share.roleOf', { email: person.email || person.user_id })"
          @change="changeRole(person.email, $event)"
        />
        <button
          type="button"
          class="rounded-full p-1.5 text-text-muted hover:bg-surface-muted"
          :aria-label="t('share.remove', { email: person.email || person.user_id })"
          @click="revoke.mutate(person.user_id)"
        >
          <X class="size-4" aria-hidden="true" />
        </button>
      </li>
    </ul>
  </AppDialog>
</template>
