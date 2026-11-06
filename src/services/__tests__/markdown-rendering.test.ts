import { describe, expect, it } from 'vitest'
import { renderMarkdown } from '../markdown/markdown-service'

describe('on-demand Markdown rendering', () => {
  it('highlights common languages and safely falls back for an unknown language', () => {
    const highlighted = renderMarkdown('```typescript\nconst answer: number = 42\n```').html
    expect(highlighted).toContain('hljs-keyword')
    const fallback = renderMarkdown('```not-a-registered-language\n<script>alert(1)</script>\n```').html
    expect(fallback).toContain('&lt;script&gt;')
    expect(fallback).not.toContain('<script>')
  })
})
