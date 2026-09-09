<template>
  <div class="md-viewer-container" :class="[`font-size--${historyStore.fontSize}`, `width-mode--${historyStore.readingWidth}`]" @dragover.prevent="isDragging = true" @dragleave.prevent="isDragging = false" @drop.prevent="handleDrop">
    <!-- Drag overlay -->
    <div v-if="isDragging" class="md-drag-overlay">
      <Icon icon="solar:document-text-linear" class="md-drag-icon" />
      <span class="md-drag-text">{{ t('markdownPreview.dropToOpen') }}</span>
    </div>

    <!-- Main Content Area -->
    <div class="md-viewer-body" :class="[`toc-pos--${historyStore.tocPosition}`]">
      <!-- TOC / Outline Sidebar (Docked Left) -->
      <aside
        v-if="historyStore.tocOpen && toc.length && historyStore.tocPosition === 'left'"
        class="md-toc-sidebar md-toc-sidebar--left"
      >
        <header class="md-toc-header">
          <div class="md-toc-title">
            <Icon icon="solar:list-linear" />
            <span>{{ t('markdownPreview.outline') }}</span>
            <span class="md-toc-count">{{ toc.length }}</span>
          </div>
          <div class="md-toc-header-actions">
            <button
              type="button"
              class="md-toc-action-btn"
              :title="t('markdownPreview.dockRight')"
              @click="historyStore.setTocPosition('right')"
            >
              <Icon icon="solar:sidebar-minimalistic-linear" class="flip-h" />
            </button>
            <button
              type="button"
              class="md-toc-action-btn"
              :title="t('common.close')"
              @click="closeTocExplicitly"
            >
              <Icon icon="solar:close-circle-linear" />
            </button>
          </div>
        </header>

        <!-- Search filter for TOC -->
        <div v-if="toc.length > 5" class="md-toc-filter">
          <Icon icon="solar:magnifer-linear" class="filter-icon" />
          <input
            v-model="tocSearchQuery"
            type="text"
            class="filter-input"
            :placeholder="t('markdownPreview.searchOutline')"
          />
          <button
            v-if="tocSearchQuery"
            type="button"
            class="filter-clear"
            @click="tocSearchQuery = ''"
          >
            ×
          </button>
        </div>

        <nav class="md-toc-nav" aria-label="Table of Contents">
          <button
            v-for="item in filteredToc"
            :key="item.id"
            type="button"
            class="md-toc-item"
            :class="[`level-${item.level}`, { active: activeHeadingId === item.id }]"
            :title="item.text"
            @click="scrollToHeading(item.id)"
          >
            <span class="md-toc-indicator"></span>
            <span class="md-toc-label">{{ item.text }}</span>
          </button>
          <div v-if="!filteredToc.length" class="md-toc-empty">
            <span>{{ t('markdownPreview.noMatches') }}</span>
          </div>
        </nav>
      </aside>

      <!-- Document Article Scrollable Host -->
      <main ref="scrollContainer" class="md-content-scroll" @scroll="handleScroll">
        <div class="md-content-wrapper">
          <article ref="articleContent" class="md-rendered-content" @click="handleArticleClick" v-html="renderedHtml"></article>
        </div>

        <!-- Floating TOC Quick Trigger (visible when sidebar is closed, strictly on same side) -->
        <transition name="fade">
          <button
            v-if="!historyStore.tocOpen && toc.length"
            type="button"
            class="md-toc-floating-btn"
            :class="[`pos--${historyStore.tocPosition}`]"
            :title="t('markdownPreview.toggleOutline')"
            @click="openTocFromFloating"
          >
            <Icon icon="solar:list-linear" class="floating-icon" />
            <span class="floating-text">{{ t('markdownPreview.outline') }}</span>
            <span class="floating-badge">{{ toc.length }}</span>
          </button>
        </transition>
      </main>

      <!-- TOC / Outline Sidebar (Docked Right) -->
      <aside
        v-if="historyStore.tocOpen && toc.length && historyStore.tocPosition === 'right'"
        class="md-toc-sidebar md-toc-sidebar--right"
      >
        <header class="md-toc-header">
          <div class="md-toc-title">
            <Icon icon="solar:list-linear" />
            <span>{{ t('markdownPreview.outline') }}</span>
            <span class="md-toc-count">{{ toc.length }}</span>
          </div>
          <div class="md-toc-header-actions">
            <button
              type="button"
              class="md-toc-action-btn"
              :title="t('markdownPreview.dockLeft')"
              @click="historyStore.setTocPosition('left')"
            >
              <Icon icon="solar:sidebar-minimalistic-linear" />
            </button>
            <button
              type="button"
              class="md-toc-action-btn"
              :title="t('common.close')"
              @click="closeTocExplicitly"
            >
              <Icon icon="solar:close-circle-linear" />
            </button>
          </div>
        </header>

        <!-- Search filter for TOC -->
        <div v-if="toc.length > 5" class="md-toc-filter">
          <Icon icon="solar:magnifer-linear" class="filter-icon" />
          <input
            v-model="tocSearchQuery"
            type="text"
            class="filter-input"
            :placeholder="t('markdownPreview.searchOutline')"
          />
          <button
            v-if="tocSearchQuery"
            type="button"
            class="filter-clear"
            @click="tocSearchQuery = ''"
          >
            ×
          </button>
        </div>

        <nav class="md-toc-nav" aria-label="Table of Contents">
          <button
            v-for="item in filteredToc"
            :key="item.id"
            type="button"
            class="md-toc-item"
            :class="[`level-${item.level}`, { active: activeHeadingId === item.id }]"
            :title="item.text"
            @click="scrollToHeading(item.id)"
          >
            <span class="md-toc-indicator"></span>
            <span class="md-toc-label">{{ item.text }}</span>
          </button>
          <div v-if="!filteredToc.length" class="md-toc-empty">
            <span>{{ t('markdownPreview.noMatches') }}</span>
          </div>
        </nav>
      </aside>
    </div>

    <!-- Search Toolbar Float (Ctrl+F) -->
    <transition name="fade">
      <div v-if="searchOpen" class="md-search-float" @keydown.esc="closeSearch">
        <Icon icon="solar:magnifer-linear" class="search-icon" />
        <input
          ref="searchInput"
          v-model="searchQuery"
          type="text"
          class="search-input"
          :placeholder="t('markdownPreview.searchPlaceholder')"
          @keydown.enter="nextMatch"
        />
        <span class="match-count">{{ matchCountLabel }}</span>
        <button type="button" class="search-nav-btn" :title="t('markdownPreview.prevMatch')" @click="prevMatch">
          <Icon icon="solar:arrow-up-linear" />
        </button>
        <button type="button" class="search-nav-btn" :title="t('markdownPreview.nextMatch')" @click="nextMatch">
          <Icon icon="solar:arrow-down-linear" />
        </button>
        <button type="button" class="search-nav-btn close-btn" :title="t('common.close')" @click="closeSearch">
          ×
        </button>
      </div>
    </transition>

    <!-- Scroll to Top Quick Button -->
    <transition name="fade">
      <button v-if="showBackToTop" type="button" class="md-back-to-top" :title="t('markdownPreview.backToTop')" @click="scrollToTop">
        <Icon icon="solar:arrow-up-linear" />
      </button>
    </transition>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { Icon } from '@iconify/vue'
import { useMessage } from 'naive-ui'
import { useI18n } from 'vue-i18n'
import mermaid from 'mermaid'
import {
  renderMarkdown,
  type TocItem,
} from '@/services/markdown/markdown-service'
import { useMarkdownHistoryStore } from '@/stores/markdown-history'
import { usePreferencesStore } from '@/stores/preferences'
import { openExternalUrl } from '@/services/app-service'

const props = defineProps<{
  content: string
  filePath?: string
}>()

const emit = defineEmits<{
  (e: 'file-dropped', path: string): void
  (e: 'stats-updated', stats: { wordCount: number; readingTimeMinutes: number }): void
}>()

const { t } = useI18n({ useScope: 'global' })
const message = useMessage()
const historyStore = useMarkdownHistoryStore()
const preferencesStore = usePreferencesStore()

const scrollContainer = ref<HTMLElement | null>(null)
const articleContent = ref<HTMLElement | null>(null)
const searchInput = ref<HTMLInputElement | null>(null)

const isDragging = ref(false)
const renderedHtml = ref('')
const toc = ref<TocItem[]>([])
const activeHeadingId = ref('')
const showBackToTop = ref(false)
const userExplicitlyClosedToc = ref(false)
const tocSearchQuery = ref('')

const filteredToc = computed(() => {
  const query = tocSearchQuery.value.trim().toLowerCase()
  if (!query) return toc.value
  return toc.value.filter(item => item.text.toLowerCase().includes(query))
})

// In-document search
const searchOpen = ref(false)
const searchQuery = ref('')
const searchMatches = ref<HTMLElement[]>([])
const currentMatchIndex = ref(-1)

const matchCountLabel = computed(() => {
  if (!searchQuery.value.trim()) return ''
  if (!searchMatches.value.length) return t('markdownPreview.noMatches')
  return `${currentMatchIndex.value + 1} / ${searchMatches.value.length}`
})

function escapeHtml(str: string) {
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;')
}

function resolveLinkedDocument(baseFile: string, href: string): string | null {
  const rawPath = href.split('#', 1)[0]?.split('?', 1)[0]?.trim() || ''
  if (!rawPath || /^(?:[a-z][a-z\d+.-]*:|\/\/|#)/i.test(rawPath)) return null

  try {
    const decodedPath = decodeURIComponent(rawPath)
    if (!/\.(?:md|markdown|mdown|mkd|txt)$/i.test(decodedPath)) return null
    if (/^([a-zA-Z]:[\\/]|\\\\|\/)/.test(decodedPath)) return decodedPath

    const cleanBase = baseFile.replace(/\\/g, '/')
    const lastSlash = cleanBase.lastIndexOf('/')
    const baseDir = lastSlash >= 0 ? cleanBase.slice(0, lastSlash) : ''
    const segments = (baseDir ? baseDir.split('/') : []).concat(decodedPath.replace(/\\/g, '/').split('/'))
    const resolved: string[] = []

    for (const segment of segments) {
      if (!segment || segment === '.') continue
      if (segment === '..') {
        if (resolved.length > 0 && resolved[resolved.length - 1] !== '..') resolved.pop()
      } else {
        resolved.push(segment)
      }
    }

    let result = resolved.join('/')
    if (/^[a-zA-Z]:\//.test(cleanBase) && !/^[a-zA-Z]:\//.test(result)) {
      result = `${cleanBase.slice(0, 2)}/${result}`
    }
    return result
  } catch {
    return null
  }
}

watch(
  () => [props.content, props.filePath],
  () => {
    void parseDocument()
  },
  { immediate: true }
)

watch(
  () => preferencesStore.resolvedTheme,
  () => {
    void renderMermaidDiagrams()
  }
)

function openTocFromFloating() {
  userExplicitlyClosedToc.value = false
  historyStore.tocOpen = true
}

function closeTocExplicitly() {
  userExplicitlyClosedToc.value = true
  historyStore.tocOpen = false
}

async function parseDocument() {
  if (!props.content) {
    renderedHtml.value = ''
    toc.value = []
    return
  }

  const result = renderMarkdown(props.content, { currentFilePath: props.filePath })
  renderedHtml.value = result.html
  toc.value = result.toc
  emit('stats-updated', {
    wordCount: result.wordCount,
    readingTimeMinutes: result.readingTimeMinutes,
  })

  // Auto open outline if document has headings and user hasn't explicitly closed it
  if (toc.value.length > 0 && !historyStore.tocOpen && !userExplicitlyClosedToc.value) {
    historyStore.tocOpen = true
  }

  await nextTick()
  updateActiveHeading()
  await renderMermaidDiagrams()
}

async function renderMermaidDiagrams() {
  if (!articleContent.value) return
  const containers = articleContent.value.querySelectorAll<HTMLElement>('.md-mermaid-container')
  if (!containers.length) return

  const isDark =
    preferencesStore.resolvedTheme === 'dark' ||
    document.documentElement.getAttribute('data-theme') === 'dark'

  mermaid.initialize({
    startOnLoad: false,
    theme: isDark ? 'dark' : 'default',
    themeVariables: isDark
      ? {
          darkMode: true,
          background: 'transparent',
          mainBkg: '#2c2c2e',
          nodeBorder: '#4fa397',
          clusterBkg: '#1c1c1e',
          clusterBorder: '#38383a',
          lineColor: '#8e8e93',
          fontFamily: 'system-ui, -apple-system, sans-serif',
          fontSize: '13px',
        }
      : {
          darkMode: false,
          background: 'transparent',
          mainBkg: '#ffffff',
          nodeBorder: '#39786f',
          clusterBkg: '#f5f5f7',
          clusterBorder: '#d1d1d6',
          lineColor: '#636366',
          fontFamily: 'system-ui, -apple-system, sans-serif',
          fontSize: '13px',
        },
    securityLevel: 'strict',
  })

  let index = 0
  for (const container of Array.from(containers)) {
    const rawEl = container.querySelector('.md-mermaid-raw')
    const viewport = container.querySelector<HTMLElement>('.md-mermaid-viewport')
    if (!rawEl || !viewport) continue

    const encoded = rawEl.getAttribute('data-source') || ''
    const code = decodeURIComponent(encoded).trim()
    if (!code) continue

    index++
    const renderId = `mermaid-svg-${Date.now()}-${index}`

    try {
      const { svg } = await mermaid.render(renderId, code)
      viewport.innerHTML = svg
    } catch (err: unknown) {
      console.warn('Mermaid render error:', err)
      const errorMsg = err instanceof Error ? err.message : String(err)
      viewport.innerHTML = `<div class="md-mermaid-error">
        <span class="error-badge">流程图语法提示</span>
        <span class="error-msg">${escapeHtml(errorMsg || '图表语法有误，已切换为代码视图')}</span>
      </div>`
      const codeEl = container.querySelector<HTMLElement>('.md-mermaid-code')
      if (codeEl) codeEl.hidden = false
      const toggleBtn = container.querySelector<HTMLElement>(
        '.md-mermaid-view-toggle .view-toggle-text'
      )
      if (toggleBtn) toggleBtn.textContent = '隐藏代码'
    }
  }
}

function handleScroll() {
  if (!scrollContainer.value) return
  showBackToTop.value = scrollContainer.value.scrollTop > 350
  updateActiveHeading()
}

function updateActiveHeading() {
  if (!scrollContainer.value || !toc.value.length) return
  const containerRect = scrollContainer.value.getBoundingClientRect()
  const headings = toc.value
    .map(item => ({ id: item.id, el: document.getElementById(item.id) }))
    .filter((item): item is { id: string; el: HTMLElement } => Boolean(item.el))

  let currentId = toc.value[0]?.id || ''
  for (const h of headings) {
    const rect = h.el.getBoundingClientRect()
    if (rect.top - containerRect.top <= 100) {
      currentId = h.id
    } else {
      break
    }
  }
  activeHeadingId.value = currentId
}

function scrollToHeading(id: string) {
  const target = document.getElementById(id)
  if (!target || !scrollContainer.value) return

  activeHeadingId.value = id
  const containerRect = scrollContainer.value.getBoundingClientRect()
  const targetRect = target.getBoundingClientRect()
  const currentScrollTop = scrollContainer.value.scrollTop
  const targetTop = currentScrollTop + (targetRect.top - containerRect.top) - 16

  scrollContainer.value.scrollTo({
    top: Math.max(0, targetTop),
    behavior: 'smooth',
  })
}

function scrollToTop() {
  scrollContainer.value?.scrollTo({ top: 0, behavior: 'smooth' })
}

function handleArticleClick(event: MouseEvent) {
  const target = event.target as HTMLElement | null
  if (!target) return

  // 1. Mermaid View Toggle
  const viewToggleBtn = target.closest('.md-mermaid-view-toggle') as HTMLElement | null
  if (viewToggleBtn) {
    event.preventDefault()
    event.stopPropagation()
    const container = viewToggleBtn.closest('.md-mermaid-container') as HTMLElement | null
    if (container) {
      const viewport = container.querySelector<HTMLElement>('.md-mermaid-viewport')
      const codeBlock = container.querySelector<HTMLElement>('.md-mermaid-code')
      const textEl = viewToggleBtn.querySelector('.view-toggle-text')

      if (codeBlock && viewport) {
        const isCodeVisible = !codeBlock.hidden
        if (isCodeVisible) {
          codeBlock.hidden = true
          viewport.hidden = false
          if (textEl) textEl.textContent = '查看源码'
        } else {
          codeBlock.hidden = false
          viewport.hidden = true
          if (textEl) textEl.textContent = '查看图表'
        }
      }
    }
    return
  }

  // 2. Check Copy Button click
  const copyBtn = target.closest('.md-code-copy-btn') as HTMLElement | null
  if (copyBtn) {
    event.preventDefault()
    event.stopPropagation()
    const encodedCode = copyBtn.getAttribute('data-code') || ''
    const rawCode = decodeURIComponent(encodedCode)
    void navigator.clipboard.writeText(rawCode).then(() => {
      const copyTextEl = copyBtn.querySelector('.copy-text')
      if (copyTextEl) copyTextEl.textContent = t('markdownPreview.copied')
      copyBtn.classList.add('copied')
      setTimeout(() => {
        if (copyTextEl) copyTextEl.textContent = t('markdownPreview.copy')
        copyBtn.classList.remove('copied')
      }, 2000)
    })
    return
  }

  // 3. Check Anchor links inside document
  const anchor = target.closest('a') as HTMLAnchorElement | null
  if (anchor) {
    const href = anchor.getAttribute('href') || ''
    if (href.startsWith('#')) {
      event.preventDefault()
      const targetId = href.slice(1)
      scrollToHeading(targetId)
      return
    }

    if (/^https?:\/\//i.test(href)) {
      event.preventDefault()
      void openExternalUrl(href)
      return
    }

    const linkedDocument = resolveLinkedDocument(props.filePath || '', href)
    event.preventDefault()
    if (linkedDocument) {
      emit('file-dropped', linkedDocument)
    }
  }
}

function handleDrop(event: DragEvent) {
  isDragging.value = false
  const files = event.dataTransfer?.files
  if (!files || !files.length) return

  const file = files[0]
  // On desktop Tauri, file object often contains 'path'
  const filePath = (file as unknown as { path?: string }).path
  if (filePath && /\.(md|markdown|mdown|mkd|txt)$/i.test(filePath)) {
    emit('file-dropped', filePath)
  } else if (filePath) {
    message.warning(t('markdownPreview.unsupportedFileType'))
  }
}

// In-document search methods
function openSearch() {
  searchOpen.value = true
  void nextTick(() => {
    searchInput.value?.focus()
    searchInput.value?.select()
  })
}

function closeSearch() {
  searchOpen.value = false
  searchQuery.value = ''
  clearSearchHighlights()
}

function clearSearchHighlights() {
  if (!articleContent.value) return
  const marks = articleContent.value.querySelectorAll('mark.md-search-highlight')
  marks.forEach(mark => {
    const parent = mark.parentNode
    if (parent) {
      parent.replaceChild(document.createTextNode(mark.textContent || ''), mark)
      parent.normalize()
    }
  })
  searchMatches.value = []
  currentMatchIndex.value = -1
}

watch(searchQuery, query => {
  clearSearchHighlights()
  const q = query.trim()
  if (!q || !articleContent.value) return

  const regex = new RegExp(q.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'gi')
  const walker = document.createTreeWalker(articleContent.value, NodeFilter.SHOW_TEXT, {
    acceptNode(node) {
      if (node.parentElement?.closest('.md-code-block, script, style, mark')) {
        return NodeFilter.FILTER_REJECT
      }
      return regex.test(node.textContent || '') ? NodeFilter.FILTER_ACCEPT : NodeFilter.FILTER_SKIP
    },
  })

  const textNodes: Node[] = []
  while (walker.nextNode()) {
    textNodes.push(walker.currentNode)
  }

  const matches: HTMLElement[] = []
  for (const node of textNodes) {
    const parent = node.parentNode
    if (!parent) continue
    const text = node.textContent || ''
    const fragment = document.createDocumentFragment()
    let lastIdx = 0
    regex.lastIndex = 0

    let match: RegExpExecArray | null
    while ((match = regex.exec(text)) !== null) {
      const matchText = match[0]
      const start = match.index
      if (start > lastIdx) {
        fragment.appendChild(document.createTextNode(text.slice(lastIdx, start)))
      }
      const mark = document.createElement('mark')
      mark.className = 'md-search-highlight'
      mark.textContent = matchText
      fragment.appendChild(mark)
      matches.push(mark)
      lastIdx = start + matchText.length
    }
    if (lastIdx < text.length) {
      fragment.appendChild(document.createTextNode(text.slice(lastIdx)))
    }
    parent.replaceChild(fragment, node)
  }

  searchMatches.value = matches
  if (matches.length > 0) {
    currentMatchIndex.value = 0
    highlightCurrentMatch()
  }
})

function nextMatch() {
  if (!searchMatches.value.length) return
  currentMatchIndex.value = (currentMatchIndex.value + 1) % searchMatches.value.length
  highlightCurrentMatch()
}

function prevMatch() {
  if (!searchMatches.value.length) return
  currentMatchIndex.value =
    (currentMatchIndex.value - 1 + searchMatches.value.length) % searchMatches.value.length
  highlightCurrentMatch()
}

function highlightCurrentMatch() {
  searchMatches.value.forEach((el, idx) => {
    if (idx === currentMatchIndex.value) {
      el.classList.add('current')
      el.scrollIntoView({ behavior: 'smooth', block: 'center' })
    } else {
      el.classList.remove('current')
    }
  })
}

function handleKeydown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'f') {
    e.preventDefault()
    openSearch()
    return
  }

  if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key.toLowerCase() === 'o') {
    e.preventDefault()
    historyStore.toggleToc()
    return
  }
}

onMounted(() => {
  window.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeydown)
})

defineExpose({
  openSearch,
  scrollToTop,
})
</script>

<style scoped lang="scss">
.md-viewer-container {
  display: flex;
  flex: 1;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  min-width: 0;
  overflow: hidden;
  position: relative;
  width: 100%;
}

.md-viewer-body {
  display: flex;
  flex: 1;
  height: 100%;
  min-height: 0;
  min-width: 0;
  overflow: hidden;
  position: relative;
  width: 100%;

  .md-toc-sidebar--left {
    border-left: none;
    border-right: 0.5px solid var(--lumina-separator);
  }

  .md-toc-sidebar--right {
    border-left: 0.5px solid var(--lumina-separator);
    border-right: none;
  }
}

/* Scroll Area & Container */
.md-content-scroll {
  flex: 1;
  height: 100%;
  min-height: 0;
  min-width: 0;
  overflow-x: hidden;
  overflow-y: auto;
  scroll-behavior: smooth;
}

.md-content-wrapper {
  box-sizing: border-box;
  margin: 0 auto;
  min-height: 100%;
  padding: 32px 48px 80px;
  transition: max-width 0.2s ease;
  width: 100%;
}

/* Reading Width Modes */
.width-mode--centered .md-content-wrapper {
  max-width: 1060px;
}

.width-mode--full .md-content-wrapper {
  max-width: 100%;
  padding-inline: 48px;
}

/* Font Size Scales */
.font-size--normal {
  --md-font-size: 14.5px;
  --md-line-height: 1.75;
}

.font-size--large {
  --md-font-size: 16px;
  --md-line-height: 1.8;
}

.font-size--huge {
  --md-font-size: 18px;
  --md-line-height: 1.85;
}

/* Drag overlay */
.md-drag-overlay {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-surface-elevated) 88%, var(--lumina-primary) 12%);
  border: 2px dashed var(--lumina-primary);
  border-radius: var(--lumina-radius-lg);
  color: var(--lumina-primary);
  display: flex;
  flex-direction: column;
  gap: 12px;
  inset: 12px;
  justify-content: center;
  pointer-events: none;
  position: absolute;
  z-index: 50;
  backdrop-filter: blur(4px);
}

.md-drag-icon {
  font-size: 48px;
}

.md-drag-text {
  font-size: 16px;
  font-weight: 600;
}

/* Floating TOC Quick Trigger (when sidebar is collapsed) */
.md-toc-floating-btn {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-surface-elevated) 88%, transparent);
  border: 0.5px solid var(--lumina-separator);
  border-radius: 20px;
  box-shadow: var(--lumina-shadow-md);
  color: var(--lumina-text);
  cursor: pointer;
  display: inline-flex;
  font-size: 12px;
  font-weight: 550;
  gap: 6px;
  padding: 6px 12px;
  position: absolute;
  top: 18px;
  transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  z-index: 20;
  backdrop-filter: blur(12px);

  &.pos--left {
    left: 20px;
    right: auto;
  }

  &.pos--right {
    left: auto;
    right: 20px;
  }

  .floating-icon {
    color: var(--lumina-primary);
    font-size: 14px;
  }

  .floating-text {
    letter-spacing: 0.02em;
  }

  .floating-badge {
    background: var(--lumina-surface-3);
    border-radius: 10px;
    color: var(--lumina-text-secondary);
    font-size: 10px;
    font-weight: 600;
    padding: 1px 6px;
  }

  &:hover {
    background: var(--lumina-surface-elevated);
    border-color: var(--lumina-primary);
    box-shadow: var(--lumina-shadow-lg);
    color: var(--lumina-primary);
    transform: translateY(-1px);
  }

  &:active {
    transform: translateY(0);
  }
}

/* TOC / Outline Sidebar */
.md-toc-sidebar {
  background: var(--lumina-sidebar-bg);
  display: flex;
  flex: 0 0 260px;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  overflow: hidden;
  user-select: none;
  backdrop-filter: var(--lumina-vibrancy);
}

.md-toc-header {
  align-items: center;
  border-bottom: 0.5px solid var(--lumina-separator);
  display: flex;
  flex: 0 0 42px;
  justify-content: space-between;
  padding: 0 12px;
}

.md-toc-title {
  align-items: center;
  color: var(--lumina-text-secondary);
  display: flex;
  font-size: 12px;
  font-weight: 600;
  gap: 6px;
  letter-spacing: 0.03em;
  text-transform: uppercase;
}

.md-toc-count {
  background: var(--lumina-surface-3);
  border-radius: 10px;
  color: var(--lumina-text-tertiary);
  font-size: 10px;
  font-weight: 600;
  padding: 1px 6px;
}

.md-toc-header-actions {
  align-items: center;
  display: flex;
  gap: 2px;
}

.md-toc-action-btn {
  align-items: center;
  background: transparent;
  border: 0;
  border-radius: var(--lumina-radius-sm);
  color: var(--lumina-text-tertiary);
  cursor: pointer;
  display: inline-flex;
  font-size: 15px;
  height: 24px;
  justify-content: center;
  padding: 0;
  transition: all 0.15s ease;
  width: 24px;

  &:hover {
    background: var(--lumina-control-hover);
    color: var(--lumina-text);
  }

  .flip-h {
    transform: scaleX(-1);
  }
}

.md-toc-filter {
  align-items: center;
  border-bottom: 0.5px solid var(--lumina-separator);
  display: flex;
  padding: 6px 10px;
  position: relative;

  .filter-icon {
    color: var(--lumina-text-tertiary);
    font-size: 13px;
    left: 16px;
    pointer-events: none;
    position: absolute;
  }

  .filter-input {
    background: var(--lumina-control-bg);
    border: 0.5px solid var(--lumina-separator);
    border-radius: var(--lumina-radius-sm);
    box-sizing: border-box;
    color: var(--lumina-text);
    font-size: 11.5px;
    outline: none;
    padding: 4px 22px 4px 24px;
    transition: border-color 0.15s ease;
    width: 100%;

    &:focus {
      border-color: var(--lumina-primary);
    }
  }

  .filter-clear {
    align-items: center;
    background: transparent;
    border: 0;
    color: var(--lumina-text-tertiary);
    cursor: pointer;
    display: inline-flex;
    font-size: 13px;
    height: 18px;
    justify-content: center;
    position: absolute;
    right: 14px;
    width: 18px;

    &:hover {
      color: var(--lumina-text);
    }
  }
}

.md-toc-empty {
  color: var(--lumina-text-tertiary);
  font-size: 11.5px;
  padding: 18px 12px;
  text-align: center;
}

.md-toc-nav {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 2px;
  overflow-y: auto;
  padding: 10px 8px;
}

.md-toc-item {
  align-items: center;
  background: transparent;
  border: 0;
  border-radius: var(--lumina-radius-sm);
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: flex;
  font-size: 12px;
  gap: 6px;
  line-height: 1.4;
  padding: 5px 8px;
  text-align: left;
  transition: all 0.15s ease;
  width: 100%;

  &:hover {
    background: var(--lumina-control-hover);
    color: var(--lumina-text);
  }

  &.active {
    background: var(--lumina-primary-soft);
    color: var(--lumina-primary);
    font-weight: 550;

    .md-toc-indicator {
      background: var(--lumina-primary);
      opacity: 1;
    }
  }

  &.level-1 { font-weight: 600; }
  &.level-2 { padding-left: 16px; }
  &.level-3 { padding-left: 26px; font-size: 11.5px; opacity: 0.88; }
  &.level-4, &.level-5, &.level-6 { padding-left: 36px; font-size: 11px; opacity: 0.75; }
}

.md-toc-indicator {
  border-radius: 50%;
  flex: 0 0 4px;
  height: 4px;
  opacity: 0;
  transition: opacity 0.15s ease;
  width: 4px;
}

.md-toc-label {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* Float Search Bar */
.md-search-float {
  align-items: center;
  background: var(--lumina-surface-elevated);
  border: 0.5px solid var(--lumina-separator-strong);
  border-radius: var(--lumina-radius-md);
  box-shadow: var(--lumina-shadow-lg);
  display: flex;
  gap: 8px;
  padding: 6px 10px;
  position: absolute;
  right: 24px;
  top: 16px;
  z-index: 40;
  backdrop-filter: var(--lumina-vibrancy);
}

.search-icon {
  color: var(--lumina-text-tertiary);
  font-size: 15px;
}

.search-input {
  background: transparent;
  border: 0;
  color: var(--lumina-text);
  font-size: 13px;
  outline: none;
  width: 160px;

  &::placeholder {
    color: var(--lumina-text-tertiary);
  }
}

.match-count {
  color: var(--lumina-text-secondary);
  font-size: 11px;
  min-width: 44px;
  text-align: center;
}

.search-nav-btn {
  align-items: center;
  background: transparent;
  border: 0;
  border-radius: 4px;
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: flex;
  font-size: 13px;
  height: 22px;
  justify-content: center;
  padding: 0;
  width: 22px;

  &:hover {
    background: var(--lumina-control-hover);
    color: var(--lumina-text);
  }

  &.close-btn {
    font-size: 16px;
  }
}

/* Search Highlights */
:deep(mark.md-search-highlight) {
  background: rgba(255, 213, 79, 0.4);
  border-radius: 2px;
  color: inherit;
  padding: 0 1px;

  &.current {
    background: #ffb300;
    color: #000000;
    outline: 2px solid #ff8f00;
  }
}

/* Scroll To Top */
.md-back-to-top {
  align-items: center;
  background: var(--lumina-surface-elevated);
  border: 0.5px solid var(--lumina-separator-strong);
  border-radius: 50%;
  bottom: 24px;
  box-shadow: var(--lumina-shadow-md);
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: flex;
  font-size: 16px;
  height: 36px;
  justify-content: center;
  position: absolute;
  right: 280px;
  transition: all 0.2s ease;
  width: 36px;
  z-index: 30;
  backdrop-filter: var(--lumina-vibrancy);

  &:hover {
    background: var(--lumina-control-hover);
    color: var(--lumina-text);
    transform: translateY(-2px);
  }
}

/* Rendered Markdown Typography */
.md-rendered-content {
  color: var(--lumina-text);
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
  font-size: var(--md-font-size, 14.5px);
  line-height: var(--md-line-height, 1.75);
  word-break: break-word;

  :deep(h1),
  :deep(h2),
  :deep(h3),
  :deep(h4),
  :deep(h5),
  :deep(h6) {
    align-items: center;
    color: var(--lumina-text);
    display: flex;
    font-weight: 650;
    line-height: 1.35;
    margin-bottom: 12px;
    margin-top: 28px;
    position: relative;
    scroll-margin-top: 20px;

    .md-heading-anchor {
      color: var(--lumina-text-tertiary);
      font-size: 0.8em;
      margin-left: 8px;
      opacity: 0;
      text-decoration: none;
      transition: opacity 0.15s ease;
    }

    &:hover .md-heading-anchor {
      opacity: 0.6;
    }

    .md-heading-anchor:hover {
      opacity: 1;
    }
  }

  :deep(h1) {
    border-bottom: 1px solid var(--lumina-separator);
    font-size: 2em;
    margin-top: 16px;
    padding-bottom: 10px;
  }

  :deep(h2) {
    border-bottom: 0.5px solid var(--lumina-separator);
    font-size: 1.5em;
    padding-bottom: 8px;
  }

  :deep(h3) { font-size: 1.25em; }
  :deep(h4) { font-size: 1.1em; }
  :deep(h5) { font-size: 1em; }
  :deep(h6) { font-size: 0.9em; color: var(--lumina-text-secondary); }

  :deep(p) {
    margin: 12px 0;
  }

  :deep(a.md-link) {
    color: var(--lumina-primary);
    text-decoration: none;
    border-bottom: 1px dotted transparent;
    transition: border-color 0.15s ease;

    &:hover {
      border-bottom-color: var(--lumina-primary);
      text-decoration: underline;
    }
  }

  :deep(ul),
  :deep(ol) {
    margin: 10px 0;
    padding-left: 26px;

    li {
      margin: 4px 0;
    }
  }

  /* Inline Code */
  :deep(code:not(.hljs)) {
    background: var(--lumina-surface-3);
    border: 0.5px solid var(--lumina-separator);
    border-radius: 4px;
    color: var(--lumina-primary);
    font-family: var(--lumina-font-mono, 'JetBrains Mono', Consolas, Menlo, monospace);
    font-size: 0.88em;
    padding: 2px 6px;
  }

  /* Blockquote */
  :deep(blockquote) {
    background: var(--lumina-surface-2);
    border-left: 3.5px solid var(--lumina-primary);
    border-radius: 0 var(--lumina-radius-sm) var(--lumina-radius-sm) 0;
    color: var(--lumina-text-secondary);
    margin: 14px 0;
    padding: 10px 16px;

    p {
      margin: 4px 0;
    }
  }

  /* Code Blocks */
  :deep(.md-code-block) {
    background: var(--lumina-surface-elevated);
    border: 0.5px solid var(--lumina-separator);
    border-radius: var(--lumina-radius-md);
    box-shadow: var(--lumina-shadow-sm);
    margin: 18px 0;
    overflow: hidden;
  }

  :deep(.md-code-header) {
    align-items: center;
    background: color-mix(in srgb, var(--lumina-surface-3) 65%, transparent);
    border-bottom: 0.5px solid var(--lumina-separator);
    display: flex;
    justify-content: space-between;
    min-height: 32px;
    padding: 4px 12px;
    user-select: none;
  }

  :deep(.md-code-badge) {
    color: var(--lumina-text-secondary);
    font-family: var(--lumina-font-mono, monospace);
    font-size: 11px;
    font-weight: 550;
    text-transform: uppercase;
  }

  :deep(.md-code-copy-btn) {
    align-items: center;
    background: transparent;
    border: 0.5px solid transparent;
    border-radius: var(--lumina-radius-sm);
    color: var(--lumina-text-secondary);
    cursor: pointer;
    display: inline-flex;
    font-size: 11.5px;
    gap: 4px;
    padding: 2px 8px;
    transition: all 0.15s ease;

    &:hover {
      background: var(--lumina-control-hover);
      border-color: var(--lumina-separator);
      color: var(--lumina-text);
    }

    &.copied {
      background: color-mix(in srgb, var(--lumina-primary) 15%, transparent);
      color: var(--lumina-primary);
    }
  }

  :deep(pre.hljs) {
    background: transparent;
    border-radius: 0;
    box-sizing: border-box;
    font-family: var(--lumina-font-mono, 'JetBrains Mono', Consolas, monospace);
    font-size: 12.5px;
    line-height: 1.6;
    margin: 0;
    overflow-x: auto;
    padding: 14px 16px;
  }

  /* Mermaid Diagram Cards */
  :deep(.md-mermaid-container) {
    background: var(--lumina-surface-elevated);
    border: 0.5px solid var(--lumina-separator);
    border-radius: var(--lumina-radius-md);
    box-shadow: var(--lumina-shadow-sm);
    margin: 20px 0;
    overflow: hidden;
    transition: border-color var(--lumina-duration-fast) ease;

    &:hover {
      border-color: color-mix(in srgb, var(--lumina-primary) 40%, var(--lumina-separator));
    }
  }

  :deep(.md-mermaid-header) {
    align-items: center;
    background: color-mix(in srgb, var(--lumina-surface-3) 65%, transparent);
    border-bottom: 0.5px solid var(--lumina-separator);
    display: flex;
    justify-content: space-between;
    min-height: 34px;
    padding: 4px 12px;
    user-select: none;
  }

  :deep(.md-mermaid-badge) {
    align-items: center;
    color: var(--lumina-primary);
    display: inline-flex;
    font-family: var(--lumina-font-mono, monospace);
    font-size: 11px;
    font-weight: 650;
    gap: 6px;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  :deep(.md-mermaid-actions) {
    align-items: center;
    display: flex;
    gap: 6px;
  }

  :deep(.md-mermaid-view-toggle) {
    align-items: center;
    background: transparent;
    border: 0.5px solid var(--lumina-separator);
    border-radius: var(--lumina-radius-sm);
    color: var(--lumina-text-secondary);
    cursor: pointer;
    display: inline-flex;
    font-size: 11px;
    font-weight: 500;
    padding: 2px 8px;
    transition: all 0.15s ease;

    &:hover {
      background: var(--lumina-control-hover);
      border-color: var(--lumina-text-tertiary);
      color: var(--lumina-text);
    }
  }

  :deep(.md-mermaid-viewport) {
    align-items: center;
    background: color-mix(in srgb, var(--lumina-surface-2) 35%, transparent);
    display: flex;
    justify-content: center;
    min-height: 120px;
    overflow-x: auto;
    overflow-y: hidden;
    padding: 24px 16px;

    svg {
      display: block;
      height: auto;
      max-width: 100%;
    }
  }

  :deep(.md-mermaid-loading) {
    align-items: center;
    color: var(--lumina-text-tertiary);
    display: flex;
    font-size: 12px;
    gap: 8px;
    justify-content: center;
    padding: 24px;
  }

  :deep(.md-mermaid-spinner) {
    animation: md-spin 0.8s linear infinite;
    border: 2px solid var(--lumina-separator);
    border-radius: 50%;
    border-top-color: var(--lumina-primary);
    height: 16px;
    width: 16px;
  }

  :deep(.md-mermaid-error) {
    align-items: center;
    background: color-mix(in srgb, #cf222e 8%, var(--lumina-surface-2));
    border-radius: var(--lumina-radius-sm);
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 16px;
    text-align: center;

    .error-badge {
      background: color-mix(in srgb, #cf222e 15%, transparent);
      border-radius: 4px;
      color: #cf222e;
      font-size: 11px;
      font-weight: 600;
      padding: 2px 6px;
    }

    .error-msg {
      color: var(--lumina-text-secondary);
      font-size: 12px;
      line-height: 1.5;
    }
  }

  :deep(.md-mermaid-code) {
    border-top: 0.5px solid var(--lumina-separator);
    margin: 0;
  }

  /* Tables */
  :deep(.md-table-container) {
    margin: 16px 0;
    max-width: 100%;
    overflow-x: auto;
  }

  :deep(.md-table) {
    border-collapse: collapse;
    border-spacing: 0;
    font-size: 0.94em;
    width: 100%;

    th,
    td {
      border: 0.5px solid var(--lumina-separator);
      padding: 8px 14px;
      text-align: left;
    }

    th {
      background: var(--lumina-surface-2);
      color: var(--lumina-text);
      font-weight: 600;
    }

    tr:nth-child(even) td {
      background: color-mix(in srgb, var(--lumina-surface-2) 40%, transparent);
    }

    tr:hover td {
      background: var(--lumina-control-hover);
    }
  }

  /* Images */
  :deep(.md-image-wrapper) {
    display: block;
    margin: 16px 0;
    text-align: center;
  }

  :deep(.md-image) {
    border: 0.5px solid var(--lumina-separator);
    border-radius: var(--lumina-radius-md);
    box-shadow: var(--lumina-shadow-sm);
    display: inline-block;
    max-height: 80vh;
    max-width: 100%;
    object-fit: contain;
  }

  /* GitHub Alerts / Callouts */
  :deep(.md-alert) {
    border-left: 4px solid var(--lumina-primary);
    border-radius: 0 var(--lumina-radius-md) var(--lumina-radius-md) 0;
    margin: 16px 0;
    padding: 12px 16px;

    .md-alert-title {
      font-size: 13px;
      font-weight: 650;
      margin-bottom: 4px;
    }

    .md-alert-content {
      font-size: 0.94em;

      p {
        margin: 4px 0;
      }
    }

    &.md-alert--note {
      background: color-mix(in srgb, #0969da 8%, var(--lumina-surface-2));
      border-left-color: #0969da;
      .md-alert-title { color: #0969da; }
    }

    &.md-alert--tip {
      background: color-mix(in srgb, #1a7f37 8%, var(--lumina-surface-2));
      border-left-color: #1a7f37;
      .md-alert-title { color: #1a7f37; }
    }

    &.md-alert--important {
      background: color-mix(in srgb, #8250df 8%, var(--lumina-surface-2));
      border-left-color: #8250df;
      .md-alert-title { color: #8250df; }
    }

    &.md-alert--warning {
      background: color-mix(in srgb, #9a6700 8%, var(--lumina-surface-2));
      border-left-color: #9a6700;
      .md-alert-title { color: #9a6700; }
    }

    &.md-alert--caution {
      background: color-mix(in srgb, #cf222e 8%, var(--lumina-surface-2));
      border-left-color: #cf222e;
      .md-alert-title { color: #cf222e; }
    }
  }

  /* Horizontal Rules */
  :deep(hr) {
    background: var(--lumina-separator);
    border: 0;
    height: 1px;
    margin: 24px 0;
  }
}

@keyframes md-spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
