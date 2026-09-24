import { invoke } from '@tauri-apps/api/core'

export interface AgentSkill {
  id: string
  name: string
  description: string
  version?: string
  agentType: string
  agentLabel: string
  associatedAgents?: string[]
  path: string
  isGitRepo: boolean
  gitRemoteUrl?: string
  gitBranch?: string
  currentCommit?: string
  isDirty: boolean
  hasUpdate: boolean
  latestCommit?: string
  latestVersion?: string
  updateSummary?: string
  commitsBehind: number
  lastCheckedAt?: number
  cleanDesc?: string
  displayAgentTag?: string
  searchIndex?: string
}

export interface SkillUpdateResult {
  skillId: string
  success: boolean
  message: string
  previousCommit?: string
  newCommit?: string
}

export async function scanAgentSkills(projectCwd?: string): Promise<AgentSkill[]> {
  return await invoke<AgentSkill[]>('scan_agent_skills', { projectCwd })
}

export async function checkSkillsUpdates(projectCwd?: string): Promise<AgentSkill[]> {
  return await invoke<AgentSkill[]>('check_agent_skills_updates', { projectCwd })
}

export async function updateAgentSkill(skillPath: string, force = false): Promise<SkillUpdateResult> {
  return await invoke<SkillUpdateResult>('update_agent_skill', { skillPath, force })
}

export async function openSkillFolder(path: string): Promise<void> {
  return await invoke<void>('open_skill_folder', { path })
}
