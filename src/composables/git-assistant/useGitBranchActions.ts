import { ref } from 'vue'
import {
  checkoutGitRemoteBranch,
  createGitBranch,
  mergeGitBranch,
  switchGitBranch,
  type GitSnapshot,
} from '@/services/git/git-service'

interface UseGitBranchActionsOptions {
  getRepositoryPath: () => string
  getCurrentBranchValue: () => string | null
  snapshot: { value: GitSnapshot | null }
  setError: (message: string) => void
  translate: (key: string) => string
  loadSnapshot: (path: string, silent?: boolean) => Promise<void>
  startGitCommand: (title: string, phase: string) => void
  finishGitCommand: (result: { command: string; stdout: string; stderr: string; message: string; suggestion?: string | null }) => void
  failGitCommand: (error: unknown) => void
}

export function useGitBranchActions(options: UseGitBranchActionsOptions) {
  const branchLoading = ref(false)
  const branchSelectorOpen = ref(false)
  const branchSelectionValue = ref<string | null>(null)
  const newBranchDraft = ref('')
  const mergeDialogOpen = ref(false)
  const mergeLoading = ref(false)
  const mergeSourceValue = ref<string | null>(null)
  const mergeMode = ref<'default' | 'no-ff'>('default')

  function openBranchSelector() {
    branchSelectionValue.value = options.getCurrentBranchValue()
    newBranchDraft.value = ''
    branchSelectorOpen.value = true
  }

  async function handleCreateBranch() {
    const branch = newBranchDraft.value.trim()
    const repoPath = options.getRepositoryPath()
    if (!repoPath || !branch) return
    branchSelectorOpen.value = false
    await runBranchAction(options.translate('gitAssistant.repo.createBranch'), branch, () => createGitBranch(repoPath, branch))
    newBranchDraft.value = ''
  }

  function openMergeDialog() {
    mergeSourceValue.value = null
    mergeMode.value = 'default'
    mergeDialogOpen.value = true
  }

  async function handleMergeBranch() {
    const repoPath = options.getRepositoryPath()
    if (!repoPath || !mergeSourceValue.value) return
    mergeLoading.value = true
    options.setError('')
    options.startGitCommand(options.translate('gitAssistant.repo.mergeBranch'), 'Merging branch')
    try {
      const result = await mergeGitBranch(repoPath, mergeSourceValue.value, mergeMode.value === 'no-ff')
      options.finishGitCommand(result)
      mergeDialogOpen.value = false
      await options.loadSnapshot(repoPath, true)
    } catch (error) {
      console.error(error)
      options.failGitCommand(error)
      options.setError(error instanceof Error ? error.message : options.translate('gitAssistant.errorFallback'))
      await options.loadSnapshot(repoPath, true)
    } finally {
      mergeLoading.value = false
    }
  }

  async function handleBranchSelection(value: string) {
    const [kind, ...nameParts] = value.split(':')
    const branch = nameParts.join(':')
    const repoPath = options.getRepositoryPath()
    if (!repoPath || !branch) return
    branchSelectorOpen.value = false
    if (kind === 'local') {
      await runBranchAction('Switching branch', branch, () => switchGitBranch(repoPath, branch))
      return
    }
    if (kind === 'remote') {
      const localBranch = branch.split('/').slice(1).join('/')
      if (!localBranch) return
      await runBranchAction('Checking out remote branch', localBranch, () => checkoutGitRemoteBranch(repoPath, branch, localBranch))
    }
  }

  async function runBranchAction(phase: string, nextBranch: string, action: () => ReturnType<typeof switchGitBranch>) {
    const repoPath = options.getRepositoryPath()
    branchLoading.value = true
    options.setError('')
    options.startGitCommand(options.translate('gitAssistant.repo.branch'), phase)
    try {
      const result = await action()
      options.finishGitCommand(result)
      if (options.snapshot.value) {
        options.snapshot.value = {
          ...options.snapshot.value,
          branch: nextBranch,
          branches: (options.snapshot.value.branches ?? []).map(branch => ({ ...branch, current: branch.name === nextBranch && branch.kind === 'local' })),
        }
      }
      void options.loadSnapshot(repoPath, true)
    } catch (error) {
      console.error(error)
      options.failGitCommand(error)
      options.setError(error instanceof Error ? error.message : options.translate('gitAssistant.errorFallback'))
    } finally {
      branchLoading.value = false
    }
  }

  return {
    branchLoading, branchSelectorOpen, branchSelectionValue, newBranchDraft,
    mergeDialogOpen, mergeLoading, mergeSourceValue, mergeMode,
    openBranchSelector, openMergeDialog, handleCreateBranch, handleMergeBranch, handleBranchSelection,
  }
}
