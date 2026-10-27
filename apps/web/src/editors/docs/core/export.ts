const ESCAPES: Record<string, string> = {
  '&': '&amp;',
  '<': '&lt;',
  '>': '&gt;',
  '"': '&quot;',
  "'": '&#39;',
}

function escapeHtml(text: string): string {
  return text.replaceAll(/[&<>"']/g, (char) => ESCAPES[char] ?? char)
}

/**
 * Wraps the editor's serialized body (produced from the sanitized ProseMirror schema) in a
 * standalone HTML document.
 */
export function toHtmlDocument(title: string, body: string, lang: string): string {
  return [
    '<!doctype html>',
    `<html lang="${escapeHtml(lang)}">`,
    '<head>',
    '<meta charset="utf-8">',
    `<title>${escapeHtml(title)}</title>`,
    '<style>body{max-width:210mm;margin:2rem auto;padding:0 1rem;font-family:Arial,sans-serif;font-size:11pt;line-height:1.5}table{border-collapse:collapse}td,th{border:1px solid #999;padding:4px 8px}img{max-width:100%}</style>',
    '</head>',
    `<body>${body}</body>`,
    '</html>',
  ].join('\n')
}

/** File name safe on every major OS, falling back when the title has no usable characters. */
export function fileName(title: string, extension: string, fallback: string): string {
  const base = title
    .replaceAll(/[\\/:*?"<>|\p{Cc}]+/gu, ' ')
    .trim()
    .slice(0, 120)
  return `${base === '' ? fallback : base}.${extension}`
}
