import { invoke } from '@tauri-apps/api/core'
import type { RemoteSkillsRegistry } from '@/types/skills'

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
