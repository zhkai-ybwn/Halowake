export interface StorageSettings {
  autoCleanupEnabled: boolean
  retentionDays: number
  lastCleanupAt: number | null
}

export interface NativeStorageOverview {
  configurationBytes: number
  dataBytes: number
  cacheBytes: number
  logBytes: number
  totalBytes: number
}

export interface StorageOverview extends NativeStorageOverview {
  localStorageBytes: number
  totalBytes: number
}

export interface StorageCleanupResult {
  performed: boolean
  reclaimedBytes: number
  deletedFiles: number
  deletedRecords: number
  completedAt: number | null
}

export interface CompleteStorageCleanupResult extends StorageCleanupResult {
  localStorageReclaimedBytes: number
  totalReclaimedBytes: number
}
