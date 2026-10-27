<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'

import { errorMessageKey } from '@/shared/api'
import { AppButton, AppDialog, InlineAlert } from '@/shared/ui'

import { useProfile } from '../api/use-profile-query'
import { useCancelDeletion, useExportData, useScheduleDeletion } from '../api/use-profile-mutations'

const { t } = useI18n()

const profile = useProfile()
const exportData = useExportData()
const scheduleDeletion = useScheduleDeletion()
const cancelDeletion = useCancelDeletion()

const deleteDialogOpen = ref(false)
const confirmName = ref('')
const gdprExpanded = ref(false)
const cancelSucceeded = ref(false)

const deletionScheduledAt = computed(() => profile.data.value?.deletion_scheduled_at ?? null)

function openDeleteDialog(): void {
  confirmName.value = ''
  deleteDialogOpen.value = true
}

function submitDeletion(): void {
  scheduleDeletion.mutate(
    { confirm_display_name: confirmName.value },
    {
      onSuccess: () => {
        deleteDialogOpen.value = false
      },
    },
  )
}

function cancelDeletionRequest(): void {
  cancelSucceeded.value = false
  cancelDeletion.mutate(undefined, {
    onSuccess: () => {
      cancelSucceeded.value = true
    },
  })
}

function formatDate(iso: string): string {
  return new Intl.DateTimeFormat(undefined, { dateStyle: 'long' }).format(new Date(iso))
}
</script>

<template>
  <div class="flex flex-col gap-8">
    <!-- Scheduled deletion banner -->
    <div
      v-if="deletionScheduledAt !== null"
      class="rounded-md border border-danger bg-danger-surface px-4 py-3 text-sm text-danger"
      role="alert"
    >
      {{ t('profile.dataPrivacy.scheduledBanner', { date: formatDate(deletionScheduledAt) }) }}
      <button
        type="button"
        class="ms-2 underline hover:no-underline"
        :disabled="cancelDeletion.isPending.value"
        @click="cancelDeletionRequest"
      >
        {{
          cancelDeletion.isPending.value
            ? t('profile.dataPrivacy.cancellingDeletion')
            : t('profile.dataPrivacy.cancelDeletion')
        }}
      </button>
    </div>

    <InlineAlert v-if="cancelSucceeded" tone="success">
      {{ t('profile.dataPrivacy.deletionCancelled') }}
    </InlineAlert>

    <!-- Export section -->
    <section>
      <h2 class="mb-1 text-base font-semibold">{{ t('profile.dataPrivacy.exportTitle') }}</h2>
      <p class="mb-4 text-sm text-text-muted">{{ t('profile.dataPrivacy.exportDescription') }}</p>
      <InlineAlert v-if="exportData.isError.value" class="mb-3">
        {{ t(errorMessageKey(exportData.error.value)) }}
      </InlineAlert>
      <AppButton :loading="exportData.isPending.value" @click="exportData.mutate()">
        {{
          exportData.isPending.value
            ? t('profile.dataPrivacy.exporting')
            : t('profile.dataPrivacy.exportButton')
        }}
      </AppButton>
    </section>

    <!-- GDPR notice -->
    <section>
      <h2 class="mb-1 text-base font-semibold">{{ t('profile.dataPrivacy.gdprTitle') }}</h2>
      <p class="text-sm text-text-muted" :class="{ 'line-clamp-3': !gdprExpanded }">
        {{ t('profile.dataPrivacy.gdprText') }}
      </p>
      <button
        type="button"
        class="mt-1 text-sm text-accent hover:underline"
        @click="gdprExpanded = !gdprExpanded"
      >
        {{
          gdprExpanded ? t('profile.dataPrivacy.gdprCollapse') : t('profile.dataPrivacy.gdprExpand')
        }}
      </button>
    </section>

    <!-- Delete section (hidden if already scheduled) -->
    <section v-if="deletionScheduledAt === null">
      <h2 class="mb-1 text-base font-semibold text-danger">
        {{ t('profile.dataPrivacy.deleteTitle') }}
      </h2>
      <p class="mb-4 text-sm text-text-muted">{{ t('profile.dataPrivacy.deleteDescription') }}</p>
      <AppButton variant="danger" @click="openDeleteDialog">
        {{ t('profile.dataPrivacy.deleteButton') }}
      </AppButton>
    </section>

    <!-- Confirm deletion dialog -->
    <AppDialog
      v-model:open="deleteDialogOpen"
      :title="t('profile.dataPrivacy.deleteTitle')"
      :description="t('profile.dataPrivacy.deleteDescription')"
      :close-label="t('profile.dataPrivacy.deleteConfirmCancel')"
    >
      <div class="flex flex-col gap-4">
        <InlineAlert v-if="scheduleDeletion.isError.value">
          {{ t(errorMessageKey(scheduleDeletion.error.value)) }}
        </InlineAlert>

        <div class="flex flex-col gap-1.5">
          <label for="delete-confirm-name" class="text-sm font-medium">{{
            t('profile.dataPrivacy.deleteConfirmLabel')
          }}</label>
          <input
            id="delete-confirm-name"
            v-model="confirmName"
            type="text"
            autocomplete="off"
            :placeholder="t('profile.dataPrivacy.deleteConfirmPlaceholder')"
            class="h-11 w-full rounded-md border border-border bg-surface-raised px-3 text-sm outline-none focus:border-danger"
          />
        </div>

        <div class="flex justify-end gap-3">
          <AppButton variant="ghost" @click="deleteDialogOpen = false">
            {{ t('profile.dataPrivacy.deleteConfirmCancel') }}
          </AppButton>
          <AppButton
            variant="danger"
            :loading="scheduleDeletion.isPending.value || confirmName.trim() === ''"
            @click="submitDeletion"
          >
            {{ t('profile.dataPrivacy.deleteConfirmSubmit') }}
          </AppButton>
        </div>
      </div>
    </AppDialog>
  </div>
</template>
