<script setup lang="ts">
import { Eye, EyeOff } from '@lucide/vue'
import { computed, onMounted, ref, useId, useTemplateRef } from 'vue'
import { useI18n } from 'vue-i18n'

const {
  type = 'text',
  error,
  hint,
  focusOnMount,
} = defineProps<{
  label: string
  type?: 'text' | 'email' | 'password'
  autocomplete?: string
  error?: string | undefined
  /** Help shown under the field until there is an error. */
  hint?: string
  focusOnMount?: boolean
}>()

const model = defineModel<string>({ required: true })

const { t } = useI18n()
const id = useId()
const describedBy = computed(() => {
  if (error !== undefined) {
    return `${id}-error`
  }
  return hint === undefined ? undefined : `${id}-hint`
})
const revealed = ref(false)
const input = useTemplateRef<HTMLInputElement>('input')
const inputType = computed(() => (type === 'password' && revealed.value ? 'text' : type))

onMounted(() => {
  if (focusOnMount) {
    input.value?.focus()
  }
})
</script>

<template>
  <div class="flex flex-col gap-1.5">
    <label :for="id" class="text-sm font-medium">{{ label }}</label>
    <div class="relative">
      <input
        :id="id"
        ref="input"
        v-model="model"
        :type="inputType"
        :autocomplete="autocomplete"
        :aria-invalid="error !== undefined"
        :aria-describedby="describedBy"
        class="h-11 w-full rounded-md border border-border bg-surface-raised px-3 text-sm transition-colors outline-none focus:border-accent aria-invalid:border-danger"
        :class="type === 'password' ? 'pe-11' : ''"
      />
      <button
        v-if="type === 'password'"
        type="button"
        class="absolute inset-y-0 end-0 grid w-11 place-items-center rounded-e-md text-text-muted hover:text-text"
        :aria-label="revealed ? t('auth.hidePassword') : t('auth.showPassword')"
        :aria-pressed="revealed"
        @click="revealed = !revealed"
      >
        <EyeOff v-if="revealed" class="size-4" aria-hidden="true" />
        <Eye v-else class="size-4" aria-hidden="true" />
      </button>
    </div>
    <p v-if="error !== undefined" :id="`${id}-error`" class="text-sm text-danger">{{ error }}</p>
    <p v-else-if="hint !== undefined" :id="`${id}-hint`" class="text-sm text-text-muted">
      {{ hint }}
    </p>
  </div>
</template>
