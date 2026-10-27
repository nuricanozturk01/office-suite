export const spreadsheetKeys = {
  all: ['spreadsheets'] as const,
  lists: () => [...spreadsheetKeys.all, 'list'] as const,
  detail: (id: string) => [...spreadsheetKeys.all, 'detail', id] as const,
}
