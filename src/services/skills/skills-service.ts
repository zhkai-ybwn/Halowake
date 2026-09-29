import { Channel, invoke } from '@tauri-apps/api/core'
import type {
  AgentSkill,
  InstallSkillArgs,
  InstallSkillResult,
  InstallSkillProgress,
  SkillInstallHistoryEntry,
  SkillUpdateResult,
} from '@/types/skills'

export type { AgentSkill, InstallSkillArgs, InstallSkillResult, SkillUpdateResult }

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

export async function installAgentSkill(args: InstallSkillArgs, onProgress: (progress: InstallSkillProgress) => void): Promise<InstallSkillResult> {
  const onProgressChannel = new Channel<InstallSkillProgress>(onProgress)
  return await invoke<InstallSkillResult>('install_agent_skill', { args, onProgress: onProgressChannel })
}

export async function listSkillInstallHistory(): Promise<SkillInstallHistoryEntry[]> {
  return await invoke<SkillInstallHistoryEntry[]>('list_skill_install_history')
}

export async function uninstallAgentSkill(skillPath: string, projectCwd?: string): Promise<boolean> {
  return await invoke<boolean>('uninstall_agent_skill', { skillPath, projectCwd })
}
