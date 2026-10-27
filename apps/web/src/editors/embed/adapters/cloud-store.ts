import { readPackage, writePackage } from '@/entities/package'
import { ApiError } from '@/shared/api'

import { type EditorFile, type FileStore, VersionConflictError } from '../core/ports'
import type { EmbeddedAppId } from '../core/apps'
import { blankFile } from './blank-file'

/** Adapter: the product's cloud (`/api/v1/packages`) as the document store of one resource. */
export function createCloudStore(app: EmbeddedAppId, resourceId: string, title: string): FileStore {
  return {
    load: async () => {
      const stored = await readPackage(resourceId)
      return stored === null
        ? null
        : { file: { name: stored.name, bytes: stored.bytes }, version: stored.version }
    },
    blank: (): Promise<EditorFile | null> => blankFile(app, resourceId, title),
    save: async (file, baseVersion) => {
      try {
        return await writePackage(resourceId, file.name, file.bytes, baseVersion, true)
      } catch (error) {
        if (error instanceof ApiError && error.code === 'version_conflict') {
          throw new VersionConflictError()
        }
        throw error
      }
    },
  }
}
