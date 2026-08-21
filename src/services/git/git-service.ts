import { invoke } from '@tauri-apps/api/core'

export interface GitSnapshot {
  repoPath: string
  repoRoot: string
  branch: string
  repositoryState: GitRepositoryState
  status: string[]
  stagedFiles: string[]
  stagedDiff: string
  fileStats: GitFileStat[]
  branches: GitBranch[]
}

export interface GitRepositoryState {
  hasCommits: boolean
  remoteName?: string | null
  remoteUrl?: string | null
  upstream?: string | null
  upstreamGone: boolean
  ahead: number
  behind: number
  mergeInProgress: boolean
  rebaseInProgress: boolean
}

export interface GitFileStat {
  path: string
  added: number | null
  removed: number | null
}

export interface GitReviewAttention {
  path: string
  score: number
  categories: string[]
  scoreBreakdown: GitReviewScoreBreakdown[]
  eligible: boolean
  skipped: boolean
}

export interface GitReviewScoreBreakdown {
  factor: string
  delta: number
  evidence: string
}

export interface GitReviewAttentionResult {
  files: GitReviewAttention[]
}

export async function scoreGitReviewFiles(repoPath: string, selectedFiles: string[]): Promise<GitReviewAttentionResult> {
  return await invoke<GitReviewAttentionResult>('score_git_review_files', {
    payload: { repoPath, selectedFiles },
  })
}

export async function loadGitSnapshot(repoPath: string): Promise<GitSnapshot> {
  return await invoke<GitSnapshot>('load_git_snapshot', { repoPath })
}

export interface GitFileDiffResponse {
  filePath: string
  staged: boolean
  diff: string
}

export interface GitBranch {
  name: string
  kind: 'local' | 'remote'
  current: boolean
  upstream?: string | null
  upstreamStatus?: string | null
}

export async function loadGitFileDiff(payload: {
  repoPath: string
  filePath: string
  staged: boolean
  fullContext?: boolean
}): Promise<GitFileDiffResponse> {
  return await invoke<GitFileDiffResponse>('load_git_file_diff', { payload })
}

export async function loadGitFileHeadDiff(repoPath: string, filePath: string, fullContext = false): Promise<GitFileDiffResponse> {
  return await invoke<GitFileDiffResponse>('load_git_file_head_diff', { payload: { repoPath, filePath, fullContext } })
}

export async function commitGitChanges(payload: {
  repoPath: string
  title: string
  body: string
  selectedFiles: string[]
}): Promise<GitCommandResult> {
  return await invoke<GitCommandResult>('commit_git_changes', { payload })
}

export interface GitCommandResult {
  command: string
  message: string
  stdout: string
  stderr: string
  suggestion?: string | null
}

export type GitSyncRecommendedAction = 'push' | 'pull' | 'resolveDivergence' | 'configureRemote' | 'publishBranch' | 'none'

export interface GitSyncStatus extends GitCommandResult {
  state: GitRepositoryState
  recommendedAction: GitSyncRecommendedAction
}

export interface GitLogEntry {
  hash: string
  shortHash: string
  authorName: string
  authorEmail: string
  date: string
  subject: string
}

export interface GitCommitChangedFile {
  status: string
  path: string
  originalPath?: string | null
  added: number | null
  removed: number | null
}

export interface GitCommitDetail extends GitLogEntry {
  body: string
  shortStat: string
  changedFiles: GitCommitChangedFile[]
}

export interface GitCommitFileDiffResponse {
  hash: string
  filePath: string
  diff: string
}

export async function fetchGitChanges(repoPath: string): Promise<GitCommandResult> {
  return await invoke<GitCommandResult>('fetch_git_changes', { payload: { repoPath } })
}

export async function syncGitStatus(repoPath: string): Promise<GitSyncStatus> {
  return await invoke<GitSyncStatus>('sync_git_status', { payload: { repoPath } })
}

export async function pushGitChanges(repoPath: string): Promise<GitCommandResult> {
  return await invoke<GitCommandResult>('push_git_changes', { payload: { repoPath } })
}

export async function pullGitChanges(repoPath: string): Promise<GitCommandResult> {
  return await invoke<GitCommandResult>('pull_git_changes', { payload: { repoPath } })
}

export async function rebaseGitChanges(repoPath: string): Promise<GitCommandResult> {
  return await invoke<GitCommandResult>('rebase_git_changes', { payload: { repoPath } })
}

export async function configureGitOrigin(repoPath: string, remoteUrl: string): Promise<GitCommandResult> {
  return await invoke<GitCommandResult>('configure_git_origin', { payload: { repoPath, remoteUrl } })
}

export async function repairGitUpstream(repoPath: string): Promise<GitCommandResult> {
  return await invoke<GitCommandResult>('repair_git_upstream', { payload: { repoPath } })
}

export async function openGitFileExternal(repoPath: string, filePath: string): Promise<GitCommandResult> {
  return await invoke<GitCommandResult>('open_git_file_external', { payload: { repoPath, filePath } })
}

export async function markGitFilesResolved(repoPath: string, filePaths: string[]): Promise<GitCommandResult> {
  return await invoke<GitCommandResult>('mark_git_files_resolved', { payload: { repoPath, filePaths } })
}

export async function revertGitFile(repoPath: string, filePath: string): Promise<GitCommandResult> {
  return await invoke<GitCommandResult>('revert_git_file', { payload: { repoPath, filePath } })
}

export async function stageGitFiles(repoPath: string, filePaths: string[]): Promise<GitCommandResult> {
  return await invoke<GitCommandResult>('stage_git_files', { payload: { repoPath, filePaths } })
}

export async function unstageGitFiles(repoPath: string, filePaths: string[]): Promise<GitCommandResult> {
  return await invoke<GitCommandResult>('unstage_git_files', { payload: { repoPath, filePaths } })
}

export async function loadGitBranches(repoPath: string): Promise<GitBranch[]> {
  return await invoke<GitBranch[]>('load_git_branches', { repoPath })
}

export async function createGitBranch(repoPath: string, branch: string): Promise<GitCommandResult> {
  return await invoke<GitCommandResult>('create_git_branch', { payload: { repoPath, branch } })
}

export async function switchGitBranch(repoPath: string, branch: string): Promise<GitCommandResult> {
  return await invoke<GitCommandResult>('switch_git_branch', { payload: { repoPath, branch } })
}

export async function checkoutGitRemoteBranch(repoPath: string, remoteBranch: string, localBranch: string): Promise<GitCommandResult> {
  return await invoke<GitCommandResult>('checkout_git_remote_branch', { payload: { repoPath, remoteBranch, localBranch } })
}

export async function mergeGitBranch(repoPath: string, sourceBranch: string, noFastForward: boolean): Promise<GitCommandResult> {
  return await invoke<GitCommandResult>('merge_git_branch', { payload: { repoPath, sourceBranch, noFastForward } })
}

export async function deleteGitBranch(repoPath: string, branch: string): Promise<GitCommandResult> {
  return await invoke<GitCommandResult>('delete_git_branch', { payload: { repoPath, branch } })
}

export async function setGitBranchUpstream(repoPath: string, branch: string, upstream?: string): Promise<GitCommandResult> {
  return await invoke<GitCommandResult>('set_git_branch_upstream', { payload: { repoPath, branch, upstream: upstream || null } })
}

export async function initGitRepository(repoPath: string): Promise<GitCommandResult> {
  return await invoke<GitCommandResult>('init_git_repository', { repoPath })
}

export async function cloneGitRepository(remoteUrl: string, destinationPath: string): Promise<GitCommandResult> {
  return await invoke<GitCommandResult>('clone_git_repository', { payload: { remoteUrl, destinationPath } })
}

export async function abortGitMerge(repoPath: string): Promise<GitCommandResult> {
  return await invoke<GitCommandResult>('abort_git_merge', { payload: { repoPath } })
}

export async function continueGitMerge(repoPath: string): Promise<GitCommandResult> {
  return await invoke<GitCommandResult>('continue_git_merge', { payload: { repoPath } })
}

export async function continueGitRebase(repoPath: string): Promise<GitCommandResult> {
  return await invoke<GitCommandResult>('continue_git_rebase', { payload: { repoPath } })
}

export async function abortGitRebase(repoPath: string): Promise<GitCommandResult> {
  return await invoke<GitCommandResult>('abort_git_rebase', { payload: { repoPath } })
}

export async function loadGitLog(repoPath: string, filePath?: string): Promise<GitLogEntry[]> {
  return await invoke<GitLogEntry[]>('load_git_log', { payload: { repoPath, filePath: filePath || null } })
}

export async function loadGitCommitDetail(repoPath: string, hash: string): Promise<GitCommitDetail> {
  return await invoke<GitCommitDetail>('load_git_commit_detail', { payload: { repoPath, hash } })
}

export async function loadGitCommitFileDiff(repoPath: string, hash: string, filePath: string, fullContext = false): Promise<GitCommitFileDiffResponse> {
  return await invoke<GitCommitFileDiffResponse>('load_git_commit_file_diff', { payload: { repoPath, hash, filePath, fullContext } })
}
