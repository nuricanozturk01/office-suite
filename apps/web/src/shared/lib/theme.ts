import { readonly, ref } from 'vue'

export type ThemePreference = 'system' | 'light' | 'dark'

const STORAGE_KEY = 'office.theme'
const PREFERENCES: readonly ThemePreference[] = ['system', 'light', 'dark']

const preference = ref<ThemePreference>('system')
let started = false

function stored(): ThemePreference {
  try {
    const value = window.localStorage.getItem(STORAGE_KEY)
    return PREFERENCES.find((option) => option === value) ?? 'system'
  } catch {
    return 'system'
  }
}

function apply(): void {
  const dark =
    preference.value === 'dark' ||
    (preference.value === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches)
  document.documentElement.classList.toggle('dark', dark)
}

/** Reads the saved choice, applies it, and follows the system setting while it is "system". */
export function startTheme(): void {
  if (started) {
    return
  }
  started = true
  preference.value = stored()
  apply()
  window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', apply)
}

export function useTheme() {
  function set(next: ThemePreference): void {
    preference.value = next
    try {
      window.localStorage.setItem(STORAGE_KEY, next)
    } catch {
      // Private mode or blocked storage: the choice only lasts for this visit.
    }
    apply()
  }
  return { preference: readonly(preference), preferences: PREFERENCES, set }
}
