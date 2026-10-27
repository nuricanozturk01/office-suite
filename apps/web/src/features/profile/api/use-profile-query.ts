import { useQuery } from '@tanstack/vue-query'

import { api, unwrap } from '@/shared/api'

import { profileKeys } from './keys'

export function useProfile() {
  return useQuery({
    queryKey: profileKeys.me(),
    queryFn: () => unwrap(api.GET('/api/v1/auth/me')),
  })
}
