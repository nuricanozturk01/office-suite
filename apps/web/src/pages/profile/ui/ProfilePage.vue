<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'

import { AppearanceTab, DataPrivacyTab, ProfileTab, SecurityTab } from '@/features/profile'
import { AppHeader } from '@/widgets/app-header'

type TabId = 'profile' | 'security' | 'appearance' | 'dataPrivacy'

const { t } = useI18n()

const activeTab = ref<TabId>('profile')

const TABS: { id: TabId; label: string }[] = [
  { id: 'profile', label: t('profile.tabs.profile') },
  { id: 'security', label: t('profile.tabs.security') },
  { id: 'appearance', label: t('profile.tabs.appearance') },
  { id: 'dataPrivacy', label: t('profile.tabs.dataPrivacy') },
]
</script>

<template>
  <div class="flex min-h-dvh flex-col bg-surface">
    <AppHeader />

    <main class="mx-auto w-full max-w-3xl flex-1 px-4 py-8">
      <h1 class="mb-8 text-2xl font-semibold">{{ t('profile.title') }}</h1>

      <div class="flex flex-col gap-6 sm:flex-row">
        <!-- Sidebar nav -->
        <nav class="shrink-0 sm:w-44" aria-label="Profile sections">
          <ul class="flex gap-2 overflow-x-auto sm:flex-col sm:overflow-x-visible">
            <li v-for="tab in TABS" :key="tab.id">
              <button
                type="button"
                class="w-full rounded-md px-3 py-2 text-left text-sm transition-colors"
                :class="
                  activeTab === tab.id
                    ? 'bg-surface-muted font-medium text-text'
                    : 'text-text-muted hover:bg-surface-muted hover:text-text'
                "
                :aria-current="activeTab === tab.id ? 'page' : undefined"
                @click="activeTab = tab.id"
              >
                {{ tab.label }}
              </button>
            </li>
          </ul>
        </nav>

        <!-- Tab panel -->
        <div class="flex-1 rounded-lg border border-border bg-surface-raised p-6">
          <ProfileTab v-if="activeTab === 'profile'" />
          <SecurityTab v-else-if="activeTab === 'security'" />
          <AppearanceTab v-else-if="activeTab === 'appearance'" />
          <DataPrivacyTab v-else-if="activeTab === 'dataPrivacy'" />
        </div>
      </div>
    </main>
  </div>
</template>
