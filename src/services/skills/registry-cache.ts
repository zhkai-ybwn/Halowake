import { invoke } from '@tauri-apps/api/core'
import type {
  RemoteSkillsRegistry,
  SkillSearchRequest,
  SkillSearchResponse,
} from '@/types/skills'

const LEGACY_KEY = 'lumina.skills.cachedRegistry'

function validRegistry(value: RemoteSkillsRegistry | null): value is RemoteSkillsRegistry {
  return Boolean(value && Array.isArray(value.skills) && value.skills.length > 0)
}

export async function readRegistryCache(): Promise<RemoteSkillsRegistry | null> {
  try {
    const registry = await invoke<RemoteSkillsRegistry | null>('load_skill_registry')
    if (validRegistry(registry)) return registry
  } catch (error) {
    console.warn('Failed to read SQLite skills cache:', error)
  }
  // One-time migration from the earlier web cache, if it contains a real catalog.
  try {
    const raw = localStorage.getItem(LEGACY_KEY)
    const legacy = raw ? JSON.parse(raw) as RemoteSkillsRegistry : null
    if (validRegistry(legacy)) {
      await writeRegistryCache(legacy)
      return legacy
    }
  } catch { /* Legacy cache unavailable. */ }
  return null
}

export async function writeRegistryCache(registry: RemoteSkillsRegistry): Promise<void> {
  await invoke('save_skill_registry', { registry })
  try { localStorage.removeItem(LEGACY_KEY) } catch { /* unavailable */ }
}

export async function searchRegistryCache(request: SkillSearchRequest): Promise<SkillSearchResponse> {
  return await invoke<SkillSearchResponse>('search_skill_registry', { request })
}

export async function rebuildRegistryFtsIndex(force = true): Promise<boolean> {
  return await invoke<boolean>('rebuild_skill_registry_fts', { force })
}

export function formatCatalogRevision(
  version?: string | null,
  updatedAt?: number | null,
  catalogRevision?: string | null
): string | null {
  if (catalogRevision) return catalogRevision
  if (!version) return null
  if (version.includes(':')) return version
  if (updatedAt && updatedAt > 0) {
    return `${version}:${updatedAt}`
  }
  return version
}

export function isRevisionMatching(
  responseRevision: string | null | undefined,
  activeRevision: string | null | undefined,
  localRevNumber?: number
): boolean {
  if (!responseRevision) return true
  if (!activeRevision) return true
  if (responseRevision === activeRevision) return true
  if (localRevNumber !== undefined && responseRevision === String(localRevNumber)) return true
  if (responseRevision.startsWith(activeRevision + ':')) return true
  return false
}
