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

