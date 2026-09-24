import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import {
  checkSkillsUpdates,
  openSkillFolder,
  scanAgentSkills,
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
  const modalVisible = ref(false)
  const searchQuery = ref('')
  const selectedCategory = ref<string>('all')
  const lastCheckTime = ref<number | null>(null)
  const errorMessage = ref<string | null>(null)

  const updateCount = computed(() => skills.value.filter(s => s.hasUpdate).length)
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

  async function scan(projectCwd?: string) {
    loading.value = true
    errorMessage.value = null
    const prevIds = new Set(skills.value.map(s => s.id))
    try {
      const scanned = await scanAgentSkills(projectCwd)
      // 保留已有的更新状态（如果存在）
      const updateMap = new Map(skills.value.map(s => [s.id, s.hasUpdate]))
      const summaryMap = new Map(skills.value.map(s => [s.id, s.updateSummary]))
      const behindMap = new Map(skills.value.map(s => [s.id, s.commitsBehind]))
      scanned.forEach(s => {
        if (updateMap.has(s.id)) {
          s.hasUpdate = updateMap.get(s.id) || false
          s.updateSummary = summaryMap.get(s.id)
          s.commitsBehind = behindMap.get(s.id) || 0
        }
        computeSkillMetadata(s)
      })
      const newSkills = prevIds.size > 0 ? scanned.filter(s => !prevIds.has(s.id)) : []
      skills.value = scanned
      return { total: scanned.length, newCount: newSkills.length, newSkills }
    } catch (err: unknown) {
      console.warn('Failed to scan agent skills:', err)
      errorMessage.value = String(err)
      return { total: skills.value.length, newCount: 0, newSkills: [] }
    } finally {
      loading.value = false
    }
  }

  async function checkUpdates(projectCwd?: string) {
    if (checkingUpdates.value) {
      return { total: skills.value.length, newCount: 0, newSkills: [], updateCount: updateCount.value }
    }
    checkingUpdates.value = true
    errorMessage.value = null
    const prevIds = new Set(skills.value.map(s => s.id))
    try {
      const updated = await checkSkillsUpdates(projectCwd)
      updated.forEach(computeSkillMetadata)
      const newSkills = prevIds.size > 0 ? updated.filter(s => !prevIds.has(s.id)) : []
      skills.value = updated
      lastCheckTime.value = Date.now()
      return {
        total: updated.length,
        newCount: newSkills.length,
        newSkills,
        updateCount: updated.filter(s => s.hasUpdate).length,
      }
    } catch (err: unknown) {
      console.warn('Failed to check agent skills updates:', err)
      errorMessage.value = String(err)
      return { total: skills.value.length, newCount: 0, newSkills: [], updateCount: updateCount.value }
    } finally {
      checkingUpdates.value = false
    }
  }

  async function updateSingle(skill: AgentSkill, force = false): Promise<SkillUpdateResult> {
    if (updatingPaths.value.includes(skill.path)) {
      return { skillId: skill.id, success: false, message: '正在更新中' }
    }

    updatingPaths.value.push(skill.path)
    try {
      const res = await updateAgentSkill(skill.path, force)
      if (res.success) {
        // 更新本地状态
        const idx = skills.value.findIndex(s => s.path === skill.path)
        if (idx !== -1) {
          skills.value[idx].hasUpdate = false
          skills.value[idx].commitsBehind = 0
          skills.value[idx].currentCommit = res.newCommit || skills.value[idx].currentCommit
          skills.value[idx].isDirty = false
          skills.value[idx].updateSummary = undefined
        }
      }
      return res
    } finally {
      updatingPaths.value = updatingPaths.value.filter(p => p !== skill.path)
    }
  }

  async function updateAll(): Promise<SkillUpdateResult[]> {
    const toUpdate = skills.value.filter(s => s.hasUpdate && s.isGitRepo)
    const results: SkillUpdateResult[] = []

    for (const skill of toUpdate) {
      const res = await updateSingle(skill, false)
      results.push(res)
    }

    return results
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

  return {
    skills,
    loading,
    checkingUpdates,
    updatingPaths,
    modalVisible,
    searchQuery,
    selectedCategory,
    lastCheckTime,
    errorMessage,
    updateCount,
    hasUpdates,
    availableCategories,
    categoryCounts,
    filteredSkills,
    scan,
    checkUpdates,
    updateSingle,
    updateAll,
    openFolder,
    openModal,
    closeModal,
  }
})
