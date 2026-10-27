export const packageKeys = {
  all: ['packages'] as const,
  versions: (resourceId: string) => [...packageKeys.all, 'versions', resourceId] as const,
}
