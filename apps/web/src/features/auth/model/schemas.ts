import { z } from 'zod'

/** Mirrors identity domain rules (`modules/identity/core/src/domain/values.rs`). */
export const PASSWORD_MIN_LENGTH = 12

const email = z.email({ error: 'validation.email' })
const required = z.string().trim().min(1, { error: 'validation.required' })

export const loginSchema = z.object({
  email,
  password: z.string().min(1, { error: 'validation.required' }),
})

export const registerSchema = z.object({
  displayName: required.max(100),
  email,
  password: z.string().min(PASSWORD_MIN_LENGTH, { error: 'validation.passwordLength' }).max(256),
})
