import { useMutation, useQueryClient } from '@tanstack/vue-query'

import { api, unwrap } from '@/shared/api'

import { profileKeys } from './keys'

const csrfHeader = { 'x-requested-with': 'office' } as const

export function useUpdateProfile() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: { display_name: string; bio?: string | null; avatar_url?: string | null }) =>
      unwrap(
        api.PATCH('/api/v1/auth/me', {
          body,
          params: { header: csrfHeader },
        }),
      ),
    onSuccess: () => qc.invalidateQueries({ queryKey: profileKeys.me() }),
  })
}

export function useChangePassword() {
  return useMutation({
    mutationFn: (body: { current_password: string; new_password: string }) =>
      unwrap(
        api.POST('/api/v1/auth/me/change-password', {
          body,
          params: { header: csrfHeader },
        }),
      ),
  })
}

export function useScheduleDeletion() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: { confirm_display_name: string }) =>
      unwrap(
        api.DELETE('/api/v1/auth/me', {
          body,
          params: { header: csrfHeader },
        }),
      ),
    onSuccess: () => qc.invalidateQueries({ queryKey: profileKeys.me() }),
  })
}

export function useCancelDeletion() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: () =>
      unwrap(
        api.POST('/api/v1/auth/me/cancel-deletion', {
          params: { header: csrfHeader },
        }),
      ),
    onSuccess: () => qc.invalidateQueries({ queryKey: profileKeys.me() }),
  })
}

export function useExportData() {
  return useMutation({
    mutationFn: async () => {
      const { data, error } = await api.GET('/api/v1/auth/me/export')
      if (error) throw new Error(JSON.stringify(error))
      return data
    },
    onSuccess: (data) => {
      const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' })
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url
      a.download = 'my-data.json'
      document.body.appendChild(a)
      a.click()
      document.body.removeChild(a)
      URL.revokeObjectURL(url)
    },
  })
}
