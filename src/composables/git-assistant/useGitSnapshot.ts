import { computed, nextTick, ref } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { useLocale } from '@/hooks/useLocale'
import { loadGitSnapshot, type GitSnapshot } from '@/services/git/git-service'
import { ensureGitProjectProfile } from '@/services/git/git-profile-service'
import {
  GIT_RECENT_REPOS_STORAGE_KEY,
  GIT_REPO_STORAGE_KEY,
} from '@/views/git-assistant/git-assistant.config'
import { normalizePath, getRepoDisplayName } from './utils'

const MAX_RECENT_REPOS = 8

export interface RecentGitRepo {
  path: string
  name: string
  openedAt: number
}

export function useGitSnapshot() {
  const { t } = useLocale()

  const loading = ref(false)
  const error = ref('')
  const repoPath = ref('')
  const snapshot = ref<GitSnapshot | null>(null)
  const recentRepos = ref<RecentGitRepo[]>([])
  const editingAliasPath = ref<string | null>(null)
  const aliasInputRefs = new Map<string, HTMLInputElement>()
  let snapshotRequestId = 0

  const displayRepoPath = computed(() => repoPath.value || snapshot.value?.repoPath || '')
  const repositoryState = computed(() => snapshot.value?.repositoryState ?? null)

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

  async function loadSnapshotByPath(path: string, silent = false) {
    if (!path) return

    const requestId = ++snapshotRequestId
    if (!silent) loading.value = true
    error.value = ''

    try {
      const result = await loadGitSnapshot(path)
      if (requestId !== snapshotRequestId) return

      snapshot.value = result
      reviewSelectedRaws.value = []
      localStorage.setItem(GIT_REPO_STORAGE_KEY, path)
      rememberRecentRepo(result.repoRoot || path)

      try {
        await ensureGitProjectProfile(result.repoRoot || path)
      } catch (profileErr) {
        console.error(profileErr)
        if (requestId === snapshotRequestId) {
          error.value = profileErr instanceof Error ? profileErr.message : t('gitAssistant.errorFallback')
        }
      }
    } catch (err) {
      console.error(err)
      if (requestId === snapshotRequestId) {
        error.value = err instanceof Error ? err.message : t('gitAssistant.errorFallback')
      }
    } finally {
      if (!silent && requestId === snapshotRequestId) {
        loading.value = false
      }
    }
  }

  // reviewSelectedRaws is used by loadSnapshotByPath to clear selection on repo switch
  const reviewSelectedRaws = ref<string[]>([])

  async function handleSelectDirectory() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: t('gitAssistant.repo.chooseDirectory'),
        defaultPath: repoPath.value || undefined,
      })

      if (!selected || Array.isArray(selected)) return

      repoPath.value = selected
      await loadSnapshotByPath(selected)
    } catch (err) {
      console.error(err)
      error.value = err instanceof Error ? err.message : t('gitAssistant.errorFallback')
    }
  }

  async function handleRefresh() {
    if (!repoPath.value) return
    await loadSnapshotByPath(repoPath.value)
  }

  async function handleSwitchRecentRepo(path: string) {
    if (!path || path === displayRepoPath.value) return
    repoPath.value = path
    snapshot.value = null
    reviewSelectedRaws.value = []
    await loadSnapshotByPath(path)
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
    reviewSelectedRaws,
    loadRecentRepos,
    loadSnapshotByPath,
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
