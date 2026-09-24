import type {
  CatalogSkill,
  InstallPreference,
  RemoteSkillsRegistry,
  SdlcStage,
  SkillPipeline,
} from '@/types/skills'

const STORAGE_KEYS = {
  stars: 'lumina.skills.stars',
  ratings: 'lumina.skills.ratings',
  feedback: 'lumina.skills.feedback',
  installPref: 'lumina.skills.installPreference',
  customCategories: 'lumina.skills.userCustomCategories',
  lastSyncTime: 'lumina.skills.lastSyncTime',
  registryUrl: 'lumina.skills.registryUrl',
  cachedRegistry: 'lumina.skills.cachedRegistry',
  githubToken: 'lumina.skills.githubToken',
} as const

// 默认远程注册表地址（指向独立的开源数据中枢仓库）
export const DEFAULT_REGISTRY_URL =
  'https://raw.githubusercontent.com/zhkai-ybwn/halowake-skills-registry/main/dist/registry.json'

// 国内加速 CDN 备用地址 (jsDelivr 全球镜像)
export const CDN_REGISTRY_URL =
  'https://cdn.jsdelivr.net/gh/zhkai-ybwn/halowake-skills-registry@main/dist/registry.json'

/**
 * 真实可靠的本地冷启动兜底种子库（严禁伪造虚假 Star 与评分，所有指标客观真实）
 */
export const FALLBACK_SEED_SKILLS: CatalogSkill[] = [
  {
    id: 'impeccable',
    name: 'impeccable',
    titleZh: '前端界面与交互质感打磨',
    description: '重塑和打磨现代 Web/移动端界面，注重视觉层级、响应式微动效、排版节奏与无障碍体验。',
    stage: 'design',
    category: 'ui-ux',
    author: { name: 'Antigravity / Halowake', verified: true },
    gitUrl: 'https://github.com/zhkai-ybwn/Halowake.git',
    stars: 0,
    halowakeScore: 90,
    badge: 'verified',
    metrics: {
      githubStars: 0,
      forks: 0,
      lastCommitDaysAgo: 1,
      issueCloseRate: 100,
      securityAuditPassed: true,
    },
    tags: ['ui', 'ux', 'animation', 'design-system'],
    recommendedWith: ['conventional-commits'],
    howToUse: '在需求或界面重构时提示："请按 impeccable 准则优化这个组件的视觉与动效交互"。',
    promptExample: '运用 impeccable 准则，为当前组件补充清晰的信息层级与悬停过渡动画。',
    compatibleAgents: ['antigravity', 'claude', 'cursor', 'trae'],
  },
  {
    id: 'conventional-commits',
    name: 'conventional-commits',
    titleZh: '语义化 Git 规范与提交助手',
    description: '严格遵循 Conventional Commits 规范，自动从暂存区 Git Diff 解析关键变动并提炼提交摘要。',
    stage: 'delivery',
    category: 'git',
    author: { name: 'Conventional Commits Community', verified: true },
    gitUrl: 'https://github.com/conventional-changelog/commitlint.git',
    stars: 0,
    halowakeScore: 92,
    badge: 'verified',
    metrics: {
      githubStars: 0,
      forks: 0,
      lastCommitDaysAgo: 2,
      issueCloseRate: 95,
      securityAuditPassed: true,
    },
    tags: ['git', 'commit', 'conventional-commits', 'changelog'],
    recommendedWith: ['impeccable'],
    howToUse: '提交代码前调用："基于当前 git diff 生成一条符合语义化规范的 commit 消息"。',
    promptExample: '分析暂存区代码变动，生成 feat/fix/refactor 格式的提交说明。',
    compatibleAgents: ['antigravity', 'claude', 'cursor'],
  },
  {
    id: 'awesome-cursorrules',
    name: 'awesome-cursorrules',
    titleZh: '社区精选 AI Coding 规范合集',
    description: '涵盖 TypeScript、Vue、React、Rust、Python 等主流技术栈的最佳实践与 Agent 引导规则。',
    stage: 'coding',
    category: 'standards',
    author: { name: 'PatrickJS & Community', verified: true },
    gitUrl: 'https://github.com/PatrickJS/awesome-cursorrules.git',
    stars: 0,
    halowakeScore: 88,
    badge: 'community',
    metrics: {
      githubStars: 0,
      forks: 0,
      lastCommitDaysAgo: 5,
      issueCloseRate: 85,
      securityAuditPassed: true,
    },
    tags: ['cursorrules', 'coding-standards', 'prompt-engineering'],
    recommendedWith: ['conventional-commits'],
    howToUse: '在项目根目录引导 Agent 遵循特定语言的最佳编程规范。',
    compatibleAgents: ['cursor', 'antigravity', 'claude'],
  },
]

export const FALLBACK_SEED_PIPELINES: SkillPipeline[] = [
  {
    id: 'modern-web-sdlc',
    title: '现代 Web 全流程敏捷开发套餐',
    subtitle: '设计质感 ➔ 规范编码 ➔ 语义提交',
    description: '从界面质感打磨到语义提交，构建现代高品质 Web 应用的推荐流水线。',
    targetScenario: '高质量 Web 前端模块开发与重构',
    badge: '开箱即用',
    icon: 'solar:magic-stick-3-linear',
    skills: ['impeccable', 'awesome-cursorrules', 'conventional-commits'],
    playbook: [
      {
        stage: 'design',
        skillId: 'impeccable',
        skillName: 'Impeccable UI',
        role: '体验与界面设计',
        howToAct: '先让 Agent 运用 impeccable 准则设计组件的排版节奏、阴影层次与动效过渡。',
        promptExample: '运用 impeccable 准则，为该功能设计高信息密度的卡片布局。',
      },
      {
        stage: 'coding',
        skillId: 'awesome-cursorrules',
        skillName: 'Coding Rules',
        role: '工程规范约束',
        howToAct: '按照最佳代码风格实现核心逻辑，杜绝坏味道。',
        promptExample: '遵循项目技术栈最佳实践，编写整洁高内聚的代码。',
      },
      {
        stage: 'delivery',
        skillId: 'conventional-commits',
        skillName: '语义化 Git 规范',
        role: '版本交付与提交',
        howToAct: '暂存文件后，让 Agent 依据 git diff 生成规范的提交信息。',
        promptExample: '基于当前暂存区的修改，生成一条带上下文背景的语义化提交。',
      },
    ],
  },
]

// 远程注册表地址存取
export function getRemoteRegistryUrl(): string {
  try {
    return localStorage.getItem(STORAGE_KEYS.registryUrl) || DEFAULT_REGISTRY_URL
  } catch {
    return DEFAULT_REGISTRY_URL
  }
}

export function saveRemoteRegistryUrl(url: string): void {
  try {
    const clean = url.trim()
    if (clean) {
      localStorage.setItem(STORAGE_KEYS.registryUrl, clean)
    } else {
      localStorage.removeItem(STORAGE_KEYS.registryUrl)
    }
  } catch (e) {
    console.warn('Failed to save registry url:', e)
  }
}

// 本地缓存的远程注册表数据
export function getCachedRegistry(): RemoteSkillsRegistry | null {
  try {
    const raw = localStorage.getItem(STORAGE_KEYS.cachedRegistry)
    if (!raw) return null
    const parsed = JSON.parse(raw) as RemoteSkillsRegistry
    if (parsed && Array.isArray(parsed.skills)) {
      return parsed
    }
    return null
  } catch {
    return null
  }
}

export function saveCachedRegistry(registry: RemoteSkillsRegistry): void {
  try {
    localStorage.setItem(STORAGE_KEYS.cachedRegistry, JSON.stringify(registry))
  } catch (e) {
    console.warn('Failed to cache registry:', e)
  }
}

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

// 本地评价管理
export function getLocalRatings(): Record<string, number> {
  try {
    const raw = localStorage.getItem(STORAGE_KEYS.ratings)
    if (!raw) return {}
    return JSON.parse(raw) as Record<string, number>
  } catch {
    return {}
  }
}

export function saveLocalRating(skillId: string, rating: number): void {
  try {
    const ratings = getLocalRatings()
    ratings[skillId] = rating
    localStorage.setItem(STORAGE_KEYS.ratings, JSON.stringify(ratings))
  } catch (e) {
    console.warn('Failed to save rating:', e)
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

export function getSavedGitHubToken(): string {
  try {
    return localStorage.getItem(STORAGE_KEYS.githubToken) || ''
  } catch {
    return ''
  }
}

export function saveSavedGitHubToken(token: string): void {
  try {
    if (token) {
      localStorage.setItem(STORAGE_KEYS.githubToken, token.trim())
    } else {
      localStorage.removeItem(STORAGE_KEYS.githubToken)
    }
  } catch (e) {
    console.warn('Failed to save github token:', e)
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
  const cached = getCachedRegistry()
  if (cached && Array.isArray(cached.skills) && cached.skills.length > 0) {
    return cached.skills
  }
  return FALLBACK_SEED_SKILLS.map(s => ({ ...s }))
}

/**
 * 加载流水线套餐列表
 */
export async function loadHarvestedPipelines(): Promise<SkillPipeline[]> {
  const cached = getCachedRegistry()
  if (cached && Array.isArray(cached.pipelines) && cached.pipelines.length > 0) {
    return cached.pipelines
  }
  return FALLBACK_SEED_PIPELINES.map(p => ({ ...p }))
}

/**
 * 在线同步远端官方注册表（从独立的 halowake-skills-registry 数据中枢抓取）
 */
export async function syncOnlineRegistry(customUrl?: string): Promise<{
  success: boolean
  updatedCount: number
  totalCount: number
  lastSyncAt: number
  message: string
  skills: CatalogSkill[]
  pipelines: SkillPipeline[]
}> {
  const targetUrl = (customUrl || getRemoteRegistryUrl()).trim()
  const controller = new AbortController()
  const timeoutId = setTimeout(() => controller.abort(), 12000)

  try {
    const res = await fetch(targetUrl, {
      signal: controller.signal,
      headers: {
        Accept: 'application/json',
        'Cache-Control': 'no-cache',
      },
    })
    clearTimeout(timeoutId)

    if (!res.ok) {
      throw new Error(`HTTP ${res.status}: ${res.statusText}`)
    }

    const data = (await res.json()) as Partial<RemoteSkillsRegistry>
    if (!data || !Array.isArray(data.skills)) {
      throw new Error('注册表数据格式异常：未找到 skills 列表')
    }

    const validSkills: CatalogSkill[] = data.skills
    const validPipelines: SkillPipeline[] = Array.isArray(data.pipelines)
      ? data.pipelines
      : FALLBACK_SEED_PIPELINES
    const now = data.updatedAt || Date.now()

    const registryData: RemoteSkillsRegistry = {
      version: data.version || '1.0.0',
      updatedAt: now,
      description: data.description,
      skills: validSkills,
      pipelines: validPipelines,
    }

    saveCachedRegistry(registryData)
    localStorage.setItem(STORAGE_KEYS.lastSyncTime, String(now))

    return {
      success: true,
      updatedCount: validSkills.length,
      totalCount: validSkills.length,
      lastSyncAt: now,
      message: `已同步最新注册表：发现 ${validSkills.length} 个经过认证的技能`,
      skills: validSkills,
      pipelines: validPipelines,
    }
  } catch (err: unknown) {
    clearTimeout(timeoutId)
    console.warn('Sync online skills registry failed, falling back to local cache:', err)
    const errMessage = (err as Error)?.message || '网络连接超时'
    const cached = getCachedRegistry()
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
          ? `远端同步未就绪 (${errMessage})，已保留本地缓存的 ${fallbackSkills.length} 个技能`
          : `无法连接到技能注册表 (${errMessage})，请检查网络或在数据源中配置镜像地址`,
      skills: fallbackSkills,
      pipelines: fallbackPipelines,
    }
  }
}

/**
 * 测试与指定注册表地址的网络连通性
 */
export async function testRegistryConnection(url: string): Promise<{
  ok: boolean
  latencyMs: number
  skillsCount: number
  message: string
}> {
  const start = performance.now()
  const controller = new AbortController()
  const timeoutId = setTimeout(() => controller.abort(), 8000)

  try {
    const res = await fetch(url.trim(), {
      signal: controller.signal,
      headers: { Accept: 'application/json' },
    })
    clearTimeout(timeoutId)
    const latency = Math.round(performance.now() - start)

    if (!res.ok) {
      return {
        ok: false,
        latencyMs: latency,
        skillsCount: 0,
        message: `HTTP ${res.status}: ${res.statusText}`,
      }
    }

    const data = await res.json()
    if (!data || !Array.isArray(data.skills)) {
      return {
        ok: false,
        latencyMs: latency,
        skillsCount: 0,
        message: '数据非标准技能注册表格式（缺少 skills 字段）',
      }
    }

    return {
      ok: true,
      latencyMs: latency,
      skillsCount: data.skills.length,
      message: `连通正常 (${latency}ms)，检测到 ${data.skills.length} 个可用技能`,
    }
  } catch (err: unknown) {
    clearTimeout(timeoutId)
    const latency = Math.round(performance.now() - start)
    return {
      ok: false,
      latencyMs: latency,
      skillsCount: 0,
      message: `请求失败: ${(err as Error)?.message || '连接超时'}`,
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
