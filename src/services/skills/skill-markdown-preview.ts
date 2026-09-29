import DOMPurify from 'dompurify'
import { Marked } from 'marked'
import type { SkillSourceGuide } from './skill-source-guide'

const markdown = new Marked({ gfm: true, breaks: true })
const MAX_PREVIEW_CHARS = 100_000
const IMAGE_HOSTS = new Set([
  'github.com',
  'raw.githubusercontent.com',
  'user-images.githubusercontent.com',
  'private-user-images.githubusercontent.com',
])

export interface SkillMarkdownPreview {
  html: string
  truncated: boolean
}

export function renderSkillMarkdownPreview(guide: SkillSourceGuide): SkillMarkdownPreview {
  const truncated = guide.text.length > MAX_PREVIEW_CHARS
  const content = truncated ? guide.text.slice(0, MAX_PREVIEW_CHARS) : guide.text
  const parsed = markdown.parse(content, { async: false }) as string
  const safe = DOMPurify.sanitize(parsed, {
    USE_PROFILES: { html: true },
    FORBID_TAGS: ['script', 'style', 'iframe', 'object', 'embed', 'form', 'input', 'textarea', 'select', 'video', 'audio', 'source', 'picture'],
    FORBID_ATTR: ['style', 'srcdoc', 'srcset', 'ping'],
  })

  const fragment = document.createElement('template')
  fragment.innerHTML = safe
  const rawBase = new URL(guide.url.replace('https://github.com/', 'https://raw.githubusercontent.com/').replace('/blob/', '/'))

  for (const link of fragment.content.querySelectorAll('a')) {
    const href = link.getAttribute('href')
    if (!href || href.startsWith('#')) {
      link.removeAttribute('href')
      continue
    }
    try {
      const resolved = new URL(href, guide.url)
      if (resolved.protocol !== 'https:') throw new Error('Unsupported URL')
      link.href = resolved.href
      link.rel = 'noopener noreferrer'
    } catch {
      link.removeAttribute('href')
    }
  }

  for (const image of fragment.content.querySelectorAll('img')) {
    const src = image.getAttribute('src')
    if (!src) {
      image.remove()
      continue
    }
    try {
      const resolved = new URL(src, rawBase)
      if (resolved.protocol !== 'https:' || !IMAGE_HOSTS.has(resolved.hostname)) throw new Error('Unsupported image host')
      image.removeAttribute('srcset')
      image.src = resolved.href
      image.loading = 'lazy'
    } catch {
      image.replaceWith(document.createTextNode(image.alt || ''))
    }
  }

  return { html: fragment.innerHTML, truncated }
}
