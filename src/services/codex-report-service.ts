import { invoke } from '@tauri-apps/api/core'
import type { CodexProjectInfo, CodexReportQuery, CodexReportSession, InstalledToolInfo } from '@/types/codex-report'

export function loadCodexProjects() {
  return invoke<CodexProjectInfo[]>('load_codex_projects')
}

export function loadCodexReportSessions(query: CodexReportQuery) {
  return invoke<CodexReportSession[]>('load_codex_report_sessions', { query })
}

export function detectInstalledAiTools() {
  return invoke<InstalledToolInfo[]>('detect_installed_ai_tools')
}

export interface CodexReportPromptTemplate {
  id: string
  name: string
  content: string
  isBuiltin: boolean
  sortOrder: number
  createdAt: number
  updatedAt: number
}

export function loadCodexReportTemplates(): Promise<CodexReportPromptTemplate[]> {
  return invoke<CodexReportPromptTemplate[]>('load_codex_report_templates')
}

export function saveCodexReportTemplate(template: CodexReportPromptTemplate): Promise<void> {
  return invoke('save_codex_report_template', { template })
}

export function deleteCodexReportTemplate(id: string): Promise<void> {
  return invoke('delete_codex_report_template', { id })
}

export function resetBuiltinCodexReportTemplates(): Promise<CodexReportPromptTemplate[]> {
  return invoke<CodexReportPromptTemplate[]>('reset_builtin_codex_report_templates')
}
