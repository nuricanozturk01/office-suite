import { api, ApiError, unwrap, unwrapWithResponse } from '@/shared/api'

export interface StoredPackage {
  name: string
  version: number
  bytes: ArrayBuffer
}

/** The file stored for a resource, or `null` when it has none yet (a new, blank document). */
export async function readPackage(resourceId: string): Promise<StoredPackage | null> {
  try {
    const { data, response } = await unwrapWithResponse(
      api.GET('/api/v1/packages/{resource_id}', {
        params: { path: { resource_id: resourceId } },
        parseAs: 'arrayBuffer',
      }),
    )
    return {
      name: decodeURIComponent(response.headers.get('X-Package-Name') ?? 'document'),
      version: Number(response.headers.get('X-Package-Version') ?? '1'),
      bytes: data,
    }
  } catch (error) {
    if (error instanceof ApiError && error.status === 404) {
      return null
    }
    throw error
  }
}

export interface SavedPackage {
  version: number
  /** Changes somebody else saved meanwhile were merged into the stored file. */
  merged: boolean
}

/**
 * Stores the file. Pass the version it is based on to detect conflicts; with `merge`, a stale
 * save is combined with the newer one when the server can (otherwise it is a conflict).
 */
export async function writePackage(
  resourceId: string,
  name: string,
  bytes: ArrayBuffer,
  baseVersion: number | null,
  merge = false,
): Promise<SavedPackage> {
  const { version, merged } = await unwrap(
    api.PUT('/api/v1/packages/{resource_id}', {
      params: {
        path: { resource_id: resourceId },
        query:
          baseVersion === null
            ? { name }
            : { name, base_version: baseVersion, ...(merge ? { merge } : {}) },
      },
      // The endpoint takes the raw file; the generated body type only documents it.
      body: '',
      bodySerializer: () => bytes,
      headers: { 'Content-Type': 'application/octet-stream' },
    }),
  )
  return { version, merged }
}

/** One kept version of a file. */
export interface PackageVersion {
  version: number
  name: string
  size: number
  savedBy: string
  /** Name of whoever saved it; `null` when the user no longer exists. */
  savedByName: string | null
  savedAt: string
}

export interface PackageVersions {
  /** The signed-in user may restore an older version. */
  canRestore: boolean
  versions: PackageVersion[]
}

/** Kept versions of a resource's file, newest first. */
export async function listPackageVersions(resourceId: string): Promise<PackageVersions> {
  const entries = await unwrap(
    api.GET('/api/v1/packages/{resource_id}/versions', {
      params: { path: { resource_id: resourceId } },
    }),
  )
  return {
    canRestore: entries.can_restore,
    versions: entries.versions.map((entry) => ({
      version: entry.version,
      name: entry.name,
      size: entry.size,
      savedBy: entry.saved_by,
      savedByName: entry.saved_by_name ?? null,
      savedAt: entry.saved_at,
    })),
  }
}

/** The file as of one kept version. */
export async function readPackageVersion(
  resourceId: string,
  version: number,
): Promise<StoredPackage> {
  const { data, response } = await unwrapWithResponse(
    api.GET('/api/v1/packages/{resource_id}/versions/{version}', {
      params: { path: { resource_id: resourceId, version } },
      parseAs: 'arrayBuffer',
    }),
  )
  return {
    name: decodeURIComponent(response.headers.get('X-Package-Name') ?? 'document'),
    version,
    bytes: data,
  }
}

/** Makes a kept version the current one (stored as a new version). */
export async function restorePackageVersion(resourceId: string, version: number): Promise<number> {
  const saved = await unwrap(
    api.POST('/api/v1/packages/{resource_id}/versions/{version}/restore', {
      params: { path: { resource_id: resourceId, version } },
    }),
  )
  return saved.version
}

const RETRY_DELAY_MS = 400
const MAX_ATTEMPTS = 10

/**
 * Stores the user's original file as the resource's package, so it opens exactly as uploaded.
 * A new resource becomes known to the workspace a moment after it is created (an event), so a
 * `404` is retried for a few seconds.
 */
export async function storeOriginalFile(resourceId: string, file: File): Promise<void> {
  const bytes = await file.arrayBuffer()
  for (let attempt = 1; ; attempt += 1) {
    try {
      await writePackage(resourceId, file.name, bytes, null)
      return
    } catch (error) {
      if (!(error instanceof ApiError) || error.status !== 404 || attempt >= MAX_ATTEMPTS) {
        throw error
      }
      await new Promise((resolve) => setTimeout(resolve, RETRY_DELAY_MS))
    }
  }
}
