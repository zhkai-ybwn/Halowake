import { invoke } from '@tauri-apps/api/core'
import type { GitAiAnalysis } from '@/types/git-ai'
import type { AiModelConfig } from '@/types/ai-settings'

export interface GitCommitPromptPreview {
  prompt: string
  trace: GitCommitPromptTrace
}

export interface GitCommitPromptTrace {
  selectedFiles: GitCommitPromptFileTrace[]
  rawChars: number
  cleanedChars: number
  evidenceCount: number
  rules: string[]
}

export interface GitCommitPromptFileTrace {
  path: string
  role: string
  scope: string
  kind: string
  strategy: string
  rawChars: number
  cleanedChars: number
  evidenceCount: number
  skipped: boolean
  reason?: string | null
}

export async function generateGitAiAnalysis(payload: {
  repoPath: string
  branch: string
  status: string[]
  stagedFiles: string[]
  stagedDiff: string
  model: AiModelConfig
}): Promise<GitAiAnalysis> {
  return invoke<GitAiAnalysis>('generate_git_ai_analysis', { payload })
}

export async function buildGitCommitPrompt(payload: {
  repoPath: string
  branch: string
  selectedFiles: string[]
  language?: string
}): Promise<GitCommitPromptPreview> {
  return invoke<GitCommitPromptPreview>('build_git_commit_prompt', { payload })
}

export async function generateGitAiAnalysisFromPrompt(payload: {
  requestId: string
  prompt: string
  model: AiModelConfig
}): Promise<GitAiAnalysis> {
  return invoke<GitAiAnalysis>('generate_git_ai_analysis_from_prompt', { payload })
}

export async function cancelGitAiAnalysis(requestId: string): Promise<boolean> {
  return invoke<boolean>('cancel_git_ai_analysis', { payload: { requestId } })
}
