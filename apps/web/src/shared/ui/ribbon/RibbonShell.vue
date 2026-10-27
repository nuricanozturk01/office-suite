<script setup lang="ts" generic="T extends string">
import { TabsContent, TabsList, TabsRoot, TabsTrigger } from 'reka-ui'

/** A tabbed ribbon: tab strip with an optional leading slot, one panel per tab. */
const { tabs, label } = defineProps<{
  tabs: readonly { id: T; label: string }[]
  label: string
}>()

const active = defineModel<T>({ required: true })

defineSlots<{
  leading?: () => unknown
  trailing?: () => unknown
  [panel: `panel-${string}`]: () => unknown
}>()
</script>

<template>
  <TabsRoot v-model="active" class="bg-surface" activation-mode="manual">
    <div class="flex items-center gap-1 px-2">
      <slot name="leading" />
      <TabsList
        :aria-label="label"
        class="flex min-w-0 flex-1 items-center gap-0.5 overflow-x-auto"
      >
        <TabsTrigger
          v-for="tab in tabs"
          :key="tab.id"
          :value="tab.id"
          class="relative shrink-0 rounded-t px-3 py-1.5 text-sm text-text-muted outline-none hover:text-text focus-visible:ring-2 focus-visible:ring-focus data-[state=active]:text-text data-[state=active]:after:absolute data-[state=active]:after:inset-x-2 data-[state=active]:after:bottom-0 data-[state=active]:after:h-0.5 data-[state=active]:after:rounded data-[state=active]:after:bg-accent"
        >
          {{ tab.label }}
        </TabsTrigger>
      </TabsList>
      <slot name="trailing" />
    </div>
    <TabsContent
      v-for="tab in tabs"
      :key="tab.id"
      :value="tab.id"
      class="mx-2 mb-2 flex min-h-[5.5rem] items-stretch gap-0 overflow-x-auto rounded-lg border border-border bg-surface-raised px-1 py-1 outline-none"
    >
      <slot :name="`panel-${tab.id}`" />
    </TabsContent>
  </TabsRoot>
</template>
