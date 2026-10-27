import { useMutation } from '@tanstack/vue-query'

import { useSessionStore } from '@/entities/session'
import { api, unwrap } from '@/shared/api'

export function useLogin() {
  const session = useSessionStore()
  return useMutation({
    mutationFn: (body: { email: string; password: string }) =>
      unwrap(api.POST('/api/v1/auth/login', { body })),
    onSuccess: (login) => {
      session.signIn(login)
    },
  })
}

export function useRegister() {
  return useMutation({
    mutationFn: (body: { email: string; display_name: string; password: string }) =>
      unwrap(api.POST('/api/v1/auth/register', { body })),
  })
}
