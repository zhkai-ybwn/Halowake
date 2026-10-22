import { invoke } from '@tauri-apps/api/core'

export interface CodexScheduleSettings {
  enabled: boolean
  times: string[]
  weekdays: number[]
  checkIntervalSeconds: number
  maxWaitMinutes: number
  accountId: string | null
}

export interface CodexScheduleRun {
  id: string
  scheduledAt: number
  deadline: number
  status: string
  detail: string
  checks: number
  failures: number
  nextCheckAt: number
  requestedAt: number | null
  requestCompleted: boolean
  requestError?: string | null
  requestEvidence?: {
    stage: string
    model: string | null
    replyReceived: boolean
    outputTokens: number | null
    serviceError?: string | null
  }
  updatedAt: number
}

export interface CodexScheduleData {
  settings: CodexScheduleSettings
  runs: CodexScheduleRun[]
}

export function loadCodexSchedule(): Promise<CodexScheduleData> {
  return invoke('load_codex_schedule')
}

export function saveCodexSchedule(settings: CodexScheduleSettings): Promise<CodexScheduleData> {
  return invoke('save_codex_schedule', { settings })
}

export function runCodexWarmupNow(): Promise<CodexScheduleData> {
  return invoke('run_codex_warmup_now')
}
