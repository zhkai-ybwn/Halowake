import { computed, nextTick, ref } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { useLocale } from '@/composables/useLocale'
import { loadGitBranches, loadGitFileStats, loadGitSnapshot, type GitBranch, type GitSnapshot } from '@/services/git/git-service'
import { ensureGitProjectProfile } from '@/services/git/git-profile-service'
import { useWorkspaceStore } from '@/stores/workspace'
import { parseGitStatusLine } from '@/utils/git-status'
import {
  GIT_RECENT_REPOS_STORAGE_KEY,
  GIT_REPO_STORAGE_KEY,
} from '@/views/git-assistant/git-assistant.config'
import { normalizePath, getRepoDisplayName } from './utils'

const MAX_RECENT_REPOS = 8
const CONFLICT_CODES = new Set(['DD', 'AU', 'UD', 'UA', 'DU', 'AA', 'UU'])

function sameRepository(left: string, right: string) {
  const normalize = (path: string) => {
    const normalized = normalizePath(path).replace(/\/$/, '')
    return /^(?:[a-z]:\/|\/\/)/i.test(normalized) ? normalized.toLowerCase() : normalized
  }
  return normalize(left) === normalize(right)
}

export interface RecentGitRepo {
  path: string
  name: string
  openedAt: number
}

export function useGitSnapshot() {
  const { t } = useLocale()
  const workspaceStore = useWorkspaceStore()

  const loading = ref(false)
  const error = ref('')
  const snapshot = ref<GitSnapshot | null>(null)
  const repoPath = computed(() => snapshot.value?.repoRoot || snapshot.value?.repoPath || '')
  const selectedRaws = ref<string[]>([])
  const branches = ref<GitBranch[] | null>(null)
  const branchListLoading = ref(false)
  const branchListError = ref('')
  const recentRepos = ref<RecentGitRepo[]>([])
  const editingAliasPath = ref<string | null>(null)
  const aliasInputRefs = new Map<string, HTMLInputElement>()
  let snapshotRequestId = 0
  let pendingSnapshotPath = ''
  let branchRequestId = 0
  let branchRequest: Promise<void> | undefined

  const displayRepoPath = repoPath
  const repositoryState = computed(() => snapshot.value?.repositoryState ?? null)

  function describeError(err: unknown) {
    return typeof err === 'string' ? err : err instanceof Error ? err.message : t('gitAssistant.errorFallback')
  }

  function invalidateBranches() {
    branchRequestId += 1
    branchRequest = undefined
    branches.value = null
    branchListLoading.value = false
    branchListError.value = ''
  }

  function ensureBranches() {
    const path = repoPath.value
    if (!path || branches.value) return Promise.resolve()
    if (branchRequest) return branchRequest

    const requestId = ++branchRequestId
    branchListLoading.value = true
    branchListError.value = ''
    branchRequest = loadGitBranches(path).then(result => {
      if (requestId === branchRequestId) branches.value = result
    }).catch(err => {
      if (requestId === branchRequestId) branchListError.value = describeError(err)
    }).finally(() => {
      if (requestId !== branchRequestId) return
      branchRequest = undefined
      branchListLoading.value = false
    })
    return branchRequest
  }

  function loadRecentRepos() {
    try {
      const raw = localStorage.getItem(GIT_RECENT_REPOS_STORAGE_KEY)
      if (!raw) {
        recentRepos.value = []
        return
      }

      const parsed = JSON.parse(raw) as Partial<RecentGitRepo>[]
      recentRepos.value = parsed
        .filter(repo => typeof repo.path === 'string' && repo.path.trim())
        .map(repo => ({
          path: repo.path as string,
          name: typeof repo.name === 'string' && repo.name ? repo.name : getRepoDisplayName(repo.path as string),
          openedAt: typeof repo.openedAt === 'number' ? repo.openedAt : 0,
        }))
        .sort((left, right) => right.openedAt - left.openedAt)
        .slice(0, MAX_RECENT_REPOS)
    } catch (err) {
      console.error(err)
      recentRepos.value = []
    }
  }

  function persistRecentRepos() {
    localStorage.setItem(GIT_RECENT_REPOS_STORAGE_KEY, JSON.stringify(recentRepos.value))
  }

  function rememberRecentRepo(path: string) {
    const normalizedPath = path.trim()
    if (!normalizedPath) return

    const existingRepo = recentRepos.value.find(repo => normalizePath(repo.path).toLowerCase() === normalizePath(normalizedPath).toLowerCase())
    const nextRepo: RecentGitRepo = {
      path: normalizedPath,
      name: existingRepo?.name || getRepoDisplayName(normalizedPath),
      openedAt: Date.now(),
    }
    recentRepos.value = [
      nextRepo,
      ...recentRepos.value.filter(repo => normalizePath(repo.path).toLowerCase() !== normalizePath(normalizedPath).toLowerCase()),
    ].slice(0, MAX_RECENT_REPOS)
    persistRecentRepos()
  }

  const ensuredProfiles = new Map<string, Promise<unknown>>()

  function ensureProjectProfile(path: string) {
    let pending = ensuredProfiles.get(path)
    if (!pending) {
      pending = ensureGitProjectProfile(path).catch(error => {
        ensuredProfiles.delete(path)
        throw error
      })
      ensuredProfiles.set(path, pending)
    }
    return pending
  }

  async function requestSnapshot(path: string, silent = false) {
    if (!path) return

    const requestId = ++snapshotRequestId
    pendingSnapshotPath = path
    if (!silent) loading.value = true
    error.value = ''

    try {
      const result = await loadGitSnapshot(path)
      if (requestId !== snapshotRequestId) return

      const root = result.repoRoot || path
      const previous = snapshot.value
      const sameBranch = sameRepository(repoPath.value, root) && previous?.branch === result.branch
      const statuses = result.status.map(line => line.slice(0, 2)).filter(code => code !== '??' && code !== '!!')
      const loadStats = result.repositoryState.hasCommits && statuses.length && !statuses.some(code => CONFLICT_CODES.has(code))
      if (loadStats && sameBranch && previous && !result.fileStats.length) {
        result.fileStats = previous.fileStats
      }
      if (!sameBranch) selectedRaws.value = []
      else if (selectedRaws.value.length) {
        const rawByPath = new Map(result.status.map(raw => [parseGitStatusLine(raw).path, raw]))
        selectedRaws.value = selectedRaws.value.flatMap(raw => {
          const currentRaw = rawByPath.get(parseGitStatusLine(raw).path)
          return currentRaw ? [currentRaw] : []
        })
      }
      snapshot.value = { ...result, repoRoot: root }
      invalidateBranches()
      workspaceStore.setCurrentProjectPath(root)
      localStorage.setItem(GIT_REPO_STORAGE_KEY, root)
      rememberRecentRepo(root)

      void ensureProjectProfile(root).catch(profileErr => {
        console.error(profileErr)
        if (requestId === snapshotRequestId) {
          error.value = describeError(profileErr)
        }
      })

      if (loadStats) {
        void loadGitFileStats(root).then(stats => {
          if (requestId === snapshotRequestId && snapshot.value) snapshot.value.fileStats = stats
        }).catch(statsError => console.error(statsError))
      }
      return snapshot.value
    } catch (err) {
      console.error(err)
      if (requestId === snapshotRequestId) {
        error.value = sameRepository(path, repoPath.value)
          ? describeError(err)
          : t('gitAssistant.repo.openFailed', { path, reason: describeError(err) })
      }
    } finally {
      if (requestId === snapshotRequestId) {
        pendingSnapshotPath = ''
        loading.value = false
      }
    }
  }

  async function loadSnapshotByPath(path: string, silent = false) {
    await refreshCurrentSnapshot(path, silent)
  }

  async function refreshCurrentSnapshot(path: string, silent = false) {
    if (!sameRepository(path, repoPath.value) || (pendingSnapshotPath && !sameRepository(path, pendingSnapshotPath))) return
    return requestSnapshot(path, silent)
  }

  async function selectRepository(path: string) {
    await requestSnapshot(path)
  }

  async function handleSelectDirectory() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: t('gitAssistant.repo.chooseDirectory'),
        defaultPath: repoPath.value || undefined,
      })

      if (!selected || Array.isArray(selected)) return

      await selectRepository(selected)
    } catch (err) {
      console.error(err)
      error.value = describeError(err)
    }
  }

  async function handleRefresh() {
    if (!repoPath.value) return
    await loadSnapshotByPath(repoPath.value)
  }

  async function handleSwitchRecentRepo(path: string) {
    if (!path || sameRepository(path, displayRepoPath.value)) return
    await selectRepository(path)
  }

  function renameRecentRepo(path: string, name: string) {
    const normalizedPath = normalizePath(path).toLowerCase()
    const repo = recentRepos.value.find(repo => normalizePath(repo.path).toLowerCase() === normalizedPath)
    if (repo) {
      repo.name = name
    }
  }

  function setAliasInputRef(el: HTMLInputElement | null, path: string) {
    if (el) {
      aliasInputRefs.set(path, el)
    } else {
      aliasInputRefs.delete(path)
    }
  }

  function startEditAlias(path: string) {
    editingAliasPath.value = path
    nextTick(() => {
      const input = aliasInputRefs.get(path)
      if (input) {
        input.focus()
        input.select()
      }
    })
  }

  function finishEditAlias(repo: { path: string; name: string }) {
    if (!repo.name.trim()) {
      repo.name = repo.path.split(/[/\\]/).pop() || repo.path
    }
    persistRecentRepos()
    editingAliasPath.value = null
  }

  function cancelEditAlias() {
    editingAliasPath.value = null
  }

  function removeRecentRepo(path: string) {
    const normalizedPath = normalizePath(path).toLowerCase()
    recentRepos.value = recentRepos.value.filter(repo => normalizePath(repo.path).toLowerCase() !== normalizedPath)
    persistRecentRepos()
  }

  return {
    loading,
    error,
    repoPath,
    snapshot,
    recentRepos,
    editingAliasPath,
    displayRepoPath,
    repositoryState,
    selectedRaws,
    branches,
    ensureBranches,
    branchListLoading,
    branchListError,
    loadRecentRepos,
    selectRepository,
    loadSnapshotByPath,
    refreshCurrentSnapshot,
    handleSelectDirectory,
    handleRefresh,
    handleSwitchRecentRepo,
    renameRecentRepo,
    setAliasInputRef,
    startEditAlias,
    finishEditAlias,
    cancelEditAlias,
    removeRecentRepo,
    normalizePath,
  }
}
