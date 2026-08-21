import { defineStore } from 'pinia'
import {
  getStorageOverview,
  loadStorageSettings,
  runStorageCleanup,
  saveStorageSettings,
} from '@/services/storage-service'
import type {
  CompleteStorageCleanupResult,
  StorageOverview,
  StorageSettings,
} from '@/types/storage'

const DEFAULT_SETTINGS: StorageSettings = {
  autoCleanupEnabled: true,
  retentionDays: 90,
  lastCleanupAt: null,
}

const EMPTY_OVERVIEW: StorageOverview = {
  configurationBytes: 0,
  dataBytes: 0,
  cacheBytes: 0,
  logBytes: 0,
  localStorageBytes: 0,
  totalBytes: 0,
}

const CLEANUP_CHECK_INTERVAL = 24 * 60 * 60 * 1000
let cleanupTimer: number | null = null

export const useStorageStore = defineStore('storage', {
  state: () => ({
    settings: { ...DEFAULT_SETTINGS },
    overview: { ...EMPTY_OVERVIEW },
    loading: false,
    saving: false,
    cleaning: false,
    initialized: false,
    error: '',
  }),

  actions: {
    async initStorage() {
      if (this.initialized) return
      this.loading = true
      this.error = ''
      try {
        this.settings = await loadStorageSettings()
        await this.runCleanup(false)
        await this.refreshOverview()
        this.initialized = true
        this.startCleanupTimer()
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error)
        console.error(error)
      } finally {
        this.loading = false
      }
    },

    async refreshOverview() {
      this.overview = await getStorageOverview()
    },

    async updateSettings(patch: Partial<Pick<StorageSettings, 'autoCleanupEnabled' | 'retentionDays'>>) {
      this.saving = true
      this.error = ''
      try {
        this.settings = await saveStorageSettings({
          ...this.settings,
          ...patch,
        })
        await this.refreshOverview()
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error)
        throw error
      } finally {
        this.saving = false
      }
    },

    async runCleanup(force = true): Promise<CompleteStorageCleanupResult> {
      this.cleaning = true
      this.error = ''
      try {
        const result = await runStorageCleanup(this.settings.retentionDays, force)
        if (result.performed && result.completedAt) {
          this.settings.lastCleanupAt = result.completedAt
        }
        await this.refreshOverview()
        return result
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error)
        throw error
      } finally {
        this.cleaning = false
      }
    },

    startCleanupTimer() {
      if (cleanupTimer !== null) return
      cleanupTimer = window.setInterval(() => {
        void this.runCleanup(false).catch(error => console.error(error))
      }, CLEANUP_CHECK_INTERVAL)
    },
  },
})
