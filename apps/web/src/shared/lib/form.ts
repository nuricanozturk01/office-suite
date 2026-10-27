import { reactive, ref } from 'vue'
import type { z } from 'zod'

/**
 * Minimal form state on top of a zod schema. Issue messages are expected to be i18n keys.
 */
export function useZodForm<Input extends Record<string, string>, Output>(
  schema: z.ZodType<Output, Input>,
  initial: Input,
) {
  const values = reactive({ ...initial })
  const errors = ref<Partial<Record<string, string>>>({})

  /** Returns parsed values, or `null` after populating `errors`. */
  function validate(): Output | null {
    const result = schema.safeParse(values)
    if (result.success) {
      errors.value = {}
      return result.data
    }
    const next: Partial<Record<string, string>> = {}
    for (const issue of result.error.issues) {
      const field = issue.path[0]
      if (typeof field === 'string' && next[field] === undefined) {
        next[field] = issue.message
      }
    }
    errors.value = next
    return null
  }

  return { values, errors, validate }
}
