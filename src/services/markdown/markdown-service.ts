import { invoke } from '@tauri-apps/api/core'
import { convertFileSrc } from '@tauri-apps/api/core'
import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
import { emit } from '@tauri-apps/api/event'
import { Marked, type RendererObject, type Tokens, type Renderer } from 'marked'
import hljs from 'highlight.js'
import DOMPurify from 'dompurify'

export interface MarkdownFileInfo {
  path: string
  name: string
  content: string
  size: number
  modifiedAt: number | null
}

export interface MarkdownFileMetadata {
  path: string
  name: string
  size: number
  modifiedAt: number | null
}

export interface TocItem {
  id: string
  text: string
  level: number
}

export interface RenderedMarkdownResult {
  html: string
  toc: TocItem[]
  wordCount: number
  readingTimeMinutes: number
}

export async function readMarkdownFile(filePath: string): Promise<MarkdownFileInfo> {
  return await invoke<MarkdownFileInfo>('read_markdown_file', { filePath })
}

export async function getMarkdownMetadata(filePath: string): Promise<MarkdownFileMetadata> {
  return await invoke<MarkdownFileMetadata>('get_markdown_file_metadata', { filePath })
}

export async function getInitialMarkdownFile(): Promise<string | null> {
  return await invoke<string | null>('get_initial_markdown_file')
}

export async function openMarkdownInEditor(filePath: string): Promise<void> {
  await invoke('open_markdown_in_editor', { filePath })
}

let standaloneWindow: WebviewWindow | null = null

export async function openMarkdownStandaloneWindow(filePath: string): Promise<void> {
  const fileName = filePath.split(/[/\\]/).pop() || 'Markdown Preview'

  if (standaloneWindow) {
    try {
      await standaloneWindow.setFocus()
      await emit('markdown-preview-open', { filePath })
      return
    } catch {
      standaloneWindow = null
    }
  }

  const encodedPath = encodeURIComponent(filePath)
  const windowLabel = 'markdown-preview'

  try {
    const existing = await WebviewWindow.getByLabel(windowLabel)
    if (existing) {
      await existing.setFocus()
      await emit('markdown-preview-open', { filePath })
      standaloneWindow = existing
      return
    }
  } catch {
    // Window doesn't exist, proceed to create
  }

  try {
    standaloneWindow = new WebviewWindow(windowLabel, {
      title: `Halowake - ${fileName}`,
      url: `/#/preview?file=${encodedPath}`,
      width: 1160,
      height: 820,
      minWidth: 640,
      minHeight: 480,
      decorations: true,
      resizable: true,
      center: true,
    })

    standaloneWindow.once('tauri://destroyed', () => {
      standaloneWindow = null
    })
    standaloneWindow.once('tauri://error', () => {
      standaloneWindow = null
    })
  } catch (err) {
    console.error('Failed to open standalone markdown window:', err)
  }
}

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;')
}

function resolveRelativePath(baseFile: string, relativePath: string): string {
  if (/^([a-zA-Z]:[\\/]|\\\\|\/)/.test(relativePath)) {
    return relativePath
  }
  const cleanBase = baseFile.replace(/\\/g, '/')
  const lastSlash = cleanBase.lastIndexOf('/')
  const baseDir = lastSlash >= 0 ? cleanBase.slice(0, lastSlash) : ''

  const segments = (baseDir ? baseDir.split('/') : []).concat(relativePath.replace(/\\/g, '/').split('/'))
  const resolved: string[] = []

  for (const seg of segments) {
    if (!seg || seg === '.') continue
    if (seg === '..') {
      if (resolved.length > 0 && resolved[resolved.length - 1] !== '..') {
        resolved.pop()
      }
    } else {
      resolved.push(seg)
    }
  }

  let result = resolved.join('/')
  if (/^[a-zA-Z]:\//.test(cleanBase) && !/^[a-zA-Z]:\//.test(result)) {
    const drive = cleanBase.slice(0, 2)
    result = `${drive}/${result}`
  }
  return result
}

function calculateStats(markdown: string): { wordCount: number; readingTimeMinutes: number } {
  // Strip code blocks and images for accurate reading count
  const cleanText = markdown
    .replace(/```[\s\S]*?```/g, '')
    .replace(/`.*?`/g, '')
    .replace(/!\[.*?\]\(.*?\)/g, '')
    .replace(/\[.*?\]\(.*?\)/g, '$1')

  const cjkMatches = cleanText.match(/[\u4e00-\u9fa5\u3040-\u30ff\uac00-\ud7af]/g) || []
  const nonCjk = cleanText.replace(/[\u4e00-\u9fa5\u3040-\u30ff\uac00-\ud7af]/g, ' ')
  const englishWords = nonCjk.trim().split(/\s+/).filter(Boolean)

  const wordCount = cjkMatches.length + englishWords.length
  const readingTimeMinutes = Math.max(1, Math.ceil(wordCount / 300))

  return { wordCount, readingTimeMinutes }
}

export function renderMarkdown(content: string, options?: { currentFilePath?: string }): RenderedMarkdownResult {
  const currentFilePath = options?.currentFilePath || ''
  const toc: TocItem[] = []
  let headingCounter = 0
  let mermaidCounter = 0

  const renderer: RendererObject = {
    heading(this: Renderer, token: Tokens.Heading) {
      const { text, depth, tokens } = token
      headingCounter++
      const plainText = (text || '').replace(/<[^>]+>/g, '').trim()
      const slug =
        plainText
          .toLowerCase()
          .replace(/[^\w\u4e00-\u9fa5-]/g, '-')
          .replace(/-+/g, '-')
          .replace(/^-|-$/g, '') || `h-${headingCounter}`
      const id = `heading-${headingCounter}-${slug}`

      toc.push({
        id,
        text: plainText,
        level: depth,
      })

      const innerContent = tokens && tokens.length ? this.parser.parseInline(tokens) : text

      return `<h${depth} id="${id}" class="md-heading md-heading-${depth}">
        <span class="md-heading-text">${innerContent}</span>
        <a href="#${id}" class="md-heading-anchor" aria-label="Anchor to ${escapeHtml(plainText)}">#</a>
      </h${depth}>\n`
    },

    code(this: Renderer, token: Tokens.Code) {
      const { text, lang } = token
      const cleanLang = (lang || '').trim().split(/\s+/)[0] || 'plaintext'
      const encodedCode = encodeURIComponent(text)

      if (cleanLang.toLowerCase() === 'mermaid') {
        mermaidCounter++
        const chartId = `mermaid-chart-${Date.now()}-${mermaidCounter}`

        return `<div class="md-mermaid-container" data-chart-id="${chartId}">
        <div class="md-mermaid-header">
          <span class="md-mermaid-badge">
            <svg class="mermaid-icon" viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <rect x="3" y="3" width="7" height="7"></rect>
              <rect x="14" y="3" width="7" height="7"></rect>
              <rect x="14" y="14" width="7" height="7"></rect>
              <rect x="3" y="14" width="7" height="7"></rect>
            </svg>
            <span>MERMAID</span>
          </span>
          <div class="md-mermaid-actions">
            <button type="button" class="md-mermaid-view-toggle" data-chart-id="${chartId}" title="切换视图">
              <span class="view-toggle-text">查看源码</span>
            </button>
            <button type="button" class="md-code-copy-btn" data-code="${encodedCode}" title="复制代码">
              <svg class="copy-icon" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
                <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
              </svg>
              <span class="copy-text">复制</span>
            </button>
          </div>
        </div>
        <div class="md-mermaid-viewport" id="${chartId}-viewport">
          <div class="md-mermaid-loading">
            <div class="md-mermaid-spinner"></div>
            <span>图表渲染中...</span>
          </div>
        </div>
        <pre class="md-mermaid-code hljs" hidden><code class="language-mermaid">${escapeHtml(text)}</code></pre>
        <div class="md-mermaid-raw" hidden data-source="${encodedCode}"></div>
      </div>\n`
      }

      const validLang = hljs.getLanguage(cleanLang) ? cleanLang : 'plaintext'
      let highlighted = ''

      try {
        if (validLang !== 'plaintext') {
          highlighted = hljs.highlight(text, { language: validLang, ignoreIllegals: true }).value
        } else {
          highlighted = escapeHtml(text)
        }
      } catch {
        highlighted = escapeHtml(text)
      }

      return `<div class="md-code-block" data-lang="${escapeHtml(cleanLang)}">
        <div class="md-code-header">
          <span class="md-code-badge">${escapeHtml(cleanLang)}</span>
          <button type="button" class="md-code-copy-btn" data-code="${encodedCode}" title="复制代码">
            <svg class="copy-icon" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
              <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
            </svg>
            <span class="copy-text">复制</span>
          </button>
        </div>
        <pre class="hljs"><code class="language-${validLang}">${highlighted}</code></pre>
      </div>\n`
    },

    image({ href, title, text }: { href: string; title?: string | null; text: string }) {
      let finalSrc = href || ''
      if (finalSrc && !/^(https?:|data:|blob:|asset:|\/\/)/i.test(finalSrc) && currentFilePath) {
        try {
          const resolved = resolveRelativePath(currentFilePath, finalSrc)
          finalSrc = convertFileSrc(resolved)
        } catch (e) {
          console.warn('Failed to resolve relative image path:', finalSrc, e)
        }
      }

      const titleAttr = title ? ` title="${escapeHtml(title)}"` : ''
      const altAttr = text ? ` alt="${escapeHtml(text)}"` : ''
      return `<span class="md-image-wrapper"><img src="${escapeHtml(finalSrc)}"${altAttr}${titleAttr} class="md-image" loading="lazy" /></span>`
    },

    link({ href, title, text }: { href: string; title?: string | null; text: string }) {
      const titleAttr = title ? ` title="${escapeHtml(title)}"` : ''
      const isExternal = /^(https?:|\/\/)/i.test(href)
      const targetAttr = isExternal ? ' target="_blank" rel="noopener noreferrer"' : ''
      return `<a href="${escapeHtml(href)}"${titleAttr}${targetAttr} class="md-link">${text}</a>`
    },

    table(this: Renderer, token: Tokens.Table) {
      let headerCells = ''
      for (let i = 0; i < token.header.length; i++) {
        headerCells += this.tablecell(token.header[i])
      }
      const headerRow = this.tablerow({ text: headerCells })

      let bodyRows = ''
      for (let i = 0; i < token.rows.length; i++) {
        let rowCells = ''
        for (let j = 0; j < token.rows[i].length; j++) {
          rowCells += this.tablecell(token.rows[i][j])
        }
        bodyRows += this.tablerow({ text: rowCells })
      }

      const tbody = bodyRows ? `<tbody>\n${bodyRows}</tbody>\n` : ''

      return `<div class="md-table-container">
        <table class="md-table">
          <thead>\n${headerRow}</thead>\n
          ${tbody}
        </table>
      </div>\n`
    },
  }

  const customMarked = new Marked({
    gfm: true,
    breaks: true,
  })
  customMarked.use({ renderer })

  // Parse HTML
  let parsedHtml = customMarked.parse(content, { async: false }) as string

  // GitHub callout alerts post-processing
  // e.g. <blockquote><p>[!NOTE]<br>content</p></blockquote>
  parsedHtml = parsedHtml.replace(
    /<blockquote>\s*<p>\[!(NOTE|TIP|IMPORTANT|WARNING|CAUTION)\](?:\s*<br>|\s*\n)?([\s\S]*?)<\/p>\s*<\/blockquote>/gi,
    (_match, alertType: string, bodyText: string) => {
      const type = alertType.toLowerCase()
      const titleMap: Record<string, string> = {
        note: 'Note',
        tip: 'Tip',
        important: 'Important',
        warning: 'Warning',
        caution: 'Caution',
      }
      const title = titleMap[type] || alertType

      return `<div class="md-alert md-alert--${type}">
        <div class="md-alert-title">${title}</div>
        <div class="md-alert-content"><p>${bodyText.trim()}</p></div>
      </div>`
    }
  )

  parsedHtml = DOMPurify.sanitize(parsedHtml, {
    USE_PROFILES: { html: true, svg: true, svgFilters: true },
    ADD_ATTR: ['target'],
    FORBID_TAGS: ['script', 'style', 'iframe', 'object', 'embed', 'form', 'input', 'textarea', 'select'],
    FORBID_ATTR: ['style', 'srcdoc'],
  })

  const stats = calculateStats(content)

  return {
    html: parsedHtml,
    toc,
    wordCount: stats.wordCount,
    readingTimeMinutes: stats.readingTimeMinutes,
  }
}
