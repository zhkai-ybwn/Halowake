import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import {
  checkSkillsUpdates,
  openSkillFolder,
  scanAgentSkills,
  uninstallAgentSkill,
  updateAgentSkill,
  type AgentSkill,
  type SkillUpdateResult,
} from '@/services/skills/skills-service'

function cleanSkillDescription(desc?: string): string {
  if (!desc) return ''
  const trimmed = desc.trim()
  if (['>-', '>', '|-', '|', '>+', '|+'].includes(trimmed)) return ''
  return trimmed
}

function getAgentLabel(agentType: string): string {
  switch (agentType) {
    case 'agents': return 'Agents'
    case 'codex': return 'Codex'
    case 'antigravity': return 'Antigravity'
    case 'cursor': return 'Cursor'
    case 'trae': return 'Trae'
    case 'claude': return 'Claude Code'
    case 'qoder': return 'Qoder'
    case 'opencode': return 'OpenCode'
    case 'windsurf': return 'Windsurf'
    case 'cline': return 'Cline'
    case 'project': return '项目专属'
    default: return agentType
  }
}

function computeSkillMetadata(skill: AgentSkill): void {
  const clean = cleanSkillDescription(skill.description)
  skill.cleanDesc = clean
  if (skill.associatedAgents && skill.associatedAgents.length > 1) {
    skill.displayAgentTag = skill.associatedAgents.map(a => getAgentLabel(a)).join(' / ')
  } else {
    skill.displayAgentTag = skill.agentLabel || getAgentLabel(skill.agentType)
  }
  const agentsText = (skill.associatedAgents || []).join(' ')
  skill.searchIndex = `${skill.name} ${clean} ${skill.agentLabel} ${skill.displayAgentTag} ${agentsText}`.toLowerCase()
}

export const useSkillsStore = defineStore('skills', () => {
  const skills = ref<AgentSkill[]>([])
  const loading = ref(false)
  const checkingUpdates = ref(false)
  const updatingPaths = ref<string[]>([])
  const uninstallingPaths = ref<string[]>([])
  const modalVisible = ref(false)
  const searchQuery = ref('')
  const selectedCategory = ref<string>('all')
  const lastCheckTime = ref<number | null>(null)
  const errorMessage = ref<string | null>(null)
  const installationRevealCount = ref(0)
  const installationRevealQuery = ref('')
  let scanRequestId = 0
  let checkRequestId = 0
  let scopeGeneration = 0
  let activeProjectCwd: string | undefined

  const updateCount = computed(() =>
    skills.value.filter(s => s.hasUpdate && s.management !== 'unmanaged' && s.management !== 'external').length
  )
  const hasUpdates = computed(() => updateCount.value > 0)

  const availableCategories = computed(() => {
    const set = new Set<string>()
    skills.value.forEach(s => {
      if (s.associatedAgents && s.associatedAgents.length > 0) {
        s.associatedAgents.forEach(a => set.add(a))
      } else if (s.agentType) {
        set.add(s.agentType)
      }
    })
    return Array.from(set)
  })

  const categoryCounts = computed<Record<string, number>>(() => {
    const counts: Record<string, number> = {}
    for (const s of skills.value) {
      if (s.associatedAgents && s.associatedAgents.length > 0) {
        for (const a of s.associatedAgents) {
          counts[a] = (counts[a] || 0) + 1
        }
      } else if (s.agentType) {
        counts[s.agentType] = (counts[s.agentType] || 0) + 1
      }
    }
    return counts
  })

  const filteredSkills = computed(() => {
    let result = skills.value

    if (selectedCategory.value === 'updates') {
      result = result.filter(s => s.hasUpdate)
    } else if (selectedCategory.value !== 'all') {
      result = result.filter(
        s =>
          s.agentType === selectedCategory.value ||
          (s.associatedAgents && s.associatedAgents.includes(selectedCategory.value))
      )
    }

    const q = searchQuery.value.trim().toLowerCase()
    if (q) {
      result = result.filter(s =>
        s.searchIndex
          ? s.searchIndex.includes(q)
          : (s.name.toLowerCase().includes(q) ||
             s.description.toLowerCase().includes(q) ||
             s.agentLabel.toLowerCase().includes(q))
      )
    }

    return result
  })

  async function scan(projectCwd?: string, options?: { silent?: boolean }) {
    activeProjectCwd = projectCwd
    const gen = ++scopeGeneration
    const requestId = ++scanRequestId
    const isSilent = Boolean(options?.silent && skills.value.length > 0)
    if (!isSilent) {
      loading.value = true
    }
    errorMessage.value = null
    const prevIds = new Set(skills.value.map(s => s.id))
    try {
      const scanned = await scanAgentSkills(projectCwd)
      if (requestId !== scanRequestId || gen !== scopeGeneration) {
        return { total: skills.value.length, newCount: 0, newSkills: [] }
      }
      // 保留已有的更新状态（如果存在）
      const updateMap = new Map(skills.value.map(s => [s.id, s.hasUpdate]))
      const summaryMap = new Map(skills.value.map(s => [s.id, s.updateSummary]))
      const behindMap = new Map(skills.value.map(s => [s.id, s.commitsBehind]))
      const statusMap = new Map(skills.value.map(s => [s.id, s.updateStatus]))
      const errorMap = new Map(skills.value.map(s => [s.id, s.updateError]))
      scanned.forEach(s => {
        if (updateMap.has(s.id)) {
          s.hasUpdate = updateMap.get(s.id) || false
          s.updateSummary = summaryMap.get(s.id)
          s.commitsBehind = behindMap.get(s.id) || 0
          s.updateStatus = statusMap.get(s.id) ?? s.updateStatus
          s.updateError = errorMap.get(s.id) ?? s.updateError
        }
        computeSkillMetadata(s)
      })
      const newSkills = prevIds.size > 0 ? scanned.filter(s => !prevIds.has(s.id)) : []
      skills.value = scanned
      return { total: scanned.length, newCount: newSkills.length, newSkills }
    } catch (err: unknown) {
      if (requestId !== scanRequestId || gen !== scopeGeneration) {
        return { total: skills.value.length, newCount: 0, newSkills: [] }
      }
      console.warn('Failed to scan agent skills:', err)
      errorMessage.value = String(err)
      return { total: skills.value.length, newCount: 0, newSkills: [] }
    } finally {
      if (requestId === scanRequestId && gen === scopeGeneration && !isSilent) loading.value = false
    }
  }

  async function checkUpdates(projectCwd?: string) {
    if (checkingUpdates.value && projectCwd === activeProjectCwd) {
      return {
        total: skills.value.length,
        newCount: 0,
        newSkills: [],
        updateCount: updateCount.value,
        unknownCount: 0,
        unsupportedCount: 0,
      }
    }
    activeProjectCwd = projectCwd
    const gen = ++scopeGeneration
    const reqId = ++checkRequestId
    checkingUpdates.value = true
    errorMessage.value = null
    const prevIds = new Set(skills.value.map(s => s.id))
    try {
      const updated = await checkSkillsUpdates(projectCwd)
      if (gen !== scopeGeneration || reqId !== checkRequestId) {
        return {
          total: skills.value.length,
          newCount: 0,
          newSkills: [],
          updateCount: updateCount.value,
          unknownCount: 0,
          unsupportedCount: 0,
        }
      }
      updated.forEach(computeSkillMetadata)
      const newSkills = prevIds.size > 0 ? updated.filter(s => !prevIds.has(s.id)) : []
      skills.value = updated
      lastCheckTime.value = Date.now()
      return {
        total: updated.length,
        newCount: newSkills.length,
        newSkills,
        updateCount: updated.filter(s => s.hasUpdate && s.management !== 'unmanaged' && s.management !== 'external').length,
        unknownCount: updated.filter(s => s.updateStatus === 'unknown' || s.updateStatus === 'error').length,
        unsupportedCount: updated.filter(s => s.updateStatus === 'unsupported').length,
      }
    } catch (err: unknown) {
      if (gen !== scopeGeneration || reqId !== checkRequestId) {
        return {
          total: skills.value.length,
          newCount: 0,
          newSkills: [],
          updateCount: updateCount.value,
          unknownCount: 0,
          unsupportedCount: 0,
        }
      }
      console.warn('Failed to check agent skills updates:', err)
      errorMessage.value = String(err)
      return {
        total: skills.value.length,
        newCount: 0,
        newSkills: [],
        updateCount: updateCount.value,
        unknownCount: 0,
        unsupportedCount: 0,
      }
    } finally {
      if (reqId === checkRequestId) checkingUpdates.value = false
    }
  }

  async function updateSingle(skill: AgentSkill, force = false): Promise<SkillUpdateResult> {
    if (skill.management === 'unmanaged' || skill.management === 'external' || skill.externallyManaged) {
      return {
        skillId: skill.id,
        success: false,
        message: '未受管或外部管理技能不支持自动更新，请打开目录手动管理',
      }
    }
    if (updatingPaths.value.includes(skill.path) || uninstallingPaths.value.includes(skill.path)) {
      return { skillId: skill.id, success: false, message: '正在执行其他操作，请稍候' }
    }

    const gen = scopeGeneration
    updatingPaths.value.push(skill.path)
    try {
      const res = await updateAgentSkill(skill.path, force)
      if (res.success && gen === scopeGeneration) {
        // 更新本地状态
        const idx = skills.value.findIndex(s => s.path === skill.path)
        if (idx !== -1) {
          skills.value[idx].hasUpdate = false
          skills.value[idx].updateStatus = 'up_to_date'
          skills.value[idx].updateError = undefined
          skills.value[idx].commitsBehind = 0
          skills.value[idx].currentCommit = res.newCommit || skills.value[idx].currentCommit
          skills.value[idx].isDirty = false
          skills.value[idx].updateSummary = undefined
          if (skills.value[idx].latestVersion) {
            skills.value[idx].version = skills.value[idx].latestVersion
            skills.value[idx].latestVersion = undefined
          }
        }
      }
      return res
    } catch (err: unknown) {
      return {
        skillId: skill.id,
        success: false,
        message: err instanceof Error ? err.message : String(err),
      }
    } finally {
      updatingPaths.value = updatingPaths.value.filter(p => p !== skill.path)
    }
  }

  async function updateAll(): Promise<SkillUpdateResult[]> {
    const toUpdate = skills.value.filter(
      s => s.hasUpdate && s.management !== 'unmanaged' && s.management !== 'external' && !s.externallyManaged
    )
    const results: SkillUpdateResult[] = []

    for (const skill of toUpdate) {
      const res = await updateSingle(skill, false)
      results.push(res)
    }

    return results
  }

  async function uninstallSingle(skill: AgentSkill, projectCwd?: string): Promise<void> {
    if (uninstallingPaths.value.includes(skill.path)) return
    if (updatingPaths.value.includes(skill.path)) {
      throw new Error('技能正在更新中，无法同时卸载')
    }
    const gen = scopeGeneration
    const targetCwd = projectCwd !== undefined ? projectCwd : activeProjectCwd
    uninstallingPaths.value.push(skill.path)
    try {
      await uninstallAgentSkill(skill.path, targetCwd)
      if (gen === scopeGeneration) {
        await scan(targetCwd)
      }
    } finally {
      uninstallingPaths.value = uninstallingPaths.value.filter(path => path !== skill.path)
    }
  }

  async function openFolder(path: string) {
    try {
      await openSkillFolder(path)
    } catch (err: unknown) {
      console.warn('Failed to open skill folder:', err)
    }
  }

  function openModal() {
    modalVisible.value = true
  }

  function closeModal() {
    modalVisible.value = false
  }

  function revealInstalledSkills(query = '') {
    installationRevealQuery.value = query
    installationRevealCount.value++
  }

  return {
    skills,
    loading,
    checkingUpdates,
    updatingPaths,
    uninstallingPaths,
    modalVisible,
    searchQuery,
    selectedCategory,
    lastCheckTime,
    errorMessage,
    installationRevealCount,
    installationRevealQuery,
    updateCount,
    hasUpdates,
    availableCategories,
    categoryCounts,
    filteredSkills,
    scan,
    checkUpdates,
    updateSingle,
    updateAll,
    uninstallSingle,
    openFolder,
    openModal,
    closeModal,
    revealInstalledSkills,
  }
})
