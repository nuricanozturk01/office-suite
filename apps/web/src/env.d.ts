/// <reference types="vite/client" />

declare const __VUE_I18N_LEGACY_API__: boolean

interface ImportMetaEnv {
  /** Backend origin, e.g. https://myoffice-api.example.com. Empty means same origin. */
  readonly VITE_API_URL?: string
}
