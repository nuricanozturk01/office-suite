import { type Ref, ref } from 'vue'

import { readPackageVersion } from '@/entities/package'
import type { EditorSession } from '@/editors/embed'
import { useVersionActions } from '@/features/version-history'

/**
 * Showing an older version of the open file in its editor: view it, make it the latest, or go
 * back. `previewing` follows the session, so it also clears when something else reloads the file.
 */
export function useVersionPreview(
  session: () => EditorSession | undefined,
  resourceId: () => string,
) {
  const previewing: Ref<number | null> = ref(null)
  const savedAt: Ref<Date | null> = ref(null)
  const canRestore = ref(false)

  const { restore } = useVersionActions(resourceId, () => {
    void session()?.reload()
  })

  async function view(version: number, at: Date, mayRestore: boolean): Promise<void> {
    const stored = await readPackageVersion(resourceId(), version)
    savedAt.value = at
    canRestore.value = mayRestore
    session()?.showVersion({ name: stored.name, bytes: stored.bytes }, version)
  }

  function restorePreviewed(): void {
    if (previewing.value !== null) {
      restore.mutate(previewing.value)
    }
  }

  function back(): void {
    void session()?.reload()
  }

  return { previewing, savedAt, canRestore, view, back, restore, restorePreviewed }
}
