import type {
  CatalogSkill,
  InstallPreference,
  PipelineStepPlaybook,
  RemoteSkillsRegistry,
  SdlcStage,
  SkillAuthor,
  SkillBadgeType,
  SkillMetrics,
  SkillPipeline,
  SkillSearchRequest,
  SkillSearchResponse,
  SkillUsageGuide,
  SubSkillItem,
} from '@/types/skills'
import { SKILLS_REGISTRY_URLS } from '@/config/skills-registry'
import { i18n } from '@/i18n'
import {
  readRegistryCache,
  rebuildRegistryFtsIndex,
  searchRegistryCache,
  writeRegistryCache,
} from './registry-cache'

export { readRegistryCache, writeRegistryCache } from './registry-cache'

export async function searchSkillRegistry(request: SkillSearchRequest): Promise<SkillSearchResponse> {
  return await searchRegistryCache(request)
}

export async function rebuildSkillRegistryFts(force = true): Promise<boolean> {
  return await rebuildRegistryFtsIndex(force)
}

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

export function saveAllUserCustomCategories(all: Record<string, string[]>): void {
  try {
    localStorage.setItem(STORAGE_KEYS.customCategories, JSON.stringify(all))
  } catch (e) {
    console.warn('Failed to save custom categories:', e)
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
    saveAllUserCustomCategories(all)
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
    saveAllUserCustomCategories(all)
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

const VALID_SDLC_STAGES = new Set<SdlcStage>([
  'requirement',
  'design',
  'coding',
  'testing',
  'delivery',
])

function isSafeRelativeSubPath(subPath: unknown): subPath is string {
  if (typeof subPath !== 'string') return false
  const trimmed = subPath.trim()
  if (!trimmed) return true
  if (trimmed.startsWith('/') || trimmed.startsWith('\\') || trimmed.includes(':')) {
    return false
  }
  const parts = trimmed.replace(/\\/g, '/').split('/')
  return parts.every(part => part.length > 0 && part !== '.' && part !== '..')
}

function isValidRemoteGitUrl(gitUrl: unknown): gitUrl is string {
  if (typeof gitUrl !== 'string') return false
  const trimmed = gitUrl.trim()
  if (!trimmed || trimmed.length > 500 || /\s/.test(trimmed)) return false
  return (
    trimmed.startsWith('https://') ||
    trimmed.startsWith('http://') ||
    trimmed.startsWith('git@')
  )
}

function sanitizeSubSkillItem(raw: unknown): SubSkillItem | null {
  if (!raw || typeof raw !== 'object') return null
  const obj = raw as Record<string, unknown>
  const id = typeof obj.id === 'string' ? obj.id.trim() : ''
  const name = typeof obj.name === 'string' ? obj.name.trim() : ''
  const subPath = typeof obj.subPath === 'string' ? obj.subPath.trim() : ''
  if (!id || !name || !subPath || !isSafeRelativeSubPath(subPath)) {
    return null
  }
  return {
    id,
    name,
    titleZh: typeof obj.titleZh === 'string' && obj.titleZh.trim() ? obj.titleZh.trim() : name,
    description: typeof obj.description === 'string' ? obj.description : '',
    subPath: subPath.replace(/\\/g, '/'),
    tags: Array.isArray(obj.tags) ? obj.tags.filter((t): t is string => typeof t === 'string') : undefined,
  }
}

function sanitizeUsageGuide(raw: unknown): SkillUsageGuide | undefined {
  if (!raw || typeof raw !== 'object') return undefined
  const guide = raw as Record<string, unknown>
  const steps = Array.isArray(guide.steps)
    ? guide.steps.filter((s): s is string => typeof s === 'string' && s.trim().length > 0)
    : []
  return {
    extractorVersion: typeof guide.extractorVersion === 'number' ? guide.extractorVersion : undefined,
    summary: typeof guide.summary === 'string' ? guide.summary : '',
    trigger: typeof guide.trigger === 'string' ? guide.trigger : undefined,
    steps,
    example: typeof guide.example === 'string' ? guide.example : undefined,
    sourceUrl: typeof guide.sourceUrl === 'string' ? guide.sourceUrl : '',
    sourceHash: typeof guide.sourceHash === 'string' ? guide.sourceHash : '',
    extractedAt: typeof guide.extractedAt === 'number' ? guide.extractedAt : 0,
  }
}

function sanitizePlaybook(raw: unknown): PipelineStepPlaybook[] | undefined {
  if (!Array.isArray(raw)) return undefined
  const result: PipelineStepPlaybook[] = []
  for (const item of raw) {
    if (!item || typeof item !== 'object') continue
    const p = item as Record<string, unknown>
    const stage =
      typeof p.stage === 'string' && VALID_SDLC_STAGES.has(p.stage as SdlcStage)
        ? (p.stage as SdlcStage)
        : undefined
    const skillId = typeof p.skillId === 'string' && p.skillId.trim() ? p.skillId.trim() : undefined
    const skillName =
      typeof p.skillName === 'string' && p.skillName.trim() ? p.skillName.trim() : (skillId || '')
    const role = typeof p.role === 'string' ? p.role.trim() : ''
    const howToAct = typeof p.howToAct === 'string' ? p.howToAct.trim() : ''
    const promptExample = typeof p.promptExample === 'string' ? p.promptExample.trim() : ''

    if (!stage || !skillId) continue

    result.push({
      stage,
      skillId,
      skillName,
      role,
      howToAct,
      promptExample,
    })
  }
  return result.length > 0 ? result : undefined
}

export function validateAndSanitizeRemoteRegistry(raw: unknown): {
  skills: CatalogSkill[]
  pipelines: SkillPipeline[]
  rejectedSkillCount: number
} {
  if (!raw || typeof raw !== 'object') {
    throw new Error('注册表数据格式异常：根节点不是对象')
  }
  const candidate = raw as Record<string, unknown>
  if (!Array.isArray(candidate.skills) || candidate.skills.length === 0) {
    throw new Error('注册表数据格式异常：未找到有效的 skills 列表')
  }

  const validSkills: CatalogSkill[] = []
  const seenSkillIds = new Set<string>()
  let rejectedSkillCount = 0

  for (const item of candidate.skills) {
    if (!item || typeof item !== 'object') {
      rejectedSkillCount++
      continue
    }
    const obj = item as Record<string, unknown>
    const id = typeof obj.id === 'string' ? obj.id.trim() : ''
    const name = typeof obj.name === 'string' ? obj.name.trim() : ''
    const gitUrl = obj.gitUrl
    if (!id || !name || !isValidRemoteGitUrl(gitUrl)) {
      rejectedSkillCount++
      continue
    }
    const lowerId = id.toLowerCase()
    if (seenSkillIds.has(lowerId)) {
      rejectedSkillCount++
      continue
    }

    if (obj.subPath !== undefined && obj.subPath !== null && !isSafeRelativeSubPath(obj.subPath)) {
      rejectedSkillCount++
      continue
    }
    const normalizedSubPath =
      typeof obj.subPath === 'string' && obj.subPath.trim()
        ? obj.subPath.trim().replace(/\\/g, '/')
        : undefined

    const description = typeof obj.description === 'string' ? obj.description : ''
    const rawStage = typeof obj.stage === 'string' ? (obj.stage as SdlcStage) : undefined
    const stage: SdlcStage =
      rawStage && VALID_SDLC_STAGES.has(rawStage)
        ? rawStage
        : inferSkillStage(name, description)

    let sanitizedSubSkills: SubSkillItem[] | undefined
    if (Array.isArray(obj.subSkills)) {
      sanitizedSubSkills = obj.subSkills
        .map(sanitizeSubSkillItem)
        .filter((s): s is SubSkillItem => s !== null)
      if (Boolean(obj.isBundle) && sanitizedSubSkills.length !== obj.subSkills.length) {
        // If a bundle declared sub-skills but some were corrupted/invalid, reject the bundle
        rejectedSkillCount++
        continue
      }
    }

    const authorObj =
      obj.author && typeof obj.author === 'object' ? (obj.author as Record<string, unknown>) : {}
    const metricsObj =
      obj.metrics && typeof obj.metrics === 'object' ? (obj.metrics as Record<string, unknown>) : {}

    const authorName = typeof authorObj.name === 'string' && authorObj.name.trim() ? authorObj.name.trim() : 'Community'
    const authorUrl = typeof authorObj.url === 'string' && authorObj.url.trim().startsWith('http') ? authorObj.url.trim() : undefined
    const authorAvatar = typeof authorObj.avatar === 'string' && authorObj.avatar.trim().startsWith('http') ? authorObj.avatar.trim() : undefined
    const authorVerified = typeof authorObj.verified === 'boolean' ? authorObj.verified : undefined
    const author: SkillAuthor = {
      name: authorName,
      ...(authorUrl ? { url: authorUrl } : {}),
      ...(authorAvatar ? { avatar: authorAvatar } : {}),
      ...(authorVerified !== undefined ? { verified: authorVerified } : {}),
    }

    const VALID_BADGES = new Set<SkillBadgeType>(['verified', 'trending', 'security', 'community'])
    const badge: SkillBadgeType =
      typeof obj.badge === 'string' && VALID_BADGES.has(obj.badge as SkillBadgeType)
        ? (obj.badge as SkillBadgeType)
        : 'community'

    const rawScore = typeof obj.halowakeScore === 'number' && Number.isFinite(obj.halowakeScore) ? obj.halowakeScore : undefined
    const halowakeScore = rawScore !== undefined ? Math.max(0, Math.min(100, Math.round(rawScore))) : undefined

    const metrics: SkillMetrics = {
      githubStars: typeof metricsObj.githubStars === 'number' && Number.isFinite(metricsObj.githubStars) ? Math.max(0, metricsObj.githubStars) : 0,
      forks: typeof metricsObj.forks === 'number' && Number.isFinite(metricsObj.forks) ? Math.max(0, metricsObj.forks) : 0,
      lastCommitDaysAgo: typeof metricsObj.lastCommitDaysAgo === 'number' && Number.isFinite(metricsObj.lastCommitDaysAgo) ? Math.max(0, metricsObj.lastCommitDaysAgo) : 0,
      issueCloseRate: typeof metricsObj.issueCloseRate === 'number' && Number.isFinite(metricsObj.issueCloseRate) ? Math.max(0, Math.min(100, metricsObj.issueCloseRate)) : 0,
      securityAuditPassed: Boolean(metricsObj.securityAuditPassed),
    }

    const tags = Array.isArray(obj.tags)
      ? obj.tags.filter((t): t is string => typeof t === 'string' && t.trim().length > 0).map(t => t.trim())
      : []
    const recommendedWith = Array.isArray(obj.recommendedWith)
      ? obj.recommendedWith.filter((t): t is string => typeof t === 'string' && t.trim().length > 0).map(t => t.trim())
      : []
    const compatibleAgents = Array.isArray(obj.compatibleAgents)
      ? obj.compatibleAgents.filter((t): t is string => typeof t === 'string' && t.trim().length > 0).map(t => t.trim().toLowerCase())
      : []

    const sourceId =
      typeof obj.sourceId === 'string' && obj.sourceId.trim() ? obj.sourceId.trim() : undefined

    seenSkillIds.add(lowerId)
    validSkills.push({
      id,
      name,
      titleZh: typeof obj.titleZh === 'string' && obj.titleZh.trim() ? obj.titleZh.trim() : name,
      description,
      stage,
      category: typeof obj.category === 'string' && obj.category.trim() ? obj.category.trim() : 'general',
      author,
      gitUrl: gitUrl.trim(),
      subPath: normalizedSubPath,
      stars: typeof obj.stars === 'number' && Number.isFinite(obj.stars) ? Math.max(0, obj.stars) : 0,
      halowakeScore,
      badge,
      metrics,
      tags,
      recommendedWith,
      compatibleAgents,
      howToUse: typeof obj.howToUse === 'string' ? obj.howToUse : '',
      promptExample: typeof obj.promptExample === 'string' ? obj.promptExample : undefined,
      usageGuide: sanitizeUsageGuide(obj.usageGuide),
      isBundle: Boolean(obj.isBundle),
      bundleCount: sanitizedSubSkills ? sanitizedSubSkills.length : undefined,
      subSkills: sanitizedSubSkills,
      sourceId,
    })
  }

  if (validSkills.length === 0) {
    throw new Error('注册表数据格式异常：所有技能条目均未通过 Schema 校验')
  }

  const validPipelines: SkillPipeline[] = []
  if (Array.isArray(candidate.pipelines)) {
    for (const item of candidate.pipelines) {
      if (!item || typeof item !== 'object') continue
      const p = item as Record<string, unknown>
      const id = typeof p.id === 'string' ? p.id.trim() : ''
      const title = typeof p.title === 'string' ? p.title.trim() : ''
      const declaredSkills = Array.isArray(p.skills)
        ? p.skills.filter((s): s is string => typeof s === 'string' && s.trim().length > 0).map(s => s.trim())
        : []
      if (!id || !title || declaredSkills.length === 0) continue
      validPipelines.push({
        id,
        title,
        subtitle: typeof p.subtitle === 'string' ? p.subtitle : '',
        description: typeof p.description === 'string' ? p.description : '',
        targetScenario: typeof p.targetScenario === 'string' ? p.targetScenario : '',
        badge: typeof p.badge === 'string' ? p.badge : 'Core',
        icon: typeof p.icon === 'string' ? p.icon : 'solar:rocket-2-linear',
        skills: declaredSkills,
        playbook: sanitizePlaybook(p.playbook),
      })
    }
  }

  return {
    skills: validSkills,
    pipelines: validPipelines.length > 0 ? validPipelines : FALLBACK_SEED_PIPELINES,
    rejectedSkillCount,
  }
}

/**
 * 在线同步远端官方注册表（从独立的 halowake-skills-registry 数据中枢抓取）
 * 严格要求：只有 SQLite / 本地缓存落盘成功后，才允许返回 status === 'synced' 并更新 lastSyncTime。
 */
export async function syncOnlineRegistry(): Promise<{
  status: 'synced' | 'persist_failed' | 'cached_fallback' | 'failed'
  success: boolean
  updatedCount: number
  totalCount: number
  lastSyncAt: number
  message: string
  skills: CatalogSkill[]
  pipelines: SkillPipeline[]
  version?: string
  catalogRevision?: string
}> {
  try {
    let rawData: Partial<RemoteSkillsRegistry> | null = null
    let validated: { skills: CatalogSkill[]; pipelines: SkillPipeline[]; rejectedSkillCount: number } | null = null
    let lastError: unknown

    for (const url of SKILLS_REGISTRY_URLS) {
      try {
        const res = await fetch(url, {
          signal: AbortSignal.timeout(12000),
          cache: 'no-store',
        })
        if (!res.ok) throw new Error(`HTTP ${res.status}: ${res.statusText}`)
        const candidate = (await res.json()) as Partial<RemoteSkillsRegistry>
        validated = validateAndSanitizeRemoteRegistry(candidate)
        rawData = candidate
        break
      } catch (error) {
        lastError = error
      }
    }

    if (!rawData || !validated) {
      throw lastError || new Error('注册表不可用')
    }

    const { skills: validSkills, pipelines: validPipelines } = validated
    const now = Date.now()

    const registryData: RemoteSkillsRegistry = {
      version: typeof rawData.version === 'string' ? rawData.version : '1.0.0',
      updatedAt: typeof rawData.updatedAt === 'number' ? rawData.updatedAt : now,
      catalogRevision:
        typeof rawData.catalogRevision === 'string' ? rawData.catalogRevision : undefined,
      description: typeof rawData.description === 'string' ? rawData.description : undefined,
      skills: validSkills,
      pipelines: validPipelines,
    }

    try {
      await writeRegistryCache(registryData)
    } catch (persistError) {
      console.warn('Failed to persist skills registry to SQLite cache:', persistError)
      const persistMsg =
        persistError instanceof Error ? persistError.message : String(persistError)
      const cached = await readRegistryCache()
      const consistentSkills = cached?.skills || FALLBACK_SEED_SKILLS
      const consistentPipelines = cached?.pipelines || FALLBACK_SEED_PIPELINES

      return {
        status: 'persist_failed',
        success: false,
        updatedCount: 0,
        totalCount: consistentSkills.length,
        lastSyncAt: getLastSyncTime() || now,
        message: `远端目录已获取，但写入本地索引库失败（${persistMsg}）；已保持现有缓存一致`,
        skills: consistentSkills,
        pipelines: consistentPipelines,
      }
    }

    localStorage.setItem(STORAGE_KEYS.lastSyncTime, String(now))

    return {
      status: 'synced',
      success: true,
      updatedCount: validSkills.length,
      totalCount: validSkills.length,
      lastSyncAt: now,
      message: i18n.global.t('skills.review.syncSuccess', { count: validSkills.length }),
      skills: validSkills,
      pipelines: validPipelines,
      version: registryData.version,
      catalogRevision: registryData.catalogRevision,
    }
  } catch (err: unknown) {
    console.warn('Sync online skills registry failed, falling back to local cache:', err)
    const errMessage = (err as Error)?.message || i18n.global.t('skills.review.timeout')
    const cached = await readRegistryCache()
    const fallbackSkills = cached?.skills || FALLBACK_SEED_SKILLS
    const fallbackPipelines = cached?.pipelines || FALLBACK_SEED_PIPELINES
    const now = Date.now()
    const hasCached = fallbackSkills.length > 0

    return {
      status: hasCached ? 'cached_fallback' : 'failed',
      success: false,
      updatedCount: 0,
      totalCount: fallbackSkills.length,
      lastSyncAt: getLastSyncTime() || now,
      message: hasCached
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
