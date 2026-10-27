import { z } from 'zod'

export const updateProfileSchema = z.object({
  display_name: z.string().min(1, 'errors.validation_failed').max(100, 'errors.validation_failed'),
  avatar_url: z
    .string()
    .max(500, 'errors.validation_failed')
    .refine((v) => v === '' || v.startsWith('https://'), 'errors.validation_failed'),
  bio: z.string().max(300, 'errors.validation_failed'),
})

export const changePasswordSchema = z
  .object({
    current_password: z.string().min(1, 'errors.validation_failed'),
    new_password: z.string().min(8, 'errors.validation_failed'),
    confirm_password: z.string().min(1, 'errors.validation_failed'),
  })
  .refine((v) => v.new_password === v.confirm_password, {
    message: 'profile.errors.mismatch',
    path: ['confirm_password'],
  })
