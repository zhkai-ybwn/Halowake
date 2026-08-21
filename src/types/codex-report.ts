export type AiToolProvider = 'all' | 'codex' | 'claude' | 'antigravity' | 'opencode'

export interface CodexReportQuery {
  from: string
  to: string
  providers?: string[]
}

export interface CodexProjectInfo {
  name: string
  cwd: string
  sessionCount: number
  lastActiveAt?: string | null
  provider?: string | null
}

export interface CodexReportSession {
  id: string
  provider: AiToolProvider | string
  startedAt: string
  endedAt: string
  cwd: string | null
  projectName: string
  userMessages: string[]
  assistantMessages: string[]
}

export interface InstalledToolInfo {
  provider: string
  name: string
  isInstalled: boolean
  sessionCount: number
}

