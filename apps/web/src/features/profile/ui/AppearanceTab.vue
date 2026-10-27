<script setup lang="ts">
import { useI18n } from 'vue-i18n'

import type { Locale } from '@/shared/i18n'
import type { ThemePreference } from '@/shared/lib'
import { useTheme } from '@/shared/lib'

const { t, locale } = useI18n()
const theme = useTheme()

const LANG_KEY = 'office.lang'
const LOCALES: Locale[] = ['tr', 'en']

function setLocale(next: Locale): void {
  locale.value = next
  try {
    window.localStorage.setItem(LANG_KEY, next)
  } catch {
    // storage blocked
  }
}
</script>

<template>
  <div class="flex flex-col gap-8">
    <section>
      <h2 class="mb-4 text-sm font-semibold tracking-wide text-text-muted uppercase">
        {{ t('profile.appearance.themeLabel') }}
      </h2>
      <div class="flex gap-3" role="radiogroup" :aria-label="t('profile.appearance.themeLabel')">
        <label
          v-for="option in theme.preferences"
          :key="option"
          class="flex cursor-pointer items-center gap-2 rounded-md border px-4 py-3 text-sm transition-colors"
          :class="
            theme.preference.value === option
              ? 'border-accent bg-accent/10 font-medium text-accent'
              : 'border-border hover:border-accent/50'
          "
        >
          <input
            type="radio"
            name="theme"
            :value="option"
            :checked="theme.preference.value === option"
            class="sr-only"
            @change="theme.set(option as ThemePreference)"
          />
          {{ t(`profile.appearance.themes.${option}`) }}
        </label>
      </div>
    </section>

    <section>
      <h2 class="mb-4 text-sm font-semibold tracking-wide text-text-muted uppercase">
        {{ t('profile.appearance.languageLabel') }}
      </h2>
      <div class="flex gap-3" role="radiogroup" :aria-label="t('profile.appearance.languageLabel')">
        <label
          v-for="lang in LOCALES"
          :key="lang"
          class="flex cursor-pointer items-center gap-2 rounded-md border px-4 py-3 text-sm transition-colors"
          :class="
            locale === lang
              ? 'border-accent bg-accent/10 font-medium text-accent'
              : 'border-border hover:border-accent/50'
          "
        >
          <input
            type="radio"
            name="language"
            :value="lang"
            :checked="locale === lang"
            class="sr-only"
            @change="setLocale(lang)"
          />
          {{ t(`profile.appearance.languages.${lang}`) }}
        </label>
      </div>
    </section>
  </div>
</template>
