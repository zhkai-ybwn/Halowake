import { defineStore } from 'pinia'

export interface RecentMarkdownFile {
  path: string
  name: string
  lastOpened: number
}

export type ReadingWidthMode = 'centered' | 'full'
export type ReadingFontSize = 'normal' | 'large' | 'huge'
export type TocPosition = 'left' | 'right'

const STORAGE_KEYS = {
  recentFiles: 'halowake.markdown.recentFiles',
  readingWidth: 'halowake.markdown.readingWidth',
  fontSize: 'halowake.markdown.fontSize',
  tocOpen: 'halowake.markdown.tocOpen',
  tocPosition: 'halowake.markdown.tocPosition',
} as const

function loadRecentFiles(): RecentMarkdownFile[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEYS.recentFiles)
    return raw ? JSON.parse(raw) : []
  } catch {
    return []
  }
}

export const useMarkdownHistoryStore = defineStore('markdownHistory', {
  state: () => ({
    recentFiles: loadRecentFiles(),
    readingWidth: (localStorage.getItem(STORAGE_KEYS.readingWidth) as ReadingWidthMode) || 'centered',
    fontSize: (localStorage.getItem(STORAGE_KEYS.fontSize) as ReadingFontSize) || 'normal',
    tocOpen: localStorage.getItem(STORAGE_KEYS.tocOpen) !== '0',
    tocPosition: (localStorage.getItem(STORAGE_KEYS.tocPosition) as TocPosition) || 'left',
  }),

  actions: {
    addRecentFile(filePath: string, fileName?: string) {
      if (!filePath) return
      const name = fileName || filePath.split(/[/\\]/).pop() || filePath
      const filtered = this.recentFiles.filter(item => item.path !== filePath)
      this.recentFiles = [
        {
          path: filePath,
          name,
          lastOpened: Date.now(),
        },
        ...filtered,
      ].slice(0, 20)

      localStorage.setItem(STORAGE_KEYS.recentFiles, JSON.stringify(this.recentFiles))
    },

    removeRecentFile(filePath: string) {
      this.recentFiles = this.recentFiles.filter(item => item.path !== filePath)
      localStorage.setItem(STORAGE_KEYS.recentFiles, JSON.stringify(this.recentFiles))
    },

    clearRecentFiles() {
      this.recentFiles = []
      localStorage.removeItem(STORAGE_KEYS.recentFiles)
    },

    setReadingWidth(width: ReadingWidthMode) {
      this.readingWidth = width
      localStorage.setItem(STORAGE_KEYS.readingWidth, width)
    },

    toggleReadingWidth() {
      this.setReadingWidth(this.readingWidth === 'centered' ? 'full' : 'centered')
    },

    setFontSize(size: ReadingFontSize) {
      this.fontSize = size
      localStorage.setItem(STORAGE_KEYS.fontSize, size)
    },

    cycleFontSize() {
      const next: Record<ReadingFontSize, ReadingFontSize> = {
        normal: 'large',
        large: 'huge',
        huge: 'normal',
      }
      this.setFontSize(next[this.fontSize] || 'normal')
    },

    toggleToc() {
      this.tocOpen = !this.tocOpen
      localStorage.setItem(STORAGE_KEYS.tocOpen, this.tocOpen ? '1' : '0')
    },

    toggleTocPosition() {
      this.tocPosition = this.tocPosition === 'left' ? 'right' : 'left'
      localStorage.setItem(STORAGE_KEYS.tocPosition, this.tocPosition)
    },

    setTocPosition(pos: TocPosition) {
      this.tocPosition = pos
      localStorage.setItem(STORAGE_KEYS.tocPosition, pos)
    },
  },
})
