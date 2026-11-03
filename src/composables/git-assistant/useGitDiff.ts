import { ref } from 'vue'
import { useLocale } from '@/composables/useLocale'
import { loadGitFileDiff, loadGitFileHeadDiff } from '@/services/git/git-service'
import type { GitAssistantFileView } from '@/views/git-assistant/git-assistant.types'
import type { GitFileStatus } from '@/types/git'

export function useGitDiff(
  getDisplayRepoPath: () => string,
  setError: (msg: string) => void,
  isInlineDiffVisible: () => boolean = () => true,
) {
  const { t } = useLocale()

  const currentDiff = ref('')
  const diffMode = ref<'head' | 'staged' | 'unstaged'>('unstaged')
  const diffLoading = ref(false)
  const showDiff = ref(false)
  const activeFileRaw = ref<string | null>(null)

  const showLogFileDiff = ref(false)
  const logFileDiff = ref('')
  const logFileDiffLoading = ref(false)
  const logDiffFileView = ref<GitAssistantFileView | null>(null)
  let diffRequestId = 0

  function clearDiff() {
    diffRequestId += 1
    currentDiff.value = ''
    diffLoading.value = false
  }

  async function loadDiffForFile(file: GitFileStatus, mode: 'head' | 'staged' | 'unstaged') {
    const repoPath = getDisplayRepoPath()
    if (!repoPath || (!isInlineDiffVisible() && !showDiff.value)) {
      clearDiff()
      return
    }

    const staged = mode === 'staged'
    if (mode !== 'head' && (staged ? !file.staged : !file.unstaged)) {
      clearDiff()
      return
    }

    const requestId = ++diffRequestId
    diffLoading.value = true
    try {
      const result = mode === 'head'
        ? await loadGitFileHeadDiff(repoPath, file.path)
        : await loadGitFileDiff({ repoPath, filePath: file.path, staged })
      if (requestId === diffRequestId && repoPath === getDisplayRepoPath()) {
        currentDiff.value = result.diff
      }
    } catch (err) {
      if (requestId === diffRequestId && repoPath === getDisplayRepoPath()) {
        console.error(err)
        currentDiff.value = ''
        setError(err instanceof Error ? err.message : t('gitAssistant.errorFallback'))
      }
    } finally {
      if (requestId === diffRequestId) diffLoading.value = false
    }
  }

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
