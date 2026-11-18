import { computed, ref } from 'vue'
import {
  checkoutGitRemoteBranch,
  createGitBranch,
  mergeGitBranch,
  switchGitBranch,
  type GitBranch,
  type GitCommandResult,
  type GitSnapshot,
} from '@/services/git/git-service'

interface UseGitBranchActionsOptions {
  getRepositoryPath: () => string
  getCurrentBranchValue: () => string | null
  getCurrentBranch: () => string
  getBranches: () => GitBranch[]
  setError: (message: string) => void
  translate: (key: string, params?: Record<string, string>) => string
  loadSnapshot: (path: string, silent?: boolean) => Promise<GitSnapshot | undefined>
  startGitCommand: (title: string, phase: string, nextAction?: '', detail?: string) => Promise<void>
  finishGitCommand: (result: GitCommandResult, nextActionLabel?: string, phase?: string, resultDetail?: string) => void
  failGitCommand: (error: unknown, resultDetail?: string) => void
}

export function useGitBranchActions(options: UseGitBranchActionsOptions) {
  const branchLoading = ref(false)
  const branchSelectorOpen = ref(false)
  const branchSelectionValue = ref<string | null>(null)
  const newBranchDraft = ref('')
  const remoteCollision = ref<{ remote: string; local: string } | null>(null)
  const trackingBranchDraft = ref('')
  const trackingNameTaken = computed(() => {
    const name = trackingBranchDraft.value.trim()
    return options.getBranches().some(branch => branch.kind === 'local' && branch.name === name)
  })
  const mergeDialogOpen = ref(false)
  const mergeLoading = ref(false)
  const mergeSourceValue = ref<string | null>(null)
  const mergeMode = ref<'default' | 'no-ff'>('default')

  function openBranchSelector() {
    branchSelectionValue.value = options.getCurrentBranchValue()
    newBranchDraft.value = ''
    remoteCollision.value = null
    trackingBranchDraft.value = ''
    branchSelectorOpen.value = true
  }

  async function handleCreateBranch() {
    const branch = newBranchDraft.value.trim()
    const repoPath = options.getRepositoryPath()
    if (!repoPath || !branch || branchLoading.value) return
    branchSelectorOpen.value = false
    await runBranchAction(repoPath, 'createBranch', branch, () => createGitBranch(repoPath, branch))
    newBranchDraft.value = ''
  }

  function openMergeDialog() {
    mergeSourceValue.value = null
    mergeMode.value = 'default'
    mergeDialogOpen.value = true
  }

  async function handleMergeBranch() {
    const repoPath = options.getRepositoryPath()
    const source = mergeSourceValue.value
    if (!repoPath || !source || mergeLoading.value) return
    mergeLoading.value = true
    mergeDialogOpen.value = false
    options.setError('')
    try {
      await options.startGitCommand(options.translate('gitAssistant.repo.mergeBranch'), options.translate('gitAssistant.gitCommand.merging'))
      const result = await mergeGitBranch(repoPath, source, mergeMode.value === 'no-ff')
      options.finishGitCommand(result)
      await options.loadSnapshot(repoPath, true)
    } catch (error) {
      console.error(error)
      options.failGitCommand(error)
      await options.loadSnapshot(repoPath, true)
      if (repoPath === options.getRepositoryPath()) options.setError(describeError(error))
    } finally {
      mergeLoading.value = false
    }
  }

  async function handleBranchSelection(value: string) {
    const [kind, ...nameParts] = value.split(':')
    const branch = nameParts.join(':')
    const repoPath = options.getRepositoryPath()
    if (!repoPath || !branch || branchLoading.value) return
    remoteCollision.value = null
    if (kind === 'local') {
      if (options.getCurrentBranch() !== branch) await switchLocalBranch(repoPath, branch)
      return
    }
    if (kind === 'remote') {
      const localBranch = branch.split('/').slice(1).join('/')
      if (!localBranch) return
      const existing = options.getBranches().find(ref => ref.kind === 'local' && ref.name === localBranch)
      if (existing && existing.upstream !== branch && existing.upstream !== `refs/remotes/${branch}`) {
        remoteCollision.value = { remote: branch, local: localBranch }
        trackingBranchDraft.value = ''
        return
      }
      await checkoutRemoteBranch(repoPath, branch, localBranch)
    }
  }

  async function chooseCollisionAction(useExisting: boolean) {
    const repoPath = options.getRepositoryPath()
    const collision = remoteCollision.value
    if (!repoPath || !collision || branchLoading.value) return
    if (useExisting) {
      if (options.getCurrentBranch() === collision.local) branchSelectorOpen.value = false
      else await switchLocalBranch(repoPath, collision.local)
    } else if (trackingBranchDraft.value.trim() && !trackingNameTaken.value) {
      await checkoutRemoteBranch(repoPath, collision.remote, trackingBranchDraft.value.trim())
    }
  }

  function switchLocalBranch(repoPath: string, branch: string) {
    return runBranchAction(repoPath, 'switchBranch', branch, () => switchGitBranch(repoPath, branch))
  }

  function checkoutRemoteBranch(repoPath: string, remote: string, branch: string) {
    return runBranchAction(repoPath, 'checkoutRemoteBranch', branch, () => checkoutGitRemoteBranch(repoPath, remote, branch))
  }

  function describeError(error: unknown) {
    return typeof error === 'string' ? error : error instanceof Error ? error.message : options.translate('gitAssistant.errorFallback')
  }

  function describeBranchState(snapshot: GitSnapshot | undefined) {
    return snapshot
      ? options.translate('gitAssistant.gitCommand.actualBranch', { branch: snapshot.branch || options.translate('gitAssistant.repo.branchEmpty') })
      : options.translate('gitAssistant.gitCommand.branchRefreshFailed')
  }

  async function runBranchAction(
    repoPath: string,
    operation: 'createBranch' | 'switchBranch' | 'checkoutRemoteBranch',
    nextBranch: string,
    action: () => Promise<GitCommandResult>,
  ) {
    branchLoading.value = true
    branchSelectorOpen.value = false
    options.setError('')
    try {
      const detail = options.translate('gitAssistant.gitCommand.branchTransition', {
        from: options.getCurrentBranch() || options.translate('gitAssistant.repo.branchEmpty'),
        to: nextBranch,
      })
      await options.startGitCommand(options.translate(`gitAssistant.repo.${operation}`), options.translate('gitAssistant.gitCommand.preparingIndex'), '', detail)
      const result = await action()
      const snapshot = await options.loadSnapshot(repoPath, true)
      options.finishGitCommand(result, '', undefined, describeBranchState(snapshot))
    } catch (error) {
      console.error(error)
      const snapshot = await options.loadSnapshot(repoPath, true)
      options.failGitCommand(error, describeBranchState(snapshot))
      if (repoPath === options.getRepositoryPath()) options.setError(describeError(error))
    } finally {
      branchLoading.value = false
    }
  }

  return {
    branchLoading, branchSelectorOpen, branchSelectionValue, newBranchDraft, remoteCollision, trackingBranchDraft, trackingNameTaken,
    mergeDialogOpen, mergeLoading, mergeSourceValue, mergeMode,
    openBranchSelector, openMergeDialog, handleCreateBranch, handleMergeBranch, handleBranchSelection, chooseCollisionAction,
  }
}
