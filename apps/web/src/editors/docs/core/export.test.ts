import { describe, expect, it } from 'vitest'

import { fileName, toHtmlDocument } from './export'

describe('toHtmlDocument', () => {
  it('escapes the title and language', () => {
    const html = toHtmlDocument('<script>"x"</script>', '<p>Hi</p>', 'tr"')
    expect(html).toContain('<title>&lt;script&gt;&quot;x&quot;&lt;/script&gt;</title>')
    expect(html).toContain('<html lang="tr&quot;">')
    expect(html).toContain('<body><p>Hi</p></body>')
  })
})

describe('fileName', () => {
  it('strips characters that are invalid in file names', () => {
    expect(fileName('Q3: plan/draft?', 'html', 'doc')).toBe('Q3  plan draft.html')
  })

  it('falls back when nothing usable remains', () => {
    expect(fileName(' / ', 'html', 'Untitled')).toBe('Untitled.html')
  })
})
