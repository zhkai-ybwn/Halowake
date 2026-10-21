import { defineStore } from 'pinia'
import { computed, ref, shallowRef, triggerRef, watch } from 'vue'
import {
  FALLBACK_SEED_PIPELINES,
  FALLBACK_SEED_SKILLS,
  getLastSyncTime,
  getLocalInstallPreference,
  getLocalStars,
  getUserCustomCategories,
  loadHarvestedCatalogSkills,
  loadHarvestedPipelines,
  readRegistryCache,
  saveAllUserCustomCategories,
  saveLocalInstallPreference,
  saveLocalStars,
  searchSkillRegistry,
  syncOnlineRegistry,
} from '@/services/skills/skill-catalog-service'
import {
  formatCatalogRevision,
  isRevisionMatching,
} from '@/services/skills/registry-cache'
import {
  buildCatalogSkillCandidate,
  buildSubSkillCandidate,
  getInstalledTargetsForCandidate,
  getSkillLookupKeys,
  type InstalledTargetsSummary,
  isCatalogSkillInstalledInTargets,
  isSubSkillInstalledInTargets,
  resolveStableCatalogKey,
  type SkillTargetFilter,
} from '@/services/skills/skill-identity'
import {
  installAgentSkill,
} from '@/services/skills/skills-service'
import type {
  AgentSkill,
  CatalogSkill,
  InstallationBatchResult,
  InstallationItemResult,
  InstallationItemStatus,
  InstallPreference,
  ExecuteInstallOptions,
  InstallSkillProgress,
  InstallTargetMode,
  InstallTargetOutcome,
  RegistrySyncResult,
  SdlcStage,
  SkillInstallHistoryEntry,
  SkillPipeline,
  SkillSearchMatchReason,
  SkillSearchMatchTier,
  SubSkillItem,
} from '@/types/skills'
import { skillCategoryLabel } from '@/utils/skill-category-label'
import { expandSearchTerms } from '@/utils/skill-synonyms'

const SEARCH_DEBOUNCE_MS = 150

export function generateOperationId(): string {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return `op-${crypto.randomUUID()}`
  }
  return `op-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`
}

export const useSkillExploreStore = defineStore('skillExplore', () => {
  // 顶层主标签：已安装列表 vs 技能探索与流水线
  const activeMainTab = ref<'installed' | 'explore' | 'history'>('installed')

  // 本地星标与评分缓存
  const starredIds = ref<Set<string>>(getLocalStars())
  const userCustomCategories = ref<Record<string, string[]>>(getUserCustomCategories())
  const lastSyncTime = ref<number | null>(getLastSyncTime())
  const isSyncing = ref(false)
  const syncStatusMessage = ref('')

  function isSkillStarred(
    skill:
      | string
      | {
          id: string
          catalogId?: string
          canonicalName?: string
          name?: string
        }
  ): boolean {
    if (starredIds.value.size === 0) return false
    const keys = getSkillLookupKeys(skill)
    return keys.some(key => starredIds.value.has(key))
  }

  function collectMatchingCategoryKeys(
    aliases: string[],
    allCategories: Record<string, string[]>
  ): string[] {
    const lowerSet = new Set(aliases.map(a => a.toLowerCase()))
    const matched = new Set<string>()
    for (const existingKey of Object.keys(allCategories)) {
      const lowerKey = existingKey.toLowerCase()
      if (lowerSet.has(lowerKey)) {
        matched.add(existingKey)
        continue
      }
      const colonIdx = lowerKey.indexOf(':')
      if (colonIdx >= 0 && lowerSet.has(lowerKey.slice(colonIdx + 1))) {
        matched.add(existingKey)
      }
    }
    for (const alias of aliases) {
      matched.add(alias)
    }
    return Array.from(matched)
  }

  function getSkillCustomCategories(
    skill:
      | string
      | {
          id: string
          catalogId?: string
          canonicalName?: string
          name?: string
        }
  ): string[] {
    const cats = userCustomCategories.value
    if (!cats || Object.keys(cats).length === 0) {
      return []
    }
    const aliases = getSkillLookupKeys(skill, catalogSkills.value)
    const keys = collectMatchingCategoryKeys(aliases, cats)
    const merged: string[] = []
    for (const key of keys) {
      const list = cats[key]
      if (Array.isArray(list)) {
        for (const tag of list) {
          if (tag && !merged.includes(tag)) {
            merged.push(tag)
          }
        }
      }
    }
    return merged
  }

  // 基础数据与数据源
  function decorateSkills(skills: CatalogSkill[]): CatalogSkill[] {
    return skills.map(s => {
      const clone = { ...s }
      clone.isStar = isSkillStarred(s)
      return clone
    })
  }

  const catalogSkills = shallowRef<CatalogSkill[]>(
    decorateSkills(FALLBACK_SEED_SKILLS.map(skill => ({ ...skill })))
  )
  const pipelines = ref<SkillPipeline[]>(FALLBACK_SEED_PIPELINES)

  // 探索页面的过滤与搜索
  const searchQuery = ref('')
  const selectedStage = ref<SdlcStage | 'all' | 'starred' | string>('all')
  const selectedCategory = ref<string>('all')
  const activeSort = ref<'relevance' | 'popular' | 'name'>('relevance')

  // v3.2 FTS5 搜索状态、命中层级（exact / general_fallback / none）、回退提示与单技能匹配依据
  const isSearching = ref(false)
  const searchMatchTier = ref<SkillSearchMatchTier>('browse')
  const searchFallbackNotice = ref<string | null>(null)
  const usedFallbackEngine = ref(false)
  const searchElapsedMs = ref(0)
  const totalMatched = ref(0)
  const hasMore = ref(false)
  const isTruncated = ref(false)
  const ftsSearchSkills = ref<CatalogSkill[] | null>(null)
  const skillMatchReasons = ref<Record<string, SkillSearchMatchReason>>({})
  const skillSearchScores = ref<Record<string, number>>({})

  let searchRequestSeq = 0
  let searchDebounceTimer: ReturnType<typeof setTimeout> | null = null
  let installOperationSeq = 0

  const isLoadingMore = ref(false)
  const loadMoreError = ref<string | null>(null)
  let loadMoreRequestSeq = 0

  // 目录版本序号（仅在 SQLite / 缓存落盘成功后再递增并发布）
  const catalogRevision = ref(0)
  const snapshotVersion = ref<string | null>('1.4.4')

  // 安装向导状态
  const installModalVisible = ref(false)
  const pendingSkill = ref<CatalogSkill | null>(null)
  const pendingSubSkill = ref<SubSkillItem | null>(null)
  const selectedBundleSubSkillIds = ref<string[]>([])
  const pendingPipeline = ref<SkillPipeline | null>(null)
  const installingIds = ref<string[]>([])
  const installProgress = ref<InstallSkillProgress | null>(null)
  const installError = ref('')
  const installCurrent = ref(0)
  const installTotal = ref(0)
  const installPreference = ref<InstallPreference>(getLocalInstallPreference())
  const reinstallPreferenceOverride = ref<InstallPreference | null>(null)
  const lastInstallBatchResult = ref<InstallationBatchResult | null>(null)

  // 详情抽屉状态 (支持单技能指引 OR 流水线全流程玩法指引)
  const detailDrawerVisible = ref(false)
  const activeDetailSkill = ref<CatalogSkill | null>(null)
  const activeDetailPipeline = ref<SkillPipeline | null>(null)

  const starredCount = computed(() => starredIds.value.size)

  const allCustomCategories = computed(() => {
    const set = new Set<string>()
    Object.values(userCustomCategories.value).forEach(list => list.forEach(item => set.add(item)))
    return Array.from(set)
  })

  // 构建截断前生效的阶段与自定义集合过滤参数
  function resolveBackendStageAndAllowedIds(): {
    stage: string | null
    allowedIds: string[] | null
  } {
    const currentStage = selectedStage.value
    if (currentStage === 'all') {
      return { stage: null, allowedIds: null }
    }
    if (currentStage === 'starred') {
      const allowed = new Set<string>(starredIds.value)
      for (const s of catalogSkills.value) {
        if (isSkillStarred(s)) allowed.add(s.id)
      }
      return { stage: null, allowedIds: Array.from(allowed) }
    }
    if (['requirement', 'design', 'coding', 'testing', 'delivery'].includes(currentStage)) {
      return { stage: currentStage, allowedIds: null }
    }
    const matchingCustomIds = new Set<string>()
    for (const [id, tags] of Object.entries(userCustomCategories.value)) {
      if (tags.includes(currentStage)) {
        matchingCustomIds.add(resolveStableCatalogKey(id, catalogSkills.value))
      }
    }
    return { stage: null, allowedIds: Array.from(matchingCustomIds) }
  }

  function resolveBackendCategory(): string | null {
    const cat = selectedCategory.value.trim()
    return cat && cat !== 'all' ? cat : null
  }

  // 保留现有前端搜索作为回退实现（当后端搜索异常或纯前端环境运行时无缝恢复）
  function runLegacyLocalSearch(): CatalogSkill[] {
    let result = catalogSkills.value

    if (selectedCategory.value !== 'all') {
      result = result.filter(s => s.category === selectedCategory.value)
    }

    if (selectedStage.value === 'starred') {
      result = result.filter(s => isSkillStarred(s))
    } else if (['requirement', 'design', 'coding', 'testing', 'delivery'].includes(selectedStage.value)) {
      result = result.filter(s => s.stage === selectedStage.value)
    } else if (selectedStage.value !== 'all') {
      result = result.filter(s => {
        const customTags = getSkillCustomCategories(s)
        return customTags.includes(selectedStage.value)
      })
    }

    const expandedGroups = expandSearchTerms(searchQuery.value)
    if (expandedGroups.length) {
      result = result.filter(s => {
        const customTags = getSkillCustomCategories(s)
        const searchable = [
          s.name,
          s.titleZh,
          s.description,
          s.category,
          skillCategoryLabel(s.category),
          s.author.name,
          s.gitUrl,
          s.subPath || '',
          s.usageGuide?.summary || '',
          s.usageGuide?.trigger || '',
          ...(s.usageGuide?.steps || []),
          ...s.tags,
          ...customTags,
        ]
          .join(' ')
          .toLowerCase()
        return expandedGroups.every(group => group.some(term => searchable.includes(term)))
      })
    }

    totalMatched.value = result.length
    hasMore.value = false
    isTruncated.value = false
    return result
  }

  // 执行后端 v3.2 FTS5 + 字段权威度搜索（带序列号竞态保护与异常回退）
  async function executeBackendSearch(seq: number) {
    const query = searchQuery.value.trim()
    if (!query) {
      if (seq === searchRequestSeq) {
        isSearching.value = false
        ftsSearchSkills.value = null
        searchMatchTier.value = 'browse'
        searchFallbackNotice.value = null
        usedFallbackEngine.value = false
        skillMatchReasons.value = {}
        skillSearchScores.value = {}
        totalMatched.value = 0
        hasMore.value = false
        isTruncated.value = false
      }
      return
    }

    const { stage, allowedIds } = resolveBackendStageAndAllowedIds()
    const category = resolveBackendCategory()
    try {
      const activeRevision = snapshotVersion.value || (catalogRevision.value ? String(catalogRevision.value) : null)
      const response = await searchSkillRegistry({
        query,
        stage,
        category,
        allowedIds,
        projectTechStack: [],
        sort: activeSort.value,
        catalogRevision: activeRevision,
        limit: 500,
      })

      // 丢弃过期请求的乱序响应，严格保证结果与最新输入一致
      if (seq !== searchRequestSeq) {
        return
      }

      // 如果响应包含 catalogRevision 且与本地期望版本不一致，丢弃该陈旧响应并根据重试规则恢复
      if (
        response.catalogRevision &&
        activeRevision &&
        !isRevisionMatching(response.catalogRevision, activeRevision, catalogRevision.value)
      ) {
        console.warn(`Catalog revision mismatch: expected ${activeRevision}, got ${response.catalogRevision}`)
        // 清理旧搜索结果，防止陈旧数据泄漏
        ftsSearchSkills.value = null
        if (seq === searchRequestSeq) {
          isSearching.value = false
          searchFallbackNotice.value = '技能目录已在后台更新，请刷新重试'
        }
        return
      }

      if (response.catalogRevision) {
        snapshotVersion.value = response.catalogRevision
      }

      const reasons: Record<string, SkillSearchMatchReason> = {}
      const scores: Record<string, number> = {}
      const skills: CatalogSkill[] = []

      for (const hit of response.hits) {
        reasons[hit.id] = hit.matchReason
        scores[hit.id] = hit.score
        skills.push(hit.skill)
      }

      skillMatchReasons.value = reasons
      skillSearchScores.value = scores
      ftsSearchSkills.value = decorateSkills(skills)
      searchMatchTier.value = response.matchTier
      searchFallbackNotice.value = response.fallbackNotice ?? null
      usedFallbackEngine.value = response.usedFallbackEngine
      searchElapsedMs.value = response.elapsedMs
      totalMatched.value = response.totalMatched
      hasMore.value = response.hasMore ?? false
      isTruncated.value = response.isTruncated ?? false
      if (seq === searchRequestSeq) {
        isSearching.value = false
      }
    } catch (error) {
      if (seq !== searchRequestSeq) {
        return
      }
      console.warn('v3.2 FTS5 search failed, falling back to local search:', error)
      ftsSearchSkills.value = null
      usedFallbackEngine.value = true
      const fallbackHits = runLegacyLocalSearch()
      totalMatched.value = fallbackHits.length
      hasMore.value = false
      isTruncated.value = false
      searchMatchTier.value = fallbackHits.length > 0 ? 'general_fallback' : 'none'
      searchFallbackNotice.value =
        fallbackHits.length > 0 ? '后端索引暂不可用，已自动切换至本地回退搜索' : null
      skillMatchReasons.value = {}
      skillSearchScores.value = {}
      if (seq === searchRequestSeq) {
        isSearching.value = false
      }
    }
  }

  let pendingTargetCount: number | null = null
  let activeLoadMorePromise: Promise<boolean> | null = null

  async function loadMoreSearchResults(targetCount?: number): Promise<boolean> {
    if (!hasMore.value) return false
    const query = searchQuery.value.trim()
    if (!query) return false

    if (targetCount !== undefined) {
      pendingTargetCount = targetCount
    }

    if (isLoadingMore.value && activeLoadMorePromise) {
      return activeLoadMorePromise
    }

    const seq = ++loadMoreRequestSeq
    const searchSeq = searchRequestSeq
    isLoadingMore.value = true
    loadMoreError.value = null

    const run = async (): Promise<boolean> => {
      try {
        while (hasMore.value) {
          if (seq !== loadMoreRequestSeq || searchSeq !== searchRequestSeq) {
            return false
          }
          const currentHits = ftsSearchSkills.value || []
          const offset = currentHits.length
          if (totalMatched.value > 0 && offset >= totalMatched.value) {
            hasMore.value = false
            break
          }
          const effectiveTarget = pendingTargetCount ?? targetCount
          if (effectiveTarget !== undefined && offset >= effectiveTarget) {
            break
          }

          const remainingToTarget =
            effectiveTarget !== undefined ? effectiveTarget - offset : 500
          const limit = Math.max(24, Math.min(remainingToTarget, 2000))

          const { stage, allowedIds } = resolveBackendStageAndAllowedIds()
          const category = resolveBackendCategory()
          const activeRevision =
            snapshotVersion.value || (catalogRevision.value ? String(catalogRevision.value) : null)

          const response = await searchSkillRegistry({
            query,
            stage,
            category,
            allowedIds,
            projectTechStack: [],
            sort: activeSort.value,
            catalogRevision: activeRevision,
            offset,
            limit,
          })

          if (seq !== loadMoreRequestSeq || searchSeq !== searchRequestSeq) {
            return false
          }

          // 校验版本是否兼容匹配，不能将不同代际数据追加到同一集合
          if (
            response.catalogRevision &&
            activeRevision &&
            !isRevisionMatching(response.catalogRevision, activeRevision, catalogRevision.value)
          ) {
            console.warn('loadMoreSearchResults: revision changed during pagination, aborting append')
            hasMore.value = false
            return false
          }

          if (response.hits.length === 0) {
            hasMore.value = false
            break
          }

          const reasons = { ...skillMatchReasons.value }
          const scores = { ...skillSearchScores.value }
          const newSkills: CatalogSkill[] = []
          const existingIdSet = new Set((ftsSearchSkills.value || []).map(s => s.id))

          for (const hit of response.hits) {
            if (!existingIdSet.has(hit.id)) {
              existingIdSet.add(hit.id)
              reasons[hit.id] = hit.matchReason
              scores[hit.id] = hit.score
              newSkills.push(hit.skill)
            }
          }

          skillMatchReasons.value = reasons
          skillSearchScores.value = scores
          ftsSearchSkills.value = [...(ftsSearchSkills.value || []), ...decorateSkills(newSkills)]
          totalMatched.value = response.totalMatched
          hasMore.value = response.hasMore ?? false
          isTruncated.value = response.isTruncated ?? false

          const currentTarget = pendingTargetCount ?? targetCount
          if (currentTarget === undefined || (ftsSearchSkills.value?.length || 0) >= currentTarget) {
            break
          }
        }
        return true
      } catch (error) {
        if (seq === loadMoreRequestSeq && searchSeq === searchRequestSeq) {
          loadMoreError.value = error instanceof Error ? error.message : String(error)
          console.warn('loadMoreSearchResults failed:', error)
        }
        return false
      } finally {
        if (seq === loadMoreRequestSeq && searchSeq === searchRequestSeq) {
          isLoadingMore.value = false
          pendingTargetCount = null
          activeLoadMorePromise = null
        }
      }
    }

    activeLoadMorePromise = run()
    return activeLoadMorePromise
  }

  function scheduleSearch(immediate = false) {
    if (searchDebounceTimer !== null) {
      clearTimeout(searchDebounceTimer)
      searchDebounceTimer = null
    }

    loadMoreRequestSeq++
    pendingTargetCount = null
    activeLoadMorePromise = null
    isLoadingMore.value = false
    loadMoreError.value = null

    const seq = ++searchRequestSeq
    const query = searchQuery.value.trim()

    if (!query) {
      isSearching.value = false
      ftsSearchSkills.value = null
      searchMatchTier.value = 'browse'
      searchFallbackNotice.value = null
      usedFallbackEngine.value = false
      skillMatchReasons.value = {}
      skillSearchScores.value = {}
      totalMatched.value = 0
      hasMore.value = false
      isTruncated.value = false
      return
    }

    isSearching.value = true
    if (immediate) {
      void executeBackendSearch(seq)
    } else {
      searchDebounceTimer = setTimeout(() => {
        searchDebounceTimer = null
        void executeBackendSearch(seq)
      }, SEARCH_DEBOUNCE_MS)
    }
  }

  // 监听搜索词变化（150ms 防抖 + 取消旧请求）
  watch(searchQuery, () => {
    scheduleSearch(false)
  })

  // 监听阶段/分类/排序切换：若处于搜索态则立即触发重新检索（截断前过滤）
  watch([selectedStage, selectedCategory, activeSort], () => {
    loadMoreError.value = null
    if (searchQuery.value.trim()) {
      scheduleSearch(true)
    }
  })

  function clearDiscoveryFilters() {
    searchQuery.value = ''
    selectedCategory.value = 'all'
    selectedStage.value = 'all'
    activeSort.value = 'relevance'
    loadMoreError.value = null
  }

  // 异步初始化：从本地缓存或种子读取技能，并在必要时从远端注册表更新
  async function initCatalogAndSources() {
    try {
      const cachedRegistry = await readRegistryCache()
      if (cachedRegistry) {
        if (cachedRegistry.skills && cachedRegistry.skills.length > 0) {
          catalogSkills.value = decorateSkills(cachedRegistry.skills)
        }
        if (cachedRegistry.pipelines && cachedRegistry.pipelines.length > 0) {
          pipelines.value = cachedRegistry.pipelines
        }
        const rev = formatCatalogRevision(
          cachedRegistry.version,
          cachedRegistry.updatedAt,
          cachedRegistry.catalogRevision
        )
        if (rev) {
          snapshotVersion.value = rev
        }
      } else {
        const cached = await loadHarvestedCatalogSkills()
        const cachedPipelines = await loadHarvestedPipelines()
        if (cached && cached.length > 0) {
          catalogSkills.value = decorateSkills(cached)
        }
        if (cachedPipelines && cachedPipelines.length > 0) {
          pipelines.value = cachedPipelines
        }
      }
      if (searchQuery.value.trim()) {
        scheduleSearch(true)
      }
      // 如果没有上次同步时间或已过期超过 6 小时，后台自动触发一次同步
      if (!lastSyncTime.value || Date.now() - lastSyncTime.value > 1000 * 60 * 60 * 6) {
        void syncRegistry()
      }
    } catch (e) {
      console.warn('init catalog error:', e)
    }
  }

  // 立即触发初始化
  initCatalogAndSources()

  const filteredSkills = computed(() => {
    // 若存在活跃搜索词且后端 v3.2 检索引擎已返回结果，直接使用已按截断前过滤与字段权威度排序的结果
    if (searchQuery.value.trim() && ftsSearchSkills.value !== null) {
      return ftsSearchSkills.value
    }
    return runLegacyLocalSearch()
  })

  function getSkillMatchReason(skillId: string): SkillSearchMatchReason | undefined {
    if (!searchQuery.value.trim()) return undefined
    return skillMatchReasons.value[skillId]
  }

  function getSkillSearchScore(skillId: string): number | undefined {
    if (!searchQuery.value.trim()) return undefined
    return skillSearchScores.value[skillId]
  }

  // 按 SDLC 阶段统计可用技能数
  const stageCounts = computed<Record<string, number>>(() => {
    const counts: Record<string, number> = { all: catalogSkills.value.length, starred: starredCount.value }
    catalogSkills.value.forEach(s => {
      counts[s.stage] = (counts[s.stage] || 0) + 1
    })
    allCustomCategories.value.forEach(cat => {
      let c = 0
      for (const s of catalogSkills.value) {
        if (getSkillCustomCategories(s).includes(cat)) {
          c++
        }
      }
      counts[cat] = c
    })
    return counts
  })

  function toggleStar(skillId: string) {
    const stableKey = resolveStableCatalogKey(skillId, catalogSkills.value)
    const aliases = getSkillLookupKeys(skillId, catalogSkills.value)
    const wasStarred = aliases.some(key => starredIds.value.has(key))

    for (const key of aliases) {
      starredIds.value.delete(key)
    }
    if (!wasStarred) {
      starredIds.value.add(stableKey)
    }

    saveLocalStars(starredIds.value)
    const isNowStar = !wasStarred
    const target = catalogSkills.value.find(s => s.id === stableKey || aliases.includes(s.id))
    if (target) {
      target.isStar = isNowStar
    }
    if (ftsSearchSkills.value) {
      const ftsTarget = ftsSearchSkills.value.find(s => s.id === stableKey || aliases.includes(s.id))
      if (ftsTarget) {
        ftsTarget.isStar = isNowStar
      }
    }
    triggerRef(catalogSkills)
    if (selectedStage.value === 'starred' && searchQuery.value.trim()) {
      scheduleSearch(true)
    }
  }

  function addSkillCustomCategory(skillId: string, category: string) {
    const clean = category.trim()
    if (!clean) return
    const stableKey = resolveStableCatalogKey(skillId, catalogSkills.value)
    const all = { ...getUserCustomCategories(), ...userCustomCategories.value }
    const aliases = collectMatchingCategoryKeys(
      getSkillLookupKeys(skillId, catalogSkills.value),
      all
    )

    const merged: string[] = []
    for (const key of [stableKey, ...aliases]) {
      const list = all[key]
      if (Array.isArray(list)) {
        for (const t of list) {
          if (t && !merged.includes(t)) {
            merged.push(t)
          }
        }
      }
      if (key !== stableKey) {
        delete all[key]
      }
    }
    if (!merged.includes(clean)) {
      merged.push(clean)
    }
    all[stableKey] = merged
    saveAllUserCustomCategories(all)
    userCustomCategories.value = all
    if (searchQuery.value.trim()) {
      scheduleSearch(true)
    }
  }

  function removeSkillCustomCategory(skillId: string, category: string) {
    const stableKey = resolveStableCatalogKey(skillId, catalogSkills.value)
    const all = { ...getUserCustomCategories(), ...userCustomCategories.value }
    const aliases = collectMatchingCategoryKeys(
      getSkillLookupKeys(skillId, catalogSkills.value),
      all
    )

    const merged: string[] = []
    for (const key of [stableKey, ...aliases]) {
      const list = all[key]
      if (Array.isArray(list)) {
        for (const t of list) {
          if (t && t !== category && !merged.includes(t)) {
            merged.push(t)
          }
        }
      }
      if (key !== stableKey) {
        delete all[key]
      }
    }
    if (merged.length > 0) {
      all[stableKey] = merged
    } else {
      delete all[stableKey]
    }
    saveAllUserCustomCategories(all)
    userCustomCategories.value = all
    if (searchQuery.value.trim()) {
      scheduleSearch(true)
    }
  }

  function cloneInstallPreference(pref: InstallPreference): InstallPreference {
    return {
      ...pref,
      targetAgents: Array.isArray(pref.targetAgents) ? [...pref.targetAgents] : ['codex'],
    }
  }

  function openInstallDialog(skill: CatalogSkill) {
    installError.value = ''
    installProgress.value = null
    lastInstallBatchResult.value = null
    reinstallPreferenceOverride.value = null
    pendingSkill.value = skill
    pendingSubSkill.value = null
    pendingPipeline.value = null
    if (skill.isBundle && skill.subSkills) {
      selectedBundleSubSkillIds.value = skill.subSkills.map(s => s.id)
    } else {
      selectedBundleSubSkillIds.value = []
    }
    installModalVisible.value = true
  }

  function openSubSkillInstallDialog(parentSkill: CatalogSkill, subSkill: SubSkillItem) {
    installError.value = ''
    installProgress.value = null
    lastInstallBatchResult.value = null
    reinstallPreferenceOverride.value = null
    pendingSkill.value = parentSkill
    pendingSubSkill.value = subSkill
    pendingPipeline.value = null
    selectedBundleSubSkillIds.value = [subSkill.id]
    installModalVisible.value = true
  }

  function openPipelineInstallDialog(pipeline: SkillPipeline) {
    installError.value = ''
    installProgress.value = null
    lastInstallBatchResult.value = null
    reinstallPreferenceOverride.value = null
    pendingPipeline.value = pipeline
    pendingSkill.value = null
    pendingSubSkill.value = null
    selectedBundleSubSkillIds.value = []
    installModalVisible.value = true
  }

  function openReinstallFromHistory(entry: SkillInstallHistoryEntry) {
    installError.value = ''
    installProgress.value = null
    lastInstallBatchResult.value = null

    const mode: InstallTargetMode =
      entry.targetMode === 'project' || entry.targetMode === 'specific_agent' || entry.targetMode === 'global'
        ? entry.targetMode
        : 'global'

    reinstallPreferenceOverride.value = {
      targetMode: mode,
      targetAgents: entry.targetAgents?.length ? [...entry.targetAgents] : ['codex'],
      customProjectPath: entry.projectCwd || undefined,
      addToGitignore: false,
      remember: false,
    }

    const matched = resolveCatalogSkill(entry.skillId)
    const fallbackSkill: CatalogSkill = matched || {
      id: entry.skillId,
      name: entry.displayName || entry.skillId,
      titleZh: entry.displayName || entry.skillId,
      description: entry.message || entry.displayName,
      category: 'general',
      stage: 'coding',
      author: { name: 'Git' },
      gitUrl: entry.gitUrl,
      subPath: entry.subPath || undefined,
      tags: [],
      stars: 0,
      halowakeScore: 0,
      badge: 'community',
      metrics: {
        githubStars: 0,
        forks: 0,
        lastCommitDaysAgo: 0,
        issueCloseRate: 0,
        securityAuditPassed: false,
      },
      recommendedWith: [],
      compatibleAgents: entry.targetAgents?.length ? [...entry.targetAgents] : ['codex'],
      howToUse: entry.gitUrl,
      isBundle: false,
    }

    pendingPipeline.value = null
    pendingSkill.value = fallbackSkill
    if (entry.subSkillId && fallbackSkill.subSkills?.length) {
      const sub = fallbackSkill.subSkills.find(
        s => s.id.toLowerCase() === entry.subSkillId!.toLowerCase()
      )
      if (sub) {
        pendingSubSkill.value = sub
        selectedBundleSubSkillIds.value = [sub.id]
      } else {
        pendingSubSkill.value = {
          id: entry.subSkillId,
          name: entry.displayName || entry.subSkillId,
          titleZh: entry.displayName || entry.subSkillId,
          description: '',
          subPath: entry.subPath || '',
        }
        selectedBundleSubSkillIds.value = [entry.subSkillId]
      }
    } else if (entry.subSkillId) {
      pendingSubSkill.value = {
        id: entry.subSkillId,
        name: entry.displayName || entry.subSkillId,
        titleZh: entry.displayName || entry.subSkillId,
        description: '',
        subPath: entry.subPath || '',
      }
      selectedBundleSubSkillIds.value = [entry.subSkillId]
    } else {
      pendingSubSkill.value = null
      selectedBundleSubSkillIds.value = fallbackSkill.isBundle && fallbackSkill.subSkills
        ? fallbackSkill.subSkills.map(s => s.id)
        : []
    }
    installModalVisible.value = true
  }

  function closeInstallDialog() {
    if (installingIds.value.length > 0) {
      return
    }
    installOperationSeq++
    installModalVisible.value = false
    pendingSkill.value = null
    pendingSubSkill.value = null
    pendingPipeline.value = null
    selectedBundleSubSkillIds.value = []
    installProgress.value = null
    installError.value = ''
    lastInstallBatchResult.value = null
    reinstallPreferenceOverride.value = null
  }

  function openDetail(skill: CatalogSkill) {
    activeDetailSkill.value = skill
    activeDetailPipeline.value = null
    detailDrawerVisible.value = true
  }

  function openPipelinePlaybook(pipeline: SkillPipeline) {
    activeDetailPipeline.value = pipeline
    activeDetailSkill.value = null
    detailDrawerVisible.value = true
  }

  function closeDetail() {
    detailDrawerVisible.value = false
    activeDetailSkill.value = null
    activeDetailPipeline.value = null
  }

  function updateInstallPreference(pref: InstallPreference, persist = pref.remember) {
    const cloned = cloneInstallPreference(pref)
    installPreference.value = cloned
    if (persist) {
      saveLocalInstallPreference(cloned)
    }
  }

  async function syncRegistry(): Promise<RegistrySyncResult> {
    isSyncing.value = true
    try {
      const res = await syncOnlineRegistry()
      syncStatusMessage.value = res.message

      if (res.status === 'synced') {
        lastSyncTime.value = res.lastSyncAt
        if (res.skills && res.skills.length > 0) {
          catalogSkills.value = decorateSkills(res.skills)
        }
        pipelines.value = res.pipelines
        if (res.version) {
          const rev = formatCatalogRevision(res.version, res.lastSyncAt, res.catalogRevision)
          if (rev) {
            snapshotVersion.value = rev
          }
        }
        catalogRevision.value++
      } else if (res.status === 'cached_fallback' && res.skills && res.skills.length > 0) {
        catalogSkills.value = decorateSkills(res.skills)
        pipelines.value = res.pipelines
      }

      if (searchQuery.value.trim()) {
        scheduleSearch(true)
      }

      return {
        status: res.status,
        success: res.success,
        updatedCount: res.updatedCount,
        totalCount: res.totalCount,
        message: res.message,
        lastSyncAt: res.lastSyncAt,
        catalogRevision: String(catalogRevision.value),
        skills: res.skills,
        pipelines: res.pipelines,
      }
    } finally {
      isSyncing.value = false
    }
  }

  interface PlannedInstallItem {
    key: string
    skillId: string
    subSkillId?: string
    displayName: string
    gitUrl: string
    subPath?: string
  }

  async function executeInstall(
    projectCwdOrOptions?: string | ExecuteInstallOptions
  ): Promise<InstallationBatchResult> {
    const options: ExecuteInstallOptions =
      typeof projectCwdOrOptions === 'string'
        ? { projectCwd: projectCwdOrOptions }
        : projectCwdOrOptions || {}

    if (options.preference) {
      const shouldPersist = options.persistPreference ?? options.preference.remember
      if (shouldPersist) {
        updateInstallPreference(options.preference, true)
      }
    }

    const pref = cloneInstallPreference(options.preference ?? installPreference.value)
    const effectiveProjectCwd = (pref.customProjectPath || options.projectCwd || '').trim() || undefined

    const requestSeq = ++installOperationSeq
    const operationId = generateOperationId()
    const retryOfOperationId =
      options.onlyFailed && lastInstallBatchResult.value
        ? lastInstallBatchResult.value.operationId
        : undefined

    installError.value = ''
    installProgress.value = { operationId, phase: 'preparing', message: '正在准备安装计划…', percent: null }
    const reportProgress = (progress: InstallSkillProgress) => {
      if (requestSeq === installOperationSeq) {
        installProgress.value = {
          ...progress,
          operationId: progress.operationId || operationId,
        }
      }
    }

    const failBatch = (message: string, items: InstallationItemResult[] = []): InstallationBatchResult => {
      const missingCount = items.filter(i => i.status === 'missing').length
      const failedCount = items.filter(i => i.status === 'failed' || i.status === 'partial').length
      const batch: InstallationBatchResult = {
        operationId,
        retryOfOperationId,
        status: 'failed',
        success: false,
        message,
        count: 0,
        installedCount: 0,
        skippedCount: 0,
        failedCount,
        missingCount,
        installedPaths: [],
        skippedPaths: [],
        items,
      }
      if (requestSeq === installOperationSeq) {
        installError.value = message
        installProgress.value = { operationId, phase: 'failed', message: '安装未完成', percent: null }
        lastInstallBatchResult.value = batch
      }
      return batch
    }

    if (pref.targetMode === 'project' && !effectiveProjectCwd) {
      return failBatch('未指定有效的项目目录，无法安装到项目内')
    }

    // 1. 构建完整安装计划（并在流水线存在缺失目录项时直接阻断）
    let plannedItems: PlannedInstallItem[] = []
    const preMissingItems: InstallationItemResult[] = []

    if (pendingSubSkill.value && pendingSkill.value) {
      const parent = pendingSkill.value
      const sub = pendingSubSkill.value
      plannedItems = [
        {
          key: sub.id,
          skillId: parent.id,
          subSkillId: sub.id,
          displayName: sub.titleZh || sub.name,
          gitUrl: parent.gitUrl,
          subPath: sub.subPath,
        },
      ]
    } else if (
      pendingSkill.value?.isBundle &&
      pendingSkill.value.subSkills &&
      selectedBundleSubSkillIds.value.length > 0
    ) {
      const parent = pendingSkill.value
      const parentSubSkills = parent.subSkills || []
      for (const subId of selectedBundleSubSkillIds.value) {
        const sub = parentSubSkills.find(s => s.id === subId)
        if (!sub) {
          preMissingItems.push({
            itemKey: `${parent.id}:${subId}`,
            skillId: parent.id,
            subSkillId: subId,
            displayName: subId,
            gitUrl: parent.gitUrl,
            status: 'missing',
            message: `合集中未找到子技能「${subId}」`,
            error: `合集中未找到子技能「${subId}」`,
            installedPaths: [],
            skippedPaths: [],
          })
        } else {
          plannedItems.push({
            key: sub.id,
            skillId: parent.id,
            subSkillId: sub.id,
            displayName: sub.titleZh || sub.name,
            gitUrl: parent.gitUrl,
            subPath: sub.subPath,
          })
        }
      }
    } else if (pendingPipeline.value) {
      const p = pendingPipeline.value
      for (const reqSkillId of p.skills) {
        const found = resolveCatalogSkill(reqSkillId)
        if (!found) {
          preMissingItems.push({
            itemKey: reqSkillId,
            skillId: reqSkillId,
            displayName: reqSkillId,
            gitUrl: '',
            status: 'missing',
            message: `目录中未找到流水线依赖技能「${reqSkillId}」`,
            error: `目录中未找到流水线依赖技能「${reqSkillId}」`,
            installedPaths: [],
            skippedPaths: [],
          })
        } else {
          plannedItems.push({
            key: found.id,
            skillId: found.id,
            displayName: found.titleZh || found.name,
            gitUrl: found.gitUrl,
            subPath: found.subPath,
          })
        }
      }
      if (preMissingItems.length > 0) {
        const missingIds = preMissingItems.map(i => i.skillId).join(', ')
        return failBatch(`流水线包含目录中未找到的依赖技能：${missingIds}`, preMissingItems)
      }
    } else if (pendingSkill.value) {
      const skill = pendingSkill.value
      plannedItems = [
        {
          key: skill.id,
          skillId: skill.id,
          displayName: skill.titleZh || skill.name,
          gitUrl: skill.gitUrl,
          subPath: skill.subPath,
        },
      ]
    }

    if (plannedItems.length === 0 && preMissingItems.length === 0) {
      return failBatch('未选择待安装技能')
    }

    let preservedItems: InstallationItemResult[] = []
    if (options.onlyFailed && lastInstallBatchResult.value) {
      const prevFailedKeys = new Set(
        lastInstallBatchResult.value.items
          .filter(i => i.status === 'failed' || i.status === 'partial')
          .map(i => i.itemKey || i.subSkillId || i.skillId)
      )
      preservedItems = lastInstallBatchResult.value.items.filter(
        i => i.status === 'installed' || i.status === 'skipped'
      )
      if (prevFailedKeys.size > 0) {
        plannedItems = plannedItems.filter(item => prevFailedKeys.has(item.key))
      }
    }

    const items: InstallationItemResult[] = [...preservedItems, ...preMissingItems]
    installTotal.value = plannedItems.length

    for (const [index, item] of plannedItems.entries()) {
      installCurrent.value = index + 1
      if (installingIds.value.includes(item.key)) continue
      installingIds.value.push(item.key)

      let targetAgentsToInstall = [...pref.targetAgents]
      let prevOutcomes: InstallTargetOutcome[] = []
      let prevInstalledPaths: string[] = []
      let prevSkippedPaths: string[] = []

      if (options.onlyFailed && lastInstallBatchResult.value) {
        const prevItem = lastInstallBatchResult.value.items.find(
          pi => (pi.subSkillId || pi.skillId) === item.key || pi.itemKey === item.key
        )
        if (prevItem && prevItem.targetOutcomes && prevItem.targetOutcomes.length > 0) {
          const failedAgents = prevItem.targetOutcomes
            .filter(to => to.status === 'failed')
            .map(to => to.targetAgent)
          if (failedAgents.length > 0) {
            targetAgentsToInstall = failedAgents
            prevOutcomes = prevItem.targetOutcomes.filter(to => to.status !== 'failed')
            prevInstalledPaths = prevItem.installedPaths || []
            prevSkippedPaths = prevItem.skippedPaths || []
          }
        }
      }

      try {
        const res = await installAgentSkill(
          {
            skillId: item.skillId,
            gitUrl: item.gitUrl,
            subPath: item.subPath,
            subSkillId: item.subSkillId,
            targetMode: pref.targetMode,
            targetAgents: targetAgentsToInstall,
            projectCwd: effectiveProjectCwd,
            addToGitignore: pref.addToGitignore,
            operationId,
            itemKey: item.key,
            retryOfOperationId,
          },
          reportProgress
        )

        const rawInstalledPaths = Array.isArray(res.installedPaths) ? res.installedPaths : []
        const rawSkippedPaths = Array.isArray(res.skippedPaths) ? res.skippedPaths : []
        const rawOutcomes = Array.isArray(res.targetOutcomes) ? res.targetOutcomes : []

        const itemInstalledPaths = Array.from(new Set([...prevInstalledPaths, ...rawInstalledPaths]))
        const itemSkippedPaths = Array.from(new Set([...prevSkippedPaths, ...rawSkippedPaths]))
        const combinedOutcomes = [...prevOutcomes, ...rawOutcomes]

        let itemStatus: InstallationItemStatus
        if (combinedOutcomes.length > 0) {
          const hasInstalled = combinedOutcomes.some(o => o.status === 'installed')
          const hasSkipped = combinedOutcomes.some(o => o.status === 'skipped')
          const hasFailed = combinedOutcomes.some(o => o.status === 'failed')

          if (hasFailed) {
            itemStatus = hasInstalled || hasSkipped ? 'partial' : 'failed'
          } else if (hasInstalled) {
            itemStatus = 'installed'
          } else {
            itemStatus = 'skipped'
          }
        } else {
          if (!res.success) {
            itemStatus = itemInstalledPaths.length > 0 ? 'partial' : 'failed'
          } else if (res.status === 'partial') {
            itemStatus = 'partial'
          } else if (res.status === 'skipped' || itemInstalledPaths.length === 0) {
            itemStatus = 'skipped'
          } else {
            itemStatus = 'installed'
          }
        }

        items.push({
          itemKey: item.key,
          skillId: item.skillId,
          subSkillId: item.subSkillId,
          displayName: item.displayName,
          gitUrl: item.gitUrl,
          subPath: item.subPath,
          status: itemStatus,
          message: res.message,
          installedPaths: itemInstalledPaths,
          skippedPaths: itemSkippedPaths,
          targetOutcomes: combinedOutcomes,
          error:
            itemStatus === 'failed' || itemStatus === 'partial'
              ? res.message || '安装失败'
              : undefined,
        })
      } catch (err) {
        const errMessage = err instanceof Error ? err.message : String(err)
        const failedAttemptOutcomes: InstallTargetOutcome[] = targetAgentsToInstall.map(agent => ({
          targetAgent: agent,
          targetPath: '',
          status: 'failed',
          error: errMessage,
          itemKey: `${item.key}::${agent}`,
        }))
        const combinedOutcomes = [...prevOutcomes, ...failedAttemptOutcomes]

        items.push({
          itemKey: item.key,
          skillId: item.skillId,
          subSkillId: item.subSkillId,
          displayName: item.displayName,
          gitUrl: item.gitUrl,
          subPath: item.subPath,
          status:
            prevInstalledPaths.length > 0 ||
            prevOutcomes.some(o => o.status === 'installed' || o.status === 'skipped')
              ? 'partial'
              : 'failed',
          message: errMessage,
          installedPaths: prevInstalledPaths,
          skippedPaths: prevSkippedPaths,
          targetOutcomes: combinedOutcomes,
          error: errMessage,
        })
      } finally {
        installingIds.value = installingIds.value.filter(id => id !== item.key)
      }
    }

    let installedCount = 0
    let skippedCount = 0
    let failedCount = 0
    let missingCount = 0

    for (const it of items) {
      if (it.status === 'missing') {
        missingCount += 1
      }
      if (it.targetOutcomes && it.targetOutcomes.length > 0) {
        for (const out of it.targetOutcomes) {
          if (out.status === 'installed') {
            installedCount += 1
          } else if (out.status === 'skipped') {
            skippedCount += 1
          } else if (out.status === 'failed') {
            failedCount += 1
          }
        }
      } else {
        if (it.status === 'installed') {
          installedCount += 1
        } else if (it.status === 'skipped') {
          skippedCount += 1
        } else if (it.status === 'failed') {
          failedCount += 1
        }
      }
    }

    const installedPaths = Array.from(new Set(items.flatMap(i => i.installedPaths)))
    const skippedPaths = Array.from(new Set(items.flatMap(i => i.skippedPaths)))

    let batchStatus: InstallationBatchResult['status']
    let batchSuccess: boolean
    let batchMessage: string

    const hasUnresolvedErrors = items.some(
      i =>
        Boolean(i.error) ||
        i.status === 'failed' ||
        i.status === 'partial' ||
        (i.targetOutcomes && i.targetOutcomes.some(to => to.status === 'failed'))
    )

    if (failedCount === 0 && missingCount === 0 && !hasUnresolvedErrors) {
      batchSuccess = true
      if (installedCount > 0) {
        batchStatus = 'installed'
        if (skippedCount > 0) {
          batchMessage = `成功新装 ${installedCount} 项，跳过已存在 ${skippedCount} 项`
        } else if (pendingSubSkill.value) {
          batchMessage = `成功单独装配子技能「${pendingSubSkill.value.titleZh || pendingSubSkill.value.name}」`
        } else {
          batchMessage = `成功安装 ${installedCount} 个技能`
        }
      } else {
        batchStatus = 'skipped'
        batchMessage = '目标目录已存在该技能，无需重复安装'
      }
    } else if (installedCount > 0 || installedPaths.length > 0) {
      batchStatus = 'partial'
      batchSuccess = false
      const firstErr = items.find(i => i.error)?.error || '部分目标安装失败'
      batchMessage = `部分完成：已新装 ${installedCount} 项，失败 ${failedCount + missingCount} 项（${firstErr}）`
    } else {
      batchStatus = 'failed'
      batchSuccess = false
      batchMessage = items.find(i => i.error)?.error || '安装失败'
    }

    const batchResult: InstallationBatchResult = {
      operationId,
      retryOfOperationId,
      status: batchStatus,
      success: batchSuccess,
      message: batchMessage,
      count: installedCount,
      installedCount,
      skippedCount,
      failedCount,
      missingCount,
      installedPaths,
      skippedPaths,
      items,
    }

    if (requestSeq === installOperationSeq) {
      lastInstallBatchResult.value = batchResult
      if (batchStatus === 'installed' || batchStatus === 'skipped') {
        closeInstallDialog()
      } else {
        installError.value = batchMessage
        installProgress.value = {
          operationId,
          phase: 'failed',
          message: batchStatus === 'partial' ? '部分安装未完成' : '安装未完成',
          percent: null,
        }
      }
    }

    return batchResult
  }

  function resolveCatalogSkill(skillOrId: CatalogSkill | string): CatalogSkill | undefined {
    if (typeof skillOrId !== 'string') return skillOrId
    const cleanId = skillOrId.toLowerCase().trim()
    return catalogSkills.value.find(s => s.id.toLowerCase() === cleanId)
  }

  // 辅助判断某技能是否已在本地安装（基于来源 gitUrl + 区分大小写 subPath 严格判定，支持按目标 Agent/项目过滤）
  function isSkillInstalled(
    skillOrId: CatalogSkill | string,
    installedSkills: AgentSkill[],
    filter?: SkillTargetFilter
  ): boolean {
    const catalogEntry = resolveCatalogSkill(skillOrId)
    if (!catalogEntry) return false
    return isCatalogSkillInstalledInTargets(catalogEntry, installedSkills, filter)
  }

  function isSubSkillInstalled(
    parentSkill: CatalogSkill,
    subSkill: SubSkillItem,
    installedSkills: AgentSkill[],
    filter?: SkillTargetFilter
  ): boolean {
    return isSubSkillInstalledInTargets(parentSkill, subSkill, installedSkills, filter)
  }

  function getSkillInstalledTargets(
    skillOrId: CatalogSkill | string,
    installedSkills: AgentSkill[],
    options?: { subSkillId?: string; subPath?: string }
  ): InstalledTargetsSummary {
    const catalogEntry = resolveCatalogSkill(skillOrId)
    if (!catalogEntry) {
      return {
        isInstalled: false,
        count: 0,
        agentTypes: [],
        labels: [],
        skills: [],
      }
    }

    if (
      catalogEntry.isBundle &&
      Array.isArray(catalogEntry.subSkills) &&
      catalogEntry.subSkills.length > 0
    ) {
      if (options?.subSkillId) {
        const sub = catalogEntry.subSkills.find(
          s => s.id.toLowerCase() === options.subSkillId!.toLowerCase()
        )
        if (!sub) {
          return { isInstalled: false, count: 0, agentTypes: [], labels: [], skills: [] }
        }
        return getInstalledTargetsForCandidate(
          buildSubSkillCandidate(catalogEntry, sub),
          installedSkills
        )
      }

      const agentTypesSet = new Set<string>()
      const labelsSet = new Set<string>()
      const matchedSkills: AgentSkill[] = []
      let installedSubCount = 0

      for (const sub of catalogEntry.subSkills) {
        const subSummary = getInstalledTargetsForCandidate(
          buildSubSkillCandidate(catalogEntry, sub),
          installedSkills
        )
        if (subSummary.isInstalled) {
          installedSubCount++
          subSummary.agentTypes.forEach(a => agentTypesSet.add(a))
          subSummary.labels.forEach(l => labelsSet.add(l))
          for (const sk of subSummary.skills) {
            if (!matchedSkills.some(m => m.path === sk.path)) {
              matchedSkills.push(sk)
            }
          }
        }
      }

      return {
        isInstalled: installedSubCount === catalogEntry.subSkills.length,
        count: matchedSkills.length,
        agentTypes: Array.from(agentTypesSet),
        labels: Array.from(labelsSet),
        skills: matchedSkills,
      }
    }

    const candidate = buildCatalogSkillCandidate(catalogEntry)
    if (options?.subSkillId) candidate.subSkillId = options.subSkillId
    if (options?.subPath !== undefined) candidate.subPath = options.subPath
    return getInstalledTargetsForCandidate(candidate, installedSkills)
  }

  // 辅助判断流水线是否已全部安装
  function getPipelineInstallStatus(
    pipeline: SkillPipeline,
    installedSkills: AgentSkill[],
    filter?: SkillTargetFilter
  ): {
    total: number
    installed: number
    isFullyInstalled: boolean
  } {
    const total = pipeline.skills.length
    const installed = pipeline.skills.filter(id =>
      isSkillInstalled(id, installedSkills, filter)
    ).length
    return {
      total,
      installed,
      isFullyInstalled: total > 0 && installed >= total,
    }
  }

  return {
    activeMainTab,
    catalogSkills,
    pipelines,
    searchQuery,
    selectedStage,
    isSearching,
    searchMatchTier,
    searchFallbackNotice,
    usedFallbackEngine,
    searchElapsedMs,
    ftsSearchSkills,
    skillMatchReasons,
    skillSearchScores,
    getSkillMatchReason,
    getSkillSearchScore,
    starredIds,
    starredCount,
    userCustomCategories,
    allCustomCategories,
    getSkillCustomCategories,
    isSkillStarred,
    lastSyncTime,
    isSyncing,
    filteredSkills,
    stageCounts,
    installModalVisible,
    pendingSkill,
    pendingSubSkill,
    selectedBundleSubSkillIds,
    pendingPipeline,
    installingIds,
    installProgress,
    installError,
    installCurrent,
    installTotal,
    installPreference,
    reinstallPreferenceOverride,
    lastInstallBatchResult,
    catalogRevision,
    snapshotVersion,
    detailDrawerVisible,
    activeDetailSkill,
    activeDetailPipeline,
    toggleStar,
    addSkillCustomCategory,
    removeSkillCustomCategory,
    openInstallDialog,
    openSubSkillInstallDialog,
    openPipelineInstallDialog,
    openReinstallFromHistory,
    closeInstallDialog,
    openDetail,
    openPipelinePlaybook,
    closeDetail,
    updateInstallPreference,
    syncRegistry,
    executeInstall,
    isSkillInstalled,
    isSubSkillInstalled,
    getSkillInstalledTargets,
    getPipelineInstallStatus,
    selectedCategory,
    activeSort,
    totalMatched,
    hasMore,
    isTruncated,
    clearDiscoveryFilters,
    syncStatusMessage,
    isLoadingMore,
    loadMoreError,
    loadMoreSearchResults,
  }
})
