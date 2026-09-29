import type {
  CatalogSkill,
  InstallPreference,
  RemoteSkillsRegistry,
  SdlcStage,
  SkillPipeline,
} from '@/types/skills'
import { SKILLS_REGISTRY_URLS } from '@/config/skills-registry'
import { i18n } from '@/i18n'
import { readRegistryCache, writeRegistryCache } from './registry-cache'

const STORAGE_KEYS = {
  stars: 'lumina.skills.stars',
  installPref: 'lumina.skills.installPreference',
  customCategories: 'lumina.skills.userCustomCategories',
  lastSyncTime: 'lumina.skills.lastSyncTime',
} as const

// Never present unverified demonstration records as installable offline skills.
export const FALLBACK_SEED_SKILLS: CatalogSkill[] = []

export const FALLBACK_SEED_PIPELINES: SkillPipeline[] = []

// 本地星标管理
export function getLocalStars(): Set<string> {
  try {
    const raw = localStorage.getItem(STORAGE_KEYS.stars)
    if (!raw) return new Set()
    const arr = JSON.parse(raw) as string[]
    return new Set(Array.isArray(arr) ? arr : [])
  } catch {
    return new Set()
  }
}

export function saveLocalStars(stars: Set<string>): void {
  try {
    localStorage.setItem(STORAGE_KEYS.stars, JSON.stringify(Array.from(stars)))
  } catch (e) {
    console.warn('Failed to save stars:', e)
  }
}

// 用户自定义分类 (Custom Categories / Tags) 管理
export function getUserCustomCategories(): Record<string, string[]> {
  try {
    const raw = localStorage.getItem(STORAGE_KEYS.customCategories)
    if (!raw) return {}
    return JSON.parse(raw) as Record<string, string[]>
  } catch {
    return {}
  }
}

export function saveUserCustomCategory(skillId: string, category: string): void {
  try {
    const all = getUserCustomCategories()
    const clean = category.trim()
    if (!clean) return
    const list = all[skillId] || []
    if (!list.includes(clean)) {
      list.push(clean)
    }
    all[skillId] = list
    localStorage.setItem(STORAGE_KEYS.customCategories, JSON.stringify(all))
  } catch (e) {
    console.warn('Failed to save custom category:', e)
  }
}

export function removeUserCustomCategory(skillId: string, category: string): void {
  try {
    const all = getUserCustomCategories()
    if (!all[skillId]) return
    all[skillId] = all[skillId].filter(c => c !== category)
    if (all[skillId].length === 0) {
      delete all[skillId]
    }
    localStorage.setItem(STORAGE_KEYS.customCategories, JSON.stringify(all))
  } catch (e) {
    console.warn('Failed to remove custom category:', e)
  }
}

export function getAllUserCustomCategories(): string[] {
  const all = getUserCustomCategories()
  const set = new Set<string>()
  Object.values(all).forEach(tags => tags.forEach(t => set.add(t)))
  return Array.from(set)
}

// 安装偏好记忆管理
export function getLocalInstallPreference(): InstallPreference {
  try {
    const raw = localStorage.getItem(STORAGE_KEYS.installPref)
    if (!raw) {
      return {
        targetMode: 'project',
        targetAgents: ['antigravity'],
        addToGitignore: true,
        remember: true,
      }
    }
    const parsed = JSON.parse(raw) as Partial<InstallPreference>
    return {
      targetMode: parsed.targetMode || 'project',
      targetAgents:
        Array.isArray(parsed.targetAgents) && parsed.targetAgents.length > 0
          ? parsed.targetAgents
          : ['antigravity'],
      addToGitignore: parsed.addToGitignore !== undefined ? parsed.addToGitignore : true,
      remember: parsed.remember !== undefined ? parsed.remember : true,
      customProjectPath: parsed.customProjectPath,
    }
  } catch {
    return {
      targetMode: 'project',
      targetAgents: ['antigravity'],
      addToGitignore: true,
      remember: true,
    }
  }
}

export function saveLocalInstallPreference(pref: InstallPreference): void {
  try {
    localStorage.setItem(STORAGE_KEYS.installPref, JSON.stringify(pref))
  } catch (e) {
    console.warn('Failed to save install preference:', e)
  }
}

export function getLastSyncTime(): number | null {
  try {
    const raw = localStorage.getItem(STORAGE_KEYS.lastSyncTime)
    return raw ? Number(raw) : null
  } catch {
    return null
  }
}

/**
 * 加载技能市场列表（优先读取本地离线缓存，其次冷启动种子）
 */
export async function loadHarvestedCatalogSkills(): Promise<CatalogSkill[]> {
  const cached = await readRegistryCache()
  if (cached && Array.isArray(cached.skills) && cached.skills.length > 0) {
    return cached.skills
  }
  return FALLBACK_SEED_SKILLS.map(s => ({ ...s }))
}

/**
 * 加载流水线套餐列表
 */
export async function loadHarvestedPipelines(): Promise<SkillPipeline[]> {
  const cached = await readRegistryCache()
  if (cached && Array.isArray(cached.pipelines) && cached.pipelines.length > 0) {
    return cached.pipelines
  }
  return FALLBACK_SEED_PIPELINES.map(p => ({ ...p }))
}

/**
 * 在线同步远端官方注册表（从独立的 halowake-skills-registry 数据中枢抓取）
 */
export async function syncOnlineRegistry(): Promise<{
  success: boolean
  updatedCount: number
  totalCount: number
  lastSyncAt: number
  message: string
  skills: CatalogSkill[]
  pipelines: SkillPipeline[]
}> {
  try {
    let data: Partial<RemoteSkillsRegistry> | null = null
    let lastError: unknown
    for (const url of SKILLS_REGISTRY_URLS) {
      try {
        const res = await fetch(url, {
          signal: AbortSignal.timeout(12000),
          cache: 'no-store',
        })
        if (!res.ok) throw new Error(`HTTP ${res.status}: ${res.statusText}`)
        const candidate = (await res.json()) as Partial<RemoteSkillsRegistry>
        if (!Array.isArray(candidate.skills) || candidate.skills.length === 0) {
          throw new Error('注册表数据格式异常')
        }
        data = candidate
        break
      } catch (error) {
        lastError = error
      }
    }
    if (!data) throw lastError || new Error('注册表不可用')
    if (!data || !Array.isArray(data.skills) || data.skills.length === 0) {
      throw new Error('注册表数据格式异常：未找到 skills 列表')
    }

    const validSkills: CatalogSkill[] = data.skills
    const validPipelines: SkillPipeline[] = Array.isArray(data.pipelines)
      ? data.pipelines
      : FALLBACK_SEED_PIPELINES
    const now = Date.now()

    const registryData: RemoteSkillsRegistry = {
      version: data.version || '1.0.0',
      updatedAt: data.updatedAt || now,
      description: data.description,
      skills: validSkills,
      pipelines: validPipelines,
    }

    try { await writeRegistryCache(registryData) } catch (error) {
      console.warn('Failed to cache skills registry:', error)
    }
    localStorage.setItem(STORAGE_KEYS.lastSyncTime, String(now))

    return {
      success: true,
      updatedCount: validSkills.length,
      totalCount: validSkills.length,
      lastSyncAt: now,
      message: i18n.global.t('skills.review.syncSuccess', { count: validSkills.length }),
      skills: validSkills,
      pipelines: validPipelines,
    }
  } catch (err: unknown) {
    console.warn('Sync online skills registry failed, falling back to local cache:', err)
    const errMessage = (err as Error)?.message || i18n.global.t('skills.review.timeout')
    const cached = await readRegistryCache()
    const fallbackSkills = cached?.skills || FALLBACK_SEED_SKILLS
    const fallbackPipelines = cached?.pipelines || FALLBACK_SEED_PIPELINES
    const now = Date.now()

    return {
      success: false,
      updatedCount: 0,
      totalCount: fallbackSkills.length,
      lastSyncAt: getLastSyncTime() || now,
      message:
        fallbackSkills.length > 0
          ? i18n.global.t('skills.review.syncCached', { error: errMessage, count: fallbackSkills.length })
          : i18n.global.t('skills.review.syncFailed', { error: errMessage }),
      skills: fallbackSkills,
      pipelines: fallbackPipelines,
    }
  }
}

/**
 * 自动推断本地已安装技能的 SDLC 阶段分类（智能归类逻辑）
 */
export function inferSkillStage(name: string, description: string = ''): SdlcStage {
  const text = `${name} ${description}`.toLowerCase()
  if (
    text.includes('prd') ||
    text.includes('spec') ||
    text.includes('req') ||
    text.includes('story') ||
    text.includes('需求')
  ) {
    return 'requirement'
  }
  if (
    text.includes('design') ||
    text.includes('ui') ||
    text.includes('ux') ||
    text.includes('style') ||
    text.includes('theme') ||
    text.includes('设计') ||
    text.includes('macos')
  ) {
    return 'design'
  }
  if (
    text.includes('test') ||
    text.includes('vitest') ||
    text.includes('jest') ||
    text.includes('security') ||
    text.includes('测试') ||
    text.includes('safe')
  ) {
    return 'testing'
  }
  if (
    text.includes('commit') ||
    text.includes('git') ||
    text.includes('release') ||
    text.includes('career') ||
    text.includes('交付') ||
    text.includes('changelog')
  ) {
    return 'delivery'
  }
  return 'coding'
}
