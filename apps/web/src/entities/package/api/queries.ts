import { useQuery } from '@tanstack/vue-query'
import { type MaybeRefOrGetter, toValue } from 'vue'

import { packageKeys } from './keys'
import { listPackageVersions } from './package'

/** Kept versions of a resource's file, newest first; fetched only while `enabled`. */
export function usePackageVersionsQuery(
  resourceId: MaybeRefOrGetter<string>,
  enabled: MaybeRefOrGetter<boolean> = true,
) {
  return useQuery({
    queryKey: () => packageKeys.versions(toValue(resourceId)),
    queryFn: () => listPackageVersions(toValue(resourceId)),
    enabled: () => toValue(enabled),
    refetchOnMount: 'always',
  })
}
