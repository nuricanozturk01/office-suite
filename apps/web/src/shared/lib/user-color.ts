/**
 * Stable per-user hue for presence carets. Colors are data here (one per collaborator),
 * so they cannot come from the static design tokens.
 */
export function userColor(seed: string): string {
  let hash = 0
  for (const char of seed) {
    hash = (hash * 31 + char.charCodeAt(0)) | 0
  }
  return `hsl(${String(Math.abs(hash) % 360)} 70% 45%)`
}
