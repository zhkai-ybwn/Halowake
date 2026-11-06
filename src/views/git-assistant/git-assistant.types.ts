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
}

export interface GitAssistantSelectionState {
  activeFileRaw: string | null
  selectedRaws: string[]
}

export interface GitAssistantAiState {
  commitLoading: boolean
  commitResult: GitAiAnalysis | null
}
