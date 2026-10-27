import { createI18n } from 'vue-i18n'

import { en } from './locales/en'
import { type MessageSchema, tr } from './locales/tr'

export type Locale = 'tr' | 'en'

export function createAppI18n(locale: Locale = 'tr') {
  return createI18n<[MessageSchema], Locale>({
    locale,
    fallbackLocale: 'en',
    messages: { tr, en },
  })
}
