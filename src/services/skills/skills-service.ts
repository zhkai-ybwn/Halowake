import { invoke } from '@tauri-apps/api/core'
import type {
  AgentSkill,
  InstallSkillArgs,
  InstallSkillResult,
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

export async function installAgentSkill(args: InstallSkillArgs): Promise<InstallSkillResult> {
  return await invoke<InstallSkillResult>('install_agent_skill', { args })
}

export async function uninstallAgentSkill(skillPath: string): Promise<boolean> {
  return await invoke<boolean>('uninstall_agent_skill', { skillPath })
}

