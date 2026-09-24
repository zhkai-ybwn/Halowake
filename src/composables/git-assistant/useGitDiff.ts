import { ref } from 'vue'
import { useLocale } from '@/composables/useLocale'
import { loadGitFileDiff, loadGitFileHeadDiff } from '@/services/git/git-service'
import type { GitAssistantFileView } from '@/views/git-assistant/git-assistant.types'
import type { GitFileStatus } from '@/types/git'

export function useGitDiff(getDisplayRepoPath: () => string, setError: (msg: string) => void) {
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

  function clearDiff() {
    currentDiff.value = ''
  }

  async function loadDiffForFile(file: GitFileStatus, mode: 'head' | 'staged' | 'unstaged') {
    const repoPath = getDisplayRepoPath()
    if (!repoPath) return

    if (mode === 'head') {
      diffLoading.value = true
      try {
        const result = await loadGitFileHeadDiff(repoPath, file.path)
        currentDiff.value = result.diff
      } catch (err) {
        console.error(err)
        currentDiff.value = ''
        setError(err instanceof Error ? err.message : t('gitAssistant.errorFallback'))
      } finally {
        diffLoading.value = false
      }
      return
    }

    const staged = mode === 'staged'
    if (staged && !file.staged) {
      currentDiff.value = ''
      return
    }

    if (!staged && !file.unstaged) {
      currentDiff.value = ''
      return
    }

    diffLoading.value = true
    try {
      const result = await loadGitFileDiff({
        repoPath,
        filePath: file.path,
        staged,
      })
      currentDiff.value = result.diff
    } catch (err) {
      console.error(err)
      currentDiff.value = ''
      setError(err instanceof Error ? err.message : t('gitAssistant.errorFallback'))
    } finally {
      diffLoading.value = false
    }
  }

  function closeDiff() {
    showDiff.value = false
    currentDiff.value = ''
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
