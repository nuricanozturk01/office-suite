import { z } from 'zod'

/** Error codes the backend emits in `problem+json` bodies and that have a translated message. */
const knownCodes = [
  'validation_failed',
  'email_taken',
  'invalid_credentials',
  'unauthorized',
  'not_found',
  'internal',
  'version_conflict',
  'user_not_found',
  'cannot_share_with_owner',
  'invalid_role',
  'wrong_password',
  'deletion_already_scheduled',
  'no_pending_deletion',
  'at_least_one_field_required',
  'display_name_mismatch',
  'csrf_rejected',
] as const

/** Client-side codes for failures that never produced a problem body. */
type ClientCode = 'network' | 'unexpected_response'

const problemSchema = z.object({
  status: z.number().int(),
  code: z.string(),
  title: z.string(),
  detail: z.string().nullish(),
})

export class ApiError extends Error {
  override readonly name = 'ApiError'

  constructor(
    readonly status: number,
    readonly code: string,
    readonly detail?: string,
  ) {
    super(code)
  }
}

export function toApiError(body: unknown, response: Response): ApiError {
  const problem = problemSchema.safeParse(body)
  if (problem.success) {
    return new ApiError(problem.data.status, problem.data.code, problem.data.detail ?? undefined)
  }
  const code: ClientCode = 'unexpected_response'
  return new ApiError(response.status, code)
}

export function networkError(): ApiError {
  const code: ClientCode = 'network'
  return new ApiError(0, code)
}

/** i18n key describing an error to the user. */
export function errorMessageKey(error: unknown): string {
  if (!(error instanceof ApiError)) {
    return 'errors.unknown'
  }
  if (error.code === 'network') {
    return 'errors.network'
  }
  return knownCodes.some((code) => code === error.code) ? `errors.${error.code}` : 'errors.unknown'
}
