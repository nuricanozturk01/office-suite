export const shareKeys = {
  all: ['shares'] as const,
  file: (fileId: string) => [...shareKeys.all, 'file', fileId] as const,
  sharedWithMe: () => [...shareKeys.all, 'shared-with-me'] as const,
}
