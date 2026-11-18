import { onScopeDispose, ref } from 'vue'
import { useLocale } from '@/composables/useLocale'
import { loadGitFileDiff, loadGitFileHeadDiff } from '@/services/git/git-service'
import type { GitAssistantFileView } from '@/views/git-assistant/git-assistant.types'
import type { GitFileStatus } from '@/types/git'

const DIFF_PREVIEW_DELAY_MS = 50
type DiffMode = 'head' | 'staged' | 'unstaged'
interface DiffPreviewRequest {
  id: number
  repoPath: string
  path: string
  mode: DiffMode
  revision?: unknown
  immediate: boolean
  promise: Promise<void>
  resolve: () => void
}

export function useGitDiff(
  getDisplayRepoPath: () => string,
  setError: (msg: string) => void,
  isInlineDiffVisible: () => boolean = () => true,
) {
  const { t } = useLocale()

  const currentDiff = ref('')
  const diffMode = ref<DiffMode>('unstaged')
  const diffLoading = ref(false)
  const showDiff = ref(false)
  const activeFileRaw = ref<string | null>(null)

  const showLogFileDiff = ref(false)
  const logFileDiff = ref('')
  const logFileDiffLoading = ref(false)
  const logDiffFileView = ref<GitAssistantFileView | null>(null)
  let diffRequestId = 0
  let running = false
  let timer: ReturnType<typeof setTimeout> | undefined
  let pending: DiffPreviewRequest | undefined
  let latest: DiffPreviewRequest | undefined

  function clearPending() {
    clearTimeout(timer)
    timer = undefined
    pending?.resolve()
    pending = undefined
  }

  function clearDiff() {
    diffRequestId += 1
    latest = undefined
    clearPending()
    currentDiff.value = ''
    diffLoading.value = false
  }

  function loadDiffForFile(file: GitFileStatus, mode: DiffMode, options?: { revision?: unknown; immediate?: boolean }): Promise<void> {
    const repoPath = getDisplayRepoPath()
    const staged = mode === 'staged'
    if (!repoPath || (!isInlineDiffVisible() && !showDiff.value)
      || (mode !== 'head' && (staged ? !file.staged : !file.unstaged))) {
      clearDiff()
      return Promise.resolve()
    }

    const { revision, immediate = showDiff.value } = options ?? {}
    if (latest?.repoPath === repoPath && latest.path === file.path && latest.mode === mode && latest.revision === revision) {
      if (immediate && latest === pending) {
        latest.immediate = true
        if (!running) {
          clearTimeout(timer)
          void runPending()
        }
      }
      return latest.promise
    }

    clearPending()
    currentDiff.value = ''
    diffLoading.value = true
    let resolve!: () => void
    const promise = new Promise<void>(done => { resolve = done })
    latest = pending = { id: ++diffRequestId, repoPath, path: file.path, mode, revision, immediate, promise, resolve }
    if (!running) dispatchPending()
    return promise
  }

  function dispatchPending() {
    if (!pending) return
    if (pending.immediate) void runPending()
    else timer = setTimeout(runPending, DIFF_PREVIEW_DELAY_MS)
  }

  async function runPending() {
    const request = pending
    pending = undefined
    timer = undefined
    if (!request) return

    const { id, repoPath, path, mode, resolve } = request
    const isCurrent = () => id === diffRequestId && repoPath === getDisplayRepoPath()
    running = true
    try {
      if (!isCurrent()) return
      const result = mode === 'head' ? await loadGitFileHeadDiff(repoPath, path)
        : await loadGitFileDiff({ repoPath, filePath: path, staged: mode === 'staged' })
      if (isCurrent()) currentDiff.value = result.diff
    } catch (err) {
      if (isCurrent()) {
        latest = undefined
        console.error(err)
        currentDiff.value = ''
        setError(typeof err === 'string' ? err : err instanceof Error ? err.message : t('gitAssistant.errorFallback'))
      }
    } finally {
      running = false
      if (id === diffRequestId) diffLoading.value = false
      resolve()
      dispatchPending()
    }
  }

  onScopeDispose(clearDiff, true)

  function closeDiff() {
    showDiff.value = false
    clearDiff()
  }

  return {
    currentDiff,
    diffMode,
    diffLoading,
    showDiff,
    activeFileRaw,
    showLogFileDiff,
    logFileDiff,
    logFileDiffLoading,
    logDiffFileView,
    clearDiff,
    loadDiffForFile,
    closeDiff,
  }
}
