import { beforeEach, describe, expect, it, vi } from 'vitest'
import { checkoutGitRemoteBranch, switchGitBranch, type GitBranch, type GitSnapshot } from '@/services/git/git-service'
import { useGitBranchActions } from '../useGitBranchActions'

vi.mock('@/services/git/git-service', () => ({
  checkoutGitRemoteBranch: vi.fn(), switchGitBranch: vi.fn(), createGitBranch: vi.fn(), mergeGitBranch: vi.fn(),
}))

const result = { command: 'git checkout', stdout: '', stderr: '', message: '' }

function setup(branches: GitBranch[] = []) {
  const startGitCommand = vi.fn(async () => {})
  const finishGitCommand = vi.fn()
  const failGitCommand = vi.fn()
  const setError = vi.fn()
  const loadSnapshot = vi.fn(async (): Promise<GitSnapshot | undefined> => undefined)
  const state = useGitBranchActions({
    getRepositoryPath: () => '/repo', getCurrentBranchValue: () => 'local:main', getCurrentBranch: () => 'main',
    getBranches: () => branches, startGitCommand, finishGitCommand, failGitCommand, setError, loadSnapshot,
    translate: (key, params) => params ? `${key} ${Object.values(params).join(' ')}` : key,
  })
  return { state, startGitCommand, finishGitCommand, failGitCommand, setError, loadSnapshot }
}

describe('Git branch actions', () => {
  beforeEach(() => vi.resetAllMocks())

  it('waits for the progress listener, then verifies actual branch state when checkout fails', async () => {
    const { state, startGitCommand, failGitCommand, setError, loadSnapshot } = setup()
    let ready!: () => void
    startGitCommand.mockImplementationOnce(() => new Promise(resolve => { ready = resolve }))
    vi.mocked(switchGitBranch).mockRejectedValueOnce('Checkout failed')
    loadSnapshot.mockResolvedValueOnce({ branch: 'main' } as GitSnapshot)
    const pending = state.handleBranchSelection('local:next')
    expect(switchGitBranch).not.toHaveBeenCalled()
    ready()
    await pending
    expect(loadSnapshot).toHaveBeenCalledWith('/repo', true)
    expect(failGitCommand).toHaveBeenCalledWith('Checkout failed', 'gitAssistant.gitCommand.actualBranch main')
    expect(setError).toHaveBeenLastCalledWith('Checkout failed')
  })

  it('keeps Git success distinct from an unverified post-checkout snapshot', async () => {
    const { state, finishGitCommand } = setup()
    vi.mocked(switchGitBranch).mockResolvedValueOnce(result)
    await state.handleBranchSelection('local:next')
    expect(finishGitCommand).toHaveBeenCalledWith(result, '', undefined, 'gitAssistant.gitCommand.branchRefreshFailed')
  })

  it('offers a unique tracking name before changing branches when an unrelated local name exists', async () => {
    const { state } = setup([{ name: 'next', kind: 'local', current: false, upstream: 'other/next' }])
    vi.mocked(checkoutGitRemoteBranch).mockResolvedValueOnce(result)
    await state.handleBranchSelection('remote:origin/next')
    expect(state.remoteCollision.value).toEqual({ remote: 'origin/next', local: 'next' })
    expect(checkoutGitRemoteBranch).not.toHaveBeenCalled()
    state.trackingBranchDraft.value = 'next'
    await state.chooseCollisionAction(false)
    expect(checkoutGitRemoteBranch).not.toHaveBeenCalled()
    state.trackingBranchDraft.value = 'tracking-next'
    await state.chooseCollisionAction(false)
    expect(checkoutGitRemoteBranch).toHaveBeenCalledExactlyOnceWith('/repo', 'origin/next', 'tracking-next')
    expect(switchGitBranch).not.toHaveBeenCalled()
  })
})
