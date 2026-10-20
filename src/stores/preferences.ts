import { defineStore } from 'pinia'
import { i18n, type AppLocale } from '@/i18n'

export type ThemeMode = 'light' | 'dark' | 'system'
export type CloseAction = 'ask' | 'hideToTray' | 'exit'

const STORAGE_KEYS = {
  locale: 'lumina.preferences.locale',
  themeMode: 'lumina.preferences.themeMode',
  closeAction: 'lumina.preferences.closeAction',
} as const

export const usePreferencesStore = defineStore('preferences', {
  state: () => ({
    locale: (localStorage.getItem(STORAGE_KEYS.locale) as AppLocale | null) ?? 'zh-CN',
    themeMode: (localStorage.getItem(STORAGE_KEYS.themeMode) as ThemeMode | null) ?? 'system',
    closeAction: (localStorage.getItem(STORAGE_KEYS.closeAction) as CloseAction | null) ?? 'ask',
    systemTheme: (typeof window !== 'undefined' && typeof window.matchMedia === 'function' && window.matchMedia('(prefers-color-scheme: dark)').matches
      ? 'dark'
      : 'light') as 'light' | 'dark',
  }),

  getters: {
    resolvedTheme: (state): 'light' | 'dark' =>
      state.themeMode === 'system' ? state.systemTheme : state.themeMode,
  },

  actions: {
    initPreferences() {
      if (typeof window !== 'undefined' && typeof window.matchMedia === 'function') {
        this.systemTheme = window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
      }
      this.syncLocale()
      this.syncThemeToDom()
      this.bindSystemTheme()
    },

    setLocale(locale: AppLocale) {
      this.locale = locale
      localStorage.setItem(STORAGE_KEYS.locale, locale)
      this.syncLocale()
    },

    setThemeMode(mode: ThemeMode) {
      this.themeMode = mode
      localStorage.setItem(STORAGE_KEYS.themeMode, mode)
      this.syncThemeToDom()
    },

    setCloseAction(action: CloseAction) {
      this.closeAction = action
      localStorage.setItem(STORAGE_KEYS.closeAction, action)
    },

    syncLocale() {
      i18n.global.locale.value = this.locale
      document.documentElement.setAttribute('lang', this.locale)
    },

    syncThemeToDom() {
      document.documentElement.setAttribute('data-theme', this.resolvedTheme)
      document.documentElement.setAttribute('data-theme-mode', this.themeMode)
    },

    bindSystemTheme() {
      if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') return

      const media = window.matchMedia('(prefers-color-scheme: dark)')
      media.addEventListener('change', (e: { matches: boolean }) => {
        this.systemTheme = e.matches ? 'dark' : 'light'
        if (this.themeMode === 'system') {
          this.syncThemeToDom()
        }
      })
    },
  },
})
