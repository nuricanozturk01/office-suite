<script setup lang="ts">
import { reactive, useTemplateRef } from 'vue'
import { useI18n } from 'vue-i18n'

import { COL_COUNT, ROW_COUNT, ROW_HEIGHT } from '../core/workbook'
import './grid.css'
import SheetHeaders from './SheetHeaders.vue'
import { useGridPointer } from './use-grid-pointer'
import { useGridView } from './use-grid-view'
import { useSheet } from './use-sheet-model'
import { ROW_HEADER_WIDTH, COL_HEADER_HEIGHT } from './viewport'

const { t } = useI18n()
const model = useSheet()
const ui = reactive(model)
const scroller = useTemplateRef('scroller')
const editor = useTemplateRef('editor')
const viewModel = useGridView(model, scroller)
const view = reactive(viewModel)
const pointer = useGridPointer(model, scroller, editor, (event) => viewModel.cellFrom(event))

const box = (value: number) => `${String(value)}px`
</script>

<template>
  <div
    class="grid min-h-0 min-w-0 flex-1"
    :style="{
      gridTemplateColumns: `${String(ROW_HEADER_WIDTH)}px 1fr`,
      gridTemplateRows: `${String(COL_HEADER_HEIGHT)}px 1fr`,
    }"
  >
    <SheetHeaders
      :columns="view.columns"
      :rows="view.rows"
      :scroll-left="view.scrollLeft"
      :scroll-top="view.scrollTop"
      @select-all="ui.selectAll()"
      @select-column="(index, extend) => ui.selectColumn(index, extend)"
      @select-row="(index, extend) => ui.selectRow(index, extend)"
      @resize="pointer.startResize"
    />
    <div
      ref="scroller"
      class="sheet-paper relative overflow-auto outline-none"
      role="grid"
      tabindex="0"
      :aria-label="t('sheets.grid')"
      :aria-rowcount="ROW_COUNT"
      :aria-colcount="COL_COUNT"
      @scroll="view.onScroll"
      @pointerdown="pointer.onPointerDown"
      @pointermove="pointer.onPointerMove"
      @pointerup="pointer.onPointerUp"
      @dblclick="pointer.onDoubleClick"
      @keydown="ui.onKeydown"
      @copy="pointer.onCopy"
      @cut="pointer.onCut"
      @paste="pointer.onPaste"
    >
      <div
        class="relative"
        :style="{ width: box(view.layout.total), height: box(ROW_COUNT * ROW_HEIGHT) }"
      >
        <div
          v-for="cell in view.cells"
          :key="cell.key"
          class="sheet-cell"
          :class="{ 'bg-[#e8f0fe]': cell.selected && cell.fill === undefined }"
          :style="{
            left: box(cell.x),
            top: box(cell.y),
            width: box(cell.w),
            height: box(ROW_HEIGHT),
            fontWeight: cell.bold ? '700' : undefined,
            fontStyle: cell.italic ? 'italic' : undefined,
            textDecoration: cell.underline ? 'underline' : undefined,
            textAlign: cell.align,
            color: cell.color,
            backgroundColor: cell.fill,
          }"
        >
          {{ cell.text }}
        </div>
        <div
          class="pointer-events-none absolute z-10 border-2 border-[#1a73e8]"
          :style="{
            left: box(view.selection.left),
            top: box(view.selection.top),
            width: box(view.selection.width),
            height: box(view.selection.height),
          }"
        />
        <button
          v-if="ui.editing === null"
          type="button"
          class="absolute z-20 size-2 cursor-crosshair border border-white bg-[#1a73e8]"
          :style="{
            left: box(view.selection.left + view.selection.width - 4),
            top: box(view.selection.top + view.selection.height - 4),
          }"
          :aria-label="t('sheets.fill')"
          @pointerdown.stop.prevent="pointer.startFill"
        />
        <input
          v-if="view.editorBox"
          ref="editor"
          class="absolute z-30 border-2 border-[#1a73e8] bg-white px-1 outline-none"
          :style="view.editorBox"
          :value="ui.editing?.draft ?? ''"
          :aria-label="t('sheets.formulaBar')"
          @input="pointer.onDraft"
          @keydown="pointer.onEditorKeydown"
          @blur="ui.commit(null)"
        />
      </div>
    </div>
  </div>
</template>
