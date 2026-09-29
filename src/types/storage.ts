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

export interface AgentStorageLocation {
  kind: 'home' | 'data' | 'cache'
  path: string
  bytes: number
  incomplete: boolean
  largestChildren: AgentStorageChild[]
}

export interface AgentStorageChild {
  name: string
  path: string
  bytes: number
}

export interface AgentStorageItem {
  id: string
  label: string
  totalBytes: number
  locations: AgentStorageLocation[]
}

export interface AgentStorageOverview {
  totalBytes: number
  systemDrive: string | null
  systemDriveBytes: number
  agents: AgentStorageItem[]
  caches: AgentCacheItem[]
}

export interface AgentCacheItem {
  key: string
  agentLabel: string
  path: string
  bytes: number
  incomplete: boolean
}

export interface AgentCacheCleanupResult {
  reclaimedBytes: number
  skippedEntries: number
}
