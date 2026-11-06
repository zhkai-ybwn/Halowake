import { invoke } from '@tauri-apps/api/core'
import type { CodexProjectInfo, CodexReportQuery, CodexReportSession, InstalledToolInfo } from '@/types/codex-report'

interface ReportLoad<T> { data: T; warnings: string[] }
type WarningHandler = (warnings: string[]) => void

async function loadReportData<T>(command: string, args: Record<string, unknown> | undefined, onWarnings?: WarningHandler): Promise<T> {
  const result = await invoke<ReportLoad<T>>(command, args)
  onWarnings?.(result.warnings)
  return result.data
}

export function loadCodexProjects(onWarnings?: WarningHandler) {
  return loadReportData<CodexProjectInfo[]>('load_codex_projects', undefined, onWarnings)
}

export function loadCodexReportSessions(query: CodexReportQuery, onWarnings?: WarningHandler) {
  return loadReportData<CodexReportSession[]>('load_codex_report_sessions', { query }, onWarnings)
}

export function detectInstalledAiTools(onWarnings?: WarningHandler) {
  return loadReportData<InstalledToolInfo[]>('detect_installed_ai_tools', undefined, onWarnings)
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
