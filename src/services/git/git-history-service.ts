import { invoke } from '@tauri-apps/api/core'
import { GIT_COMMIT_MESSAGE_HISTORY_STORAGE_KEY } from '@/views/git-assistant/git-assistant.config'

export interface GitCommitHistoryRecord {
  id: string
  repoPath: string
  repoName: string
  title: string
  body: string
  source: 'ai' | 'manual'
  selectedFileCount: number
  createdAt: number
  expiresAt?: number | null
}

export async function loadGitCommitHistory(
  repoPath?: string,
  limit?: number,
): Promise<GitCommitHistoryRecord[]> {
  await migrateLegacyCommitHistory()
  return await invoke<GitCommitHistoryRecord[]>('load_git_commit_history', {
    repoPath: repoPath || null,
    limit: limit || null,
  })
}

export async function saveGitCommitHistory(
  entry: GitCommitHistoryRecord,
): Promise<void> {
  await invoke('save_git_commit_history', { entry })
}

export async function clearGitCommitHistory(
  repoPath?: string,
): Promise<void> {
  await invoke('clear_git_commit_history', {
    repoPath: repoPath || null,
  })
}

function stringHash(str: string): string {
  let hash = 0
  for (let i = 0; i < str.length; i++) {
    hash = ((hash << 5) - hash) + str.charCodeAt(i)
    hash |= 0
  }
  return Math.abs(hash).toString(36)
}

let migrated = false
export async function migrateLegacyCommitHistory(): Promise<void> {
  if (migrated) return
  try {
    const raw = localStorage.getItem(GIT_COMMIT_MESSAGE_HISTORY_STORAGE_KEY)
    if (!raw) {
      migrated = true
      return
    }
    const parsed = JSON.parse(raw) as Partial<GitCommitHistoryRecord>[]
    if (Array.isArray(parsed) && parsed.length) {
      for (const item of parsed) {
        if (item && item.title) {
          const createdAt = typeof item.createdAt === 'number' ? item.createdAt : Date.now()
          const deterministicId = item.id || `legacy-${createdAt}-${stringHash(`${item.repoPath || ''}:${item.title}`)}`
          await invoke('save_git_commit_history', {
            entry: {
              id: deterministicId,
              repoPath: item.repoPath || '',
              repoName: item.repoName || '',
              title: item.title,
              body: item.body || '',
              source: item.source === 'manual' ? 'manual' : 'ai',
              selectedFileCount: typeof item.selectedFileCount === 'number' ? item.selectedFileCount : 0,
              createdAt,
              expiresAt: null,
            },
          })
        }
      }
    }
    localStorage.removeItem(GIT_COMMIT_MESSAGE_HISTORY_STORAGE_KEY)
    migrated = true
  } catch (err) {
    console.error('Migrate legacy commit history failed:', err)
  }
}
