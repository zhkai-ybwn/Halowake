import { invoke } from '@tauri-apps/api/core'
import type {
  CompleteStorageCleanupResult,
  NativeStorageOverview,
  StorageCleanupResult,
  StorageOverview,
  StorageSettings,
} from '@/types/storage'

const EXPIRABLE_STORAGE_ENTRIES = [
  { key: 'lumina.git.commitMessageHistory', timestampKey: 'createdAt' },
  { key: 'lumina.devdock.recentCommands.v1', timestampKey: 'usedAt' },
] as const

export async function loadStorageSettings(): Promise<StorageSettings> {
  return invoke<StorageSettings>('load_storage_settings')
}

export async function saveStorageSettings(settings: StorageSettings): Promise<StorageSettings> {
  return invoke<StorageSettings>('save_storage_settings', { settings })
}

export async function getStorageOverview(): Promise<StorageOverview> {
  const native = await invoke<NativeStorageOverview>('get_storage_overview')
  const localStorageBytes = measureLocalStorage()
  return {
    ...native,
    localStorageBytes,
    totalBytes: native.totalBytes + localStorageBytes,
  }
}

export async function runStorageCleanup(
  retentionDays: number,
  force: boolean,
): Promise<CompleteStorageCleanupResult> {
  const native = await invoke<StorageCleanupResult>('run_storage_cleanup', { force })
  const localStorageReclaimedBytes = native.performed
    ? cleanupExpiredLocalStorage(retentionDays)
    : 0
  return {
    ...native,
    localStorageReclaimedBytes,
    totalReclaimedBytes: native.reclaimedBytes + localStorageReclaimedBytes,
  }
}

function measureLocalStorage() {
  const encoder = new TextEncoder()
  let bytes = 0
  for (let index = 0; index < localStorage.length; index += 1) {
    const key = localStorage.key(index)
    if (!key) continue
    bytes += encoder.encode(key).byteLength
    bytes += encoder.encode(localStorage.getItem(key) ?? '').byteLength
  }
  return bytes
}

function cleanupExpiredLocalStorage(retentionDays: number) {
  const before = measureLocalStorage()
  const cutoff = Date.now() - Math.max(1, retentionDays) * 24 * 60 * 60 * 1000

  for (const entry of EXPIRABLE_STORAGE_ENTRIES) {
    const raw = localStorage.getItem(entry.key)
    if (!raw) continue
    try {
      const records = JSON.parse(raw) as unknown
      if (!Array.isArray(records)) continue
      const retained = records.filter(record => {
        if (!record || typeof record !== 'object') return true
        const timestamp = (record as Record<string, unknown>)[entry.timestampKey]
        return typeof timestamp !== 'number' || timestamp >= cutoff
      })
      localStorage.setItem(entry.key, JSON.stringify(retained))
    } catch (error) {
      console.error(`Failed to clean ${entry.key}`, error)
    }
  }

  return Math.max(0, before - measureLocalStorage())
}
