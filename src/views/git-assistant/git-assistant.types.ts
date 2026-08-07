import type { GitFileStatus, GitRepoInfoResponse } from '@/types/git'
import type { GitAiAnalysis } from '@/types/git-ai'

export type GitAssistantStatusFilter =
  | 'all'
  | 'modified'
  | 'added'
  | 'deleted'
  | 'renamed'
  | 'copied'
  | 'untracked'
  | 'updated-but-unmerged'
  | 'staged'
  | 'unstaged'
  | 'versioned'
  | 'recommended'

export interface GitAssistantSummary {
  total: number
  modified: number
  added: number
  deleted: number
  renamed: number
  copied: number
  untracked: number
  conflicted: number
  staged: number
  unstaged: number
}

export interface GitAssistantFileView extends GitFileStatus {
  fileName: string
  directory: string
  extension: string
  addedLines: number | null
  removedLines: number | null
  score: number | null
  scoreCategories: string[]
  recommended: boolean
}

export interface GitAssistantFileGroup {
  key: string
  label: string
  files: GitAssistantFileView[]
}

export interface GitAssistantWorkspaceState {
  repoPath: string
  snapshot: GitRepoInfoResponse | null
  loading: boolean
  error: string
}

export interface GitAssistantExplorerState {
  keyword: string
  statusFilter: GitAssistantStatusFilter
  recommendedOnly: boolean
}

export interface GitAssistantSelectionState {
  activeFileRaw: string | null
  reviewSelectedRaws: string[]
}

export interface GitAssistantAiState {
  commitLoading: boolean
  commitResult: GitAiAnalysis | null
}
