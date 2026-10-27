<script setup lang="ts">
import { useI18n } from 'vue-i18n'

import type { HeaderItem } from './use-grid-view'
import { COL_HEADER_HEIGHT, ROW_HEADER_WIDTH } from './viewport'

defineProps<{
  columns: HeaderItem[]
  rows: HeaderItem[]
  scrollLeft: number
  scrollTop: number
}>()

const emit = defineEmits<{
  selectAll: []
  selectColumn: [index: number, extend: boolean]
  selectRow: [index: number, extend: boolean]
  resize: [event: PointerEvent, index: number]
}>()

const { t } = useI18n()
</script>

<template>
  <button
    type="button"
    class="sheet-header border-e border-b border-[#e0e0e0]"
    :aria-label="t('sheets.selectAll')"
    @click="emit('selectAll')"
  />
  <div class="relative overflow-hidden border-b border-[#e0e0e0]">
    <div :style="{ transform: `translateX(${String(-scrollLeft)}px)` }">
      <div
        v-for="column in columns"
        :key="column.index"
        class="absolute top-0"
        :style="{
          left: `${String(column.x)}px`,
          width: `${String(column.size)}px`,
          height: `${String(COL_HEADER_HEIGHT)}px`,
        }"
      >
        <button
          type="button"
          class="sheet-header size-full border-e border-[#e0e0e0]"
          :class="{ 'sheet-header-selected': column.selected }"
          :aria-label="t('sheets.selectColumn', { col: column.label })"
          @click.shift.prevent="emit('selectColumn', column.index, true)"
          @click.exact.prevent="emit('selectColumn', column.index, false)"
        >
          {{ column.label }}
        </button>
        <span
          class="absolute inset-y-0 end-0 z-10 w-1 cursor-col-resize"
          role="separator"
          :aria-label="t('sheets.resizeColumn', { col: column.label })"
          @pointerdown.stop.prevent="emit('resize', $event, column.index)"
        />
      </div>
    </div>
  </div>
  <div class="relative overflow-hidden border-e border-[#e0e0e0]">
    <div :style="{ transform: `translateY(${String(-scrollTop)}px)` }">
      <button
        v-for="row in rows"
        :key="row.index"
        type="button"
        class="sheet-header absolute start-0 border-b border-[#e0e0e0]"
        :class="{ 'sheet-header-selected': row.selected }"
        :style="{
          top: `${String(row.y)}px`,
          height: `${String(row.size)}px`,
          width: `${String(ROW_HEADER_WIDTH)}px`,
        }"
        :aria-label="t('sheets.selectRow', { row: row.label })"
        @click.shift.prevent="emit('selectRow', row.index, true)"
        @click.exact.prevent="emit('selectRow', row.index, false)"
      >
        {{ row.label }}
      </button>
    </div>
  </div>
</template>
