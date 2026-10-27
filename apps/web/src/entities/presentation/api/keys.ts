export const presentationKeys = {
  all: ['presentations'] as const,
  lists: () => [...presentationKeys.all, 'list'] as const,
  detail: (id: string) => [...presentationKeys.all, 'detail', id] as const,
}
