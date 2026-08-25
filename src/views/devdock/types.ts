import type { ProjectManifest } from '@/services/project/project-service'

export interface DevDockProject {
  path: string
  name: string
  loading: boolean
  error: string
  manifest: ProjectManifest | null
  openedAt: number
  scanned?: boolean
}

export interface StoredProject {
  path?: string
  name?: string
  openedAt?: number
}

export interface RecentCommand {
  projectPath: string
  projectName: string
  commandId: string
  commandName: string
  commandPreview: string
  executor: string
  source: string
  usedAt: number
}

export type ScriptSort = 'priority' | 'name' | 'recent'
