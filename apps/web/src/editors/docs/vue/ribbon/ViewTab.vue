<script setup lang="ts">
import { List, Ruler, ZoomIn, ZoomOut } from '@lucide/vue'
import { useI18n } from 'vue-i18n'

import { RibbonButton, RibbonGroup, RibbonSelect } from '@/shared/ui'

import { ZOOM_LEVELS } from '../../core/formatting'
import { useDocsView } from '../docs-view'

const { t } = useI18n()
const { zoom, outlineOpen, rulerOpen } = useDocsView()

function step(direction: 1 | -1): void {
  const next =
    direction === 1
      ? ZOOM_LEVELS.find((level) => level > zoom.value)
      : [...ZOOM_LEVELS].reverse().find((level) => level < zoom.value)
  zoom.value = next ?? zoom.value
}
</script>

<template>
  <RibbonGroup :label="t('ribbon.group.show')">
    <RibbonButton
      :icon="Ruler"
      :label="t('ribbon.ruler')"
      size="large"
      :active="rulerOpen"
      @click="rulerOpen = !rulerOpen"
    />
    <RibbonButton
      :icon="List"
      :label="t('editor.actions.showOutline')"
      size="large"
      :active="outlineOpen"
      @click="outlineOpen = !outlineOpen"
    />
  </RibbonGroup>
  <RibbonGroup :label="t('ribbon.group.zoom')">
    <RibbonButton :icon="ZoomOut" :label="t('ribbon.zoomOut')" size="large" @click="step(-1)" />
    <RibbonSelect
      class="mt-2 w-20"
      :label="t('editor.actions.zoom')"
      :options="ZOOM_LEVELS"
      :value="zoom as (typeof ZOOM_LEVELS)[number]"
      :display="(level) => `${String(level)}%`"
      @select="(level) => (zoom = level)"
    />
    <RibbonButton :icon="ZoomIn" :label="t('ribbon.zoomIn')" size="large" @click="step(1)" />
  </RibbonGroup>
</template>
