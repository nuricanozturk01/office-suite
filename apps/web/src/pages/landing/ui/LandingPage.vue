<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { RouterLink } from 'vue-router'

import { RouteName } from '@/shared/config/routes'

import FormatsSection from './FormatsSection.vue'
import HeroDesk from './HeroDesk.vue'
import LandingHeader from './LandingHeader.vue'
import SharingSection from './SharingSection.vue'
import ToolsSection from './ToolsSection.vue'

const { t } = useI18n()
</script>

<template>
  <div class="min-h-dvh bg-surface">
    <LandingHeader />
    <main id="main" tabindex="-1">
      <section
        class="mx-auto grid max-w-6xl items-center gap-12 px-5 pt-10 pb-20 md:grid-cols-[1.05fr_1fr] md:pt-16"
      >
        <div>
          <h1 class="max-w-xl text-4xl leading-[1.05] font-semibold sm:text-5xl lg:text-6xl">
            {{ t('landing.hero.title') }}
          </h1>
          <p class="mt-6 max-w-md text-lg text-text-muted">{{ t('landing.hero.lead') }}</p>
          <div class="mt-8 flex flex-wrap items-center gap-3">
            <RouterLink
              :to="{ name: RouteName.Register }"
              class="rounded-md bg-accent px-5 py-3 font-medium text-accent-contrast hover:bg-accent-hover"
            >
              {{ t('landing.hero.primary') }}
            </RouterLink>
            <RouterLink
              :to="{ name: RouteName.Login }"
              class="rounded-md px-5 py-3 font-medium hover:bg-surface-muted"
            >
              {{ t('landing.hero.secondary') }}
            </RouterLink>
          </div>
        </div>
        <HeroDesk />
      </section>
      <ToolsSection />
      <FormatsSection />
      <SharingSection />
      <section aria-labelledby="cta-title" class="bg-ink py-16 text-white">
        <div class="mx-auto flex max-w-6xl flex-wrap items-center justify-between gap-6 px-5">
          <h2 id="cta-title" class="text-3xl font-semibold">{{ t('landing.cta.title') }}</h2>
          <RouterLink
            :to="{ name: RouteName.Register }"
            class="rounded-md bg-white px-5 py-3 font-medium text-ink hover:opacity-90"
          >
            {{ t('landing.cta.button') }}
          </RouterLink>
        </div>
      </section>
    </main>
    <footer class="mx-auto max-w-6xl px-5 py-8 text-sm text-text-muted">
      <p>{{ t('app.name') }}. {{ t('landing.footer.rights') }}</p>
    </footer>
  </div>
</template>
