import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import {
  DEFAULT_REGISTRY_URL,
  FALLBACK_SEED_PIPELINES,
  FALLBACK_SEED_SKILLS,
  getLastSyncTime,
  getLocalInstallPreference,
  getLocalRatings,
  getLocalStars,
  getRemoteRegistryUrl,
  getSavedGitHubToken,
  getUserCustomCategories,
  loadHarvestedCatalogSkills,
  loadHarvestedPipelines,
  removeUserCustomCategory,
  saveLocalInstallPreference,
  saveLocalRating,
  saveLocalStars,
  saveRemoteRegistryUrl,
  saveSavedGitHubToken,
  saveUserCustomCategory,
  syncOnlineRegistry,
  testRegistryConnection,
} from '@/services/skills/skill-catalog-service'
import {
  installAgentSkill,
} from '@/services/skills/skills-service'
import type {
  AgentSkill,
  CatalogSkill,
  InstallPreference,
  SdlcStage,
  SkillPipeline,
  SkillSource,
  SubSkillItem,
} from '@/types/skills'

export const useSkillExploreStore = defineStore('skillExplore', () => {
  // 顶层主标签：已安装列表 vs 技能探索与流水线
  const activeMainTab = ref<'installed' | 'explore'>('installed')

  // 本地星标与评分缓存
  const starredIds = ref<Set<string>>(getLocalStars())
  const userRatings = ref<Record<string, number>>(getLocalRatings())
  const userCustomCategories = ref<Record<string, string[]>>(getUserCustomCategories())
  const lastSyncTime = ref<number | null>(getLastSyncTime())
  const isSyncing = ref(false)
  const syncStatusMessage = ref('')
  const registryUrl = ref<string>(getRemoteRegistryUrl())

  // 基础数据与数据源
  function decorateSkills(skills: CatalogSkill[]): CatalogSkill[] {
    return skills.map(s => {
      const clone = { ...s }
      clone.isStar = starredIds.value.has(s.id)
      if (userRatings.value[s.id] !== undefined) {
        clone.userRating = userRatings.value[s.id]
      }
      return clone
    })
  }

  const catalogSkills = ref<CatalogSkill[]>(
    decorateSkills(FALLBACK_SEED_SKILLS.map(skill => ({ ...skill })))
  )
  const pipelines = ref<SkillPipeline[]>(FALLBACK_SEED_PIPELINES)
  const skillSources = ref<SkillSource[]>([])
  const sourcesModalVisible = ref(false)
  const githubToken = ref(getSavedGitHubToken())

  // 探索页面的过滤与搜索
  const searchQuery = ref('')
  const selectedStage = ref<SdlcStage | 'all' | 'starred' | string>('all')

  // 安装向导状态
  const installModalVisible = ref(false)
  const pendingSkill = ref<CatalogSkill | null>(null)
  const pendingSubSkill = ref<SubSkillItem | null>(null)
  const selectedBundleSubSkillIds = ref<string[]>([])
  const pendingPipeline = ref<SkillPipeline | null>(null)
  const installingIds = ref<string[]>([])
  const installPreference = ref<InstallPreference>(getLocalInstallPreference())

  // 详情抽屉状态 (支持单技能指引 OR 流水线全流程玩法指引)
  const detailDrawerVisible = ref(false)
  const activeDetailSkill = ref<CatalogSkill | null>(null)
  const activeDetailPipeline = ref<SkillPipeline | null>(null)

  // 异步初始化：从本地缓存或种子读取技能，并在必要时从远端注册表更新
  async function initCatalogAndSources() {
    try {
      const cached = await loadHarvestedCatalogSkills()
      const cachedPipelines = await loadHarvestedPipelines()
      if (cached && cached.length > 0) {
        catalogSkills.value = decorateSkills(cached)
      }
      if (cachedPipelines && cachedPipelines.length > 0) {
        pipelines.value = cachedPipelines
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

  const starredCount = computed(() => starredIds.value.size)

  const allCustomCategories = computed(() => {
    const set = new Set<string>()
    Object.values(userCustomCategories.value).forEach(list => list.forEach(item => set.add(item)))
    return Array.from(set)
  })

  const filteredSkills = computed(() => {
    let result = catalogSkills.value

    // 阶段/自定义分类过滤
    if (selectedStage.value === 'starred') {
      result = result.filter(s => starredIds.value.has(s.id))
    } else if (['requirement', 'design', 'coding', 'testing', 'delivery'].includes(selectedStage.value)) {
      result = result.filter(s => s.stage === selectedStage.value)
    } else if (selectedStage.value !== 'all') {
      // 匹配用户自定义标签
      result = result.filter(s => {
        const customTags = userCustomCategories.value[s.id] || []
        return customTags.includes(selectedStage.value)
      })
    }

    // 搜索过滤
    const q = searchQuery.value.trim().toLowerCase()
    if (q) {
      result = result.filter(s => {
        const customTags = userCustomCategories.value[s.id] || []
        return (
          s.name.toLowerCase().includes(q) ||
          s.titleZh.toLowerCase().includes(q) ||
          s.description.toLowerCase().includes(q) ||
          s.tags.some(t => t.toLowerCase().includes(q)) ||
          customTags.some(t => t.toLowerCase().includes(q))
        )
      })
    }

    return result
  })

  // 按 SDLC 阶段统计可用技能数
  const stageCounts = computed<Record<string, number>>(() => {
    const counts: Record<string, number> = { all: catalogSkills.value.length, starred: starredCount.value }
    catalogSkills.value.forEach(s => {
      counts[s.stage] = (counts[s.stage] || 0) + 1
    })
    allCustomCategories.value.forEach(cat => {
      let c = 0
      for (const [id, tags] of Object.entries(userCustomCategories.value)) {
        if (tags.includes(cat) && catalogSkills.value.some(s => s.id === id)) {
          c++
        }
      }
      counts[cat] = c
    })
    return counts
  })

  function toggleStar(skillId: string) {
    if (starredIds.value.has(skillId)) {
      starredIds.value.delete(skillId)
    } else {
      starredIds.value.add(skillId)
    }
    saveLocalStars(starredIds.value)
    const target = catalogSkills.value.find(s => s.id === skillId)
    if (target) {
      target.isStar = starredIds.value.has(skillId)
    }
  }

  function rateSkill(skillId: string, rating: number) {
    userRatings.value[skillId] = rating
    saveLocalRating(skillId, rating)
    const target = catalogSkills.value.find(s => s.id === skillId)
    if (target) {
      target.userRating = rating
    }
  }

  function addSkillCustomCategory(skillId: string, category: string) {
    saveUserCustomCategory(skillId, category)
    userCustomCategories.value = getUserCustomCategories()
  }

  function removeSkillCustomCategory(skillId: string, category: string) {
    removeUserCustomCategory(skillId, category)
    userCustomCategories.value = getUserCustomCategories()
  }

  function openInstallDialog(skill: CatalogSkill) {
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
    pendingSkill.value = parentSkill
    pendingSubSkill.value = subSkill
    pendingPipeline.value = null
    selectedBundleSubSkillIds.value = [subSkill.id]
    installModalVisible.value = true
  }

  function openPipelineInstallDialog(pipeline: SkillPipeline) {
    pendingPipeline.value = pipeline
    pendingSkill.value = null
    pendingSubSkill.value = null
    selectedBundleSubSkillIds.value = []
    installModalVisible.value = true
  }

  function closeInstallDialog() {
    installModalVisible.value = false
    pendingSkill.value = null
    pendingSubSkill.value = null
    pendingPipeline.value = null
    selectedBundleSubSkillIds.value = []
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

  function updateInstallPreference(pref: InstallPreference) {
    installPreference.value = { ...pref }
    if (pref.remember) {
      saveLocalInstallPreference(pref)
    }
  }

  async function syncRegistry(urlOverride?: string): Promise<{ updatedCount: number; totalCount: number; message: string }> {
    isSyncing.value = true
    try {
      const targetUrl = urlOverride || registryUrl.value
      const res = await syncOnlineRegistry(targetUrl)
      lastSyncTime.value = res.lastSyncAt
      syncStatusMessage.value = res.message

      if (res.skills && res.skills.length > 0) {
        catalogSkills.value = decorateSkills(res.skills)
      }
      if (res.pipelines && res.pipelines.length > 0) {
        pipelines.value = res.pipelines
      }

      return { updatedCount: res.updatedCount, totalCount: res.totalCount, message: res.message }
    } finally {
      isSyncing.value = false
    }
  }

  function updateRegistryUrl(url: string) {
    saveRemoteRegistryUrl(url)
    registryUrl.value = url.trim() || DEFAULT_REGISTRY_URL
  }

  async function testRegistry(url: string) {
    return await testRegistryConnection(url)
  }

  function openSourcesModal() {
    sourcesModalVisible.value = true
  }

  function closeSourcesModal() {
    sourcesModalVisible.value = false
  }

  function updateGitHubToken(token: string) {
    saveSavedGitHubToken(token)
    githubToken.value = token
  }


  async function executeInstall(projectCwd?: string): Promise<{
    success: boolean
    message: string
    count: number
  }> {
    const pref = installPreference.value

    // 1. 如果是单独装配子技能
    if (pendingSubSkill.value && pendingSkill.value) {
      const parent = pendingSkill.value
      const sub = pendingSubSkill.value
      installingIds.value.push(sub.id)
      try {
        const res = await installAgentSkill({
          skillId: parent.id,
          gitUrl: parent.gitUrl,
          targetMode: pref.targetMode,
          targetAgents: pref.targetAgents,
          projectCwd: pref.customProjectPath || projectCwd,
          addToGitignore: pref.addToGitignore,
          subSkillId: sub.id,
          subPath: sub.subPath,
        })
        closeInstallDialog()
        if (res.success) {
          return { success: true, message: `成功单独装配子技能「${sub.titleZh}」`, count: 1 }
        } else {
          return { success: false, message: res.message || '安装失败', count: 0 }
        }
      } catch (err) {
        closeInstallDialog()
        return { success: false, message: String(err), count: 0 }
      } finally {
        installingIds.value = installingIds.value.filter(id => id !== sub.id)
      }
    }

    // 2. 如果是合集且勾选了特定子技能
    if (pendingSkill.value?.isBundle && pendingSkill.value.subSkills && selectedBundleSubSkillIds.value.length > 0) {
      const parent = pendingSkill.value
      const selectedSubs = parent.subSkills.filter(s => selectedBundleSubSkillIds.value.includes(s.id))
      let successCount = 0
      let lastError = ''
      for (const sub of selectedSubs) {
        if (installingIds.value.includes(sub.id)) continue
        installingIds.value.push(sub.id)
        try {
          const res = await installAgentSkill({
            skillId: parent.id,
            gitUrl: parent.gitUrl,
            targetMode: pref.targetMode,
            targetAgents: pref.targetAgents,
            projectCwd: pref.customProjectPath || projectCwd,
            addToGitignore: pref.addToGitignore,
            subSkillId: sub.id,
            subPath: sub.subPath,
          })
          if (res.success) {
            successCount++
          } else {
            lastError = res.message
          }
        } catch (err) {
          lastError = String(err)
        } finally {
          installingIds.value = installingIds.value.filter(id => id !== sub.id)
        }
      }
      closeInstallDialog()
      if (successCount === selectedSubs.length) {
        return { success: true, message: `成功装配 ${successCount} 个子技能`, count: successCount }
      } else if (successCount > 0) {
        return { success: true, message: `成功装配 ${successCount} 个子技能（部分失败: ${lastError}）`, count: successCount }
      } else {
        return { success: false, message: lastError || '安装失败', count: 0 }
      }
    }

    // 3. 流水线或普通整包技能
    let skillsToInstall: CatalogSkill[] = []

    if (pendingPipeline.value) {
      const p = pendingPipeline.value
      skillsToInstall = catalogSkills.value.filter(s => p.skills.includes(s.id))
    } else if (pendingSkill.value) {
      skillsToInstall = [pendingSkill.value]
    }

    if (skillsToInstall.length === 0) {
      return { success: false, message: '未选择待安装技能', count: 0 }
    }

    let successCount = 0
    let lastError = ''

    for (const skill of skillsToInstall) {
      if (installingIds.value.includes(skill.id)) continue
      installingIds.value.push(skill.id)
      try {
        const res = await installAgentSkill({
          skillId: skill.id,
          gitUrl: skill.gitUrl,
          targetMode: pref.targetMode,
          targetAgents: pref.targetAgents,
          projectCwd: pref.customProjectPath || projectCwd,
          addToGitignore: pref.addToGitignore,
        })
        if (res.success) {
          successCount++
        } else {
          lastError = res.message
        }
      } catch (err) {
        lastError = String(err)
      } finally {
        installingIds.value = installingIds.value.filter(id => id !== skill.id)
      }
    }

    closeInstallDialog()

    if (successCount === skillsToInstall.length) {
      return {
        success: true,
        message: `成功安装 ${successCount} 个技能`,
        count: successCount,
      }
    } else if (successCount > 0) {
      return {
        success: true,
        message: `成功安装 ${successCount} 个技能（部分失败: ${lastError}）`,
        count: successCount,
      }
    } else {
      return {
        success: false,
        message: lastError || '安装失败',
        count: 0,
      }
    }
  }

  // 辅助判断某技能是否已在本地安装
  function isSkillInstalled(skillId: string, installedSkills: AgentSkill[]): boolean {
    const clean = skillId.toLowerCase().trim()
    return installedSkills.some(s => s.id.toLowerCase() === clean || s.name.toLowerCase() === clean)
  }

  // 辅助判断流水线是否已全部安装
  function getPipelineInstallStatus(pipeline: SkillPipeline, installedSkills: AgentSkill[]): {
    total: number
    installed: number
    isFullyInstalled: boolean
  } {
    const total = pipeline.skills.length
    const installed = pipeline.skills.filter(id => isSkillInstalled(id, installedSkills)).length
    return {
      total,
      installed,
      isFullyInstalled: installed >= total,
    }
  }

  return {
    activeMainTab,
    catalogSkills,
    pipelines,
    searchQuery,
    selectedStage,
    starredIds,
    starredCount,
    userRatings,
    userCustomCategories,
    allCustomCategories,
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
    installPreference,
    detailDrawerVisible,
    activeDetailSkill,
    activeDetailPipeline,
    toggleStar,
    rateSkill,
    addSkillCustomCategory,
    removeSkillCustomCategory,
    openInstallDialog,
    openSubSkillInstallDialog,
    openPipelineInstallDialog,
    closeInstallDialog,
    openDetail,
    openPipelinePlaybook,
    closeDetail,
    updateInstallPreference,
    syncRegistry,
    executeInstall,
    isSkillInstalled,
    getPipelineInstallStatus,
    skillSources,
    sourcesModalVisible,
    githubToken,
    syncStatusMessage,
    registryUrl,
    updateRegistryUrl,
    testRegistry,
    openSourcesModal,
    closeSourcesModal,
    updateGitHubToken,
  }
})

