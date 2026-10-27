import { useMutation, useQueryClient } from '@tanstack/vue-query'
import { type MaybeRefOrGetter, toValue } from 'vue'

import { packageKeys, readPackageVersion, restorePackageVersion } from '@/entities/package'

/** Download and restore of the kept versions of one resource's file. */
export function useVersionActions(resourceId: MaybeRefOrGetter<string>, onRestored: () => void) {
  const queryClient = useQueryClient()

  const download = useMutation({
    mutationFn: async (version: number) => {
      const stored = await readPackageVersion(toValue(resourceId), version)
      const url = URL.createObjectURL(new Blob([stored.bytes]))
      const link = document.createElement('a')
      link.href = url
      link.download = stored.name
      link.click()
      URL.revokeObjectURL(url)
    },
  })

  const restore = useMutation({
    mutationFn: (version: number) => restorePackageVersion(toValue(resourceId), version),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: packageKeys.versions(toValue(resourceId)) })
      onRestored()
    },
  })

  return { download, restore }
}
