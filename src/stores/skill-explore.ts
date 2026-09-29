import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import {
  FALLBACK_SEED_PIPELINES,
  FALLBACK_SEED_SKILLS,
  getLastSyncTime,
  getLocalInstallPreference,
  getLocalStars,
  getUserCustomCategories,
  loadHarvestedCatalogSkills,
  loadHarvestedPipelines,
  removeUserCustomCategory,
  saveLocalInstallPreference,
  saveLocalStars,
  saveUserCustomCategory,
  syncOnlineRegistry,
} from '@/services/skills/skill-catalog-service'
import {
  installAgentSkill,
} from '@/services/skills/skills-service'
import type {
  AgentSkill,
  CatalogSkill,
  InstallPreference,
  InstallSkillProgress,
  SdlcStage,
  SkillPipeline,
  SubSkillItem,
} from '@/types/skills'
import { skillCategoryLabel } from '@/utils/skill-category-label'

export const useSkillExploreStore = defineStore('skillExplore', () => {
  // 顶层主标签：已安装列表 vs 技能探索与流水线
  const activeMainTab = ref<'installed' | 'explore' | 'history'>('installed')

  // 本地星标与评分缓存
  const starredIds = ref<Set<string>>(getLocalStars())
  const userCustomCategories = ref<Record<string, string[]>>(getUserCustomCategories())
  const lastSyncTime = ref<number | null>(getLastSyncTime())
  const isSyncing = ref(false)
  const syncStatusMessage = ref('')

  // 基础数据与数据源
  function decorateSkills(skills: CatalogSkill[]): CatalogSkill[] {
    return skills.map(s => {
      const clone = { ...s }
      clone.isStar = starredIds.value.has(s.id)
      return clone
    })
  }

  const catalogSkills = ref<CatalogSkill[]>(
    decorateSkills(FALLBACK_SEED_SKILLS.map(skill => ({ ...skill })))
  )
  const pipelines = ref<SkillPipeline[]>(FALLBACK_SEED_PIPELINES)

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
  const installProgress = ref<InstallSkillProgress | null>(null)
  const installError = ref('')
  const installCurrent = ref(0)
  const installTotal = ref(0)
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
    const terms = searchQuery.value.trim().toLowerCase().split(/\s+/).filter(Boolean)
    if (terms.length) {
      result = result.filter(s => {
        const customTags = userCustomCategories.value[s.id] || []
        const searchable = [s.name, s.titleZh, s.description, s.category, skillCategoryLabel(s.category), s.author.name,
          s.gitUrl, s.subPath || '', s.usageGuide?.summary || '', s.usageGuide?.trigger || '',
          ...(s.usageGuide?.steps || []), ...s.tags, ...customTags].join(' ').toLowerCase()
        return terms.every(term => searchable.includes(term))
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

  function addSkillCustomCategory(skillId: string, category: string) {
    saveUserCustomCategory(skillId, category)
    userCustomCategories.value = getUserCustomCategories()
  }

  function removeSkillCustomCategory(skillId: string, category: string) {
    removeUserCustomCategory(skillId, category)
    userCustomCategories.value = getUserCustomCategories()
  }

  function openInstallDialog(skill: CatalogSkill) {
    installError.value = ''
    installProgress.value = null
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
    pendingSkill.value = parentSkill
    pendingSubSkill.value = subSkill
    pendingPipeline.value = null
    selectedBundleSubSkillIds.value = [subSkill.id]
    installModalVisible.value = true
  }

  function openPipelineInstallDialog(pipeline: SkillPipeline) {
    installError.value = ''
    installProgress.value = null
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
    installProgress.value = null
    installError.value = ''
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

  async function syncRegistry(): Promise<{ updatedCount: number; totalCount: number; message: string }> {
    isSyncing.value = true
    try {
      const res = await syncOnlineRegistry()
      if (res.success) lastSyncTime.value = res.lastSyncAt
      syncStatusMessage.value = res.message

      if (res.skills && res.skills.length > 0) {
        catalogSkills.value = decorateSkills(res.skills)
      }
      if (res.success) {
        pipelines.value = res.pipelines
      }

      return { updatedCount: res.updatedCount, totalCount: res.totalCount, message: res.message }
    } finally {
      isSyncing.value = false
    }
  }


  async function executeInstall(projectCwd?: string): Promise<{
    success: boolean
    message: string
    count: number
    installedPaths: string[]
  }> {
    const pref = installPreference.value
    installError.value = ''
    installProgress.value = { phase: 'preparing', message: '正在准备安装…', percent: null }
    const reportProgress = (progress: InstallSkillProgress) => { installProgress.value = progress }
    const fail = (message: string) => {
      installError.value = message
      installProgress.value = { phase: 'failed', message: '安装未完成', percent: null }
      return { success: false, message, count: 0, installedPaths: [] }
    }

    // 1. 如果是单独装配子技能
    if (pendingSubSkill.value && pendingSkill.value) {
      const parent = pendingSkill.value
      const sub = pendingSubSkill.value
      installCurrent.value = 1
      installTotal.value = 1
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
        }, reportProgress)
        if (res.success) {
          closeInstallDialog()
          return { success: true, message: res.installedPaths.length ? `成功单独装配子技能「${sub.titleZh}」` : res.message, count: res.installedPaths.length ? 1 : 0, installedPaths: res.installedPaths }
        } else {
          return fail(res.message || '安装失败')
        }
      } catch (err) {
        return fail(err instanceof Error ? err.message : String(err))
      } finally {
        installingIds.value = installingIds.value.filter(id => id !== sub.id)
      }
    }

    // 2. 如果是合集且勾选了特定子技能
    if (pendingSkill.value?.isBundle && pendingSkill.value.subSkills && selectedBundleSubSkillIds.value.length > 0) {
      const parent = pendingSkill.value
      const selectedSubs = parent.subSkills.filter(s => selectedBundleSubSkillIds.value.includes(s.id))
      installTotal.value = selectedSubs.length
      let successCount = 0
      let lastError = ''
      const installedPaths: string[] = []
      for (const [index, sub] of selectedSubs.entries()) {
        installCurrent.value = index + 1
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
          }, reportProgress)
          if (res.success) {
            successCount++
            installedPaths.push(...res.installedPaths)
          } else {
            lastError = res.message
          }
        } catch (err) {
          lastError = String(err)
        } finally {
          installingIds.value = installingIds.value.filter(id => id !== sub.id)
        }
      }
      if (successCount === selectedSubs.length) {
        closeInstallDialog()
        return { success: true, message: `成功装配 ${successCount} 个子技能`, count: successCount, installedPaths }
      } else if (successCount > 0) {
        closeInstallDialog()
        return { success: true, message: `成功装配 ${successCount} 个子技能（部分失败: ${lastError}）`, count: successCount, installedPaths }
      } else {
        return fail(lastError || '安装失败')
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
      return fail('未选择待安装技能')
    }

    let successCount = 0
    let lastError = ''
    const installedPaths: string[] = []
    installTotal.value = skillsToInstall.length

    for (const [index, skill] of skillsToInstall.entries()) {
      installCurrent.value = index + 1
      if (installingIds.value.includes(skill.id)) continue
      installingIds.value.push(skill.id)
      try {
        const res = await installAgentSkill({
          skillId: skill.id,
          gitUrl: skill.gitUrl,
          subPath: skill.subPath,
          targetMode: pref.targetMode,
          targetAgents: pref.targetAgents,
          projectCwd: pref.customProjectPath || projectCwd,
          addToGitignore: pref.addToGitignore,
        }, reportProgress)
        if (res.success) {
          successCount++
          installedPaths.push(...res.installedPaths)
        } else {
          lastError = res.message
        }
      } catch (err) {
        lastError = String(err)
      } finally {
        installingIds.value = installingIds.value.filter(id => id !== skill.id)
      }
    }

    if (successCount === skillsToInstall.length) {
      closeInstallDialog()
      return {
        success: true,
        message: `成功安装 ${successCount} 个技能`,
        count: successCount,
        installedPaths,
      }
    } else if (successCount > 0) {
      closeInstallDialog()
      return {
        success: true,
        message: `成功安装 ${successCount} 个技能（部分失败: ${lastError}）`,
        count: successCount,
        installedPaths,
      }
    } else {
      return fail(lastError || '安装失败')
    }
  }

  // 辅助判断某技能是否已在本地安装
  function isSkillInstalled(skillId: string, installedSkills: AgentSkill[]): boolean {
    const clean = skillId.toLowerCase().trim()
    return installedSkills.some(s => {
      const folderName = s.path.split(/[\\/]/).pop()?.toLowerCase()
      return folderName === clean || s.id.toLowerCase() === clean
    })
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
    installProgress,
    installError,
    installCurrent,
    installTotal,
    installPreference,
    detailDrawerVisible,
    activeDetailSkill,
    activeDetailPipeline,
    toggleStar,
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
    syncStatusMessage,
  }
})
