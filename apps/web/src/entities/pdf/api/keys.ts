export const pdfKeys = {
  all: ['pdfs'] as const,
  lists: () => [...pdfKeys.all, 'list'] as const,
  detail: (id: string) => [...pdfKeys.all, 'detail', id] as const,
}
