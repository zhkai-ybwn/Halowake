import { defineStore } from 'pinia'
import { ref } from 'vue'
import { check, type Update } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'
import { getApplicationVersion } from '@/services/app-service'

export interface CheckUpdateOptions {
  silent?: boolean
  openModalIfAvailable?: boolean
}

export const useUpdaterStore = defineStore('updater', () => {
  const isChecking = ref(false)
  const updateAvailable = ref(false)
  const currentVersion = ref('')
  const newVersion = ref('')
  const releaseDate = ref('')
  const releaseNotes = ref('')
  const isDownloading = ref(false)
  const downloadProgress = ref(0)
  const downloadedBytes = ref(0)
  const totalBytes = ref(0)
  const isReadyToRelaunch = ref(false)
  const errorMessage = ref<string | null>(null)
  const modalVisible = ref(false)
  const lastCheckTime = ref<Date | null>(null)

  let activeUpdate: Update | null = null

  async function checkForUpdates(options: CheckUpdateOptions = {}): Promise<boolean> {
    const { silent = false, openModalIfAvailable = true } = options
    if (isChecking.value || isDownloading.value) {
      return updateAvailable.value
    }

    isChecking.value = true
    errorMessage.value = null

    try {
      try {
        currentVersion.value = await getApplicationVersion()
      } catch {
        currentVersion.value = '1.1.2'
      }

      const update = await check()
      lastCheckTime.value = new Date()

      if (update?.available) {
        activeUpdate = update
        updateAvailable.value = true
        newVersion.value = update.version || ''
        releaseDate.value = update.date || ''
        releaseNotes.value = update.body || ''
        isReadyToRelaunch.value = false
        downloadProgress.value = 0
        downloadedBytes.value = 0
        totalBytes.value = 0

        if (openModalIfAvailable) {
          modalVisible.value = true
        }
        return true
      } else {
        activeUpdate = null
        updateAvailable.value = false
        return false
      }
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err)
      console.warn('[Updater] Check failed:', msg)

      // When GitHub has no release yet (404), Tauri returns "Could not fetch a valid release JSON from the remote"
      const isNotFoundOrEmptyRelease =
        msg.includes('Could not fetch a valid release JSON') ||
        msg.includes('404') ||
        msg.includes('status code 404')

      if (isNotFoundOrEmptyRelease) {
        activeUpdate = null
        updateAvailable.value = false
        lastCheckTime.value = new Date()
        return false
      }

      if (!silent) {
        errorMessage.value = msg
        throw err
      }
      return false
    } finally {
      isChecking.value = false
    }
  }

  async function startDownloadAndInstall() {
    if (!activeUpdate || isDownloading.value) return

    isDownloading.value = true
    errorMessage.value = null
    downloadProgress.value = 0
    downloadedBytes.value = 0
    totalBytes.value = 0

    try {
      let downloaded = 0
      let total = 0

      await activeUpdate.downloadAndInstall((event) => {
        if (event.event === 'Started') {
          total = event.data.contentLength || 0
          totalBytes.value = total
        } else if (event.event === 'Progress') {
          downloaded += event.data.chunkLength
          downloadedBytes.value = downloaded
          if (total > 0) {
            downloadProgress.value = Math.min(100, Math.round((downloaded / total) * 100))
          }
        } else if (event.event === 'Finished') {
          downloadProgress.value = 100
        }
      })

      isReadyToRelaunch.value = true
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err)
      console.error('[Updater] Download & Install failed:', msg)
      errorMessage.value = msg
      throw err
    } finally {
      isDownloading.value = false
    }
  }

  async function relaunchApplication() {
    try {
      await relaunch()
    } catch (err: unknown) {
      console.error('[Updater] Relaunch failed:', err)
      throw err
    }
  }

  function openModal() {
    modalVisible.value = true
  }

  function closeModal() {
    if (isDownloading.value) return
    modalVisible.value = false
  }

  return {
    isChecking,
    updateAvailable,
    currentVersion,
    newVersion,
    releaseDate,
    releaseNotes,
    isDownloading,
    downloadProgress,
    downloadedBytes,
    totalBytes,
    isReadyToRelaunch,
    errorMessage,
    modalVisible,
    lastCheckTime,
    checkForUpdates,
    startDownloadAndInstall,
    relaunchApplication,
    openModal,
    closeModal,
  }
})
