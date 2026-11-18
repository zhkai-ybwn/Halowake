import { beforeEach, describe, expect, it, vi } from 'vitest'
import { loadGitBranches, loadGitFileStats, loadGitSnapshot, type GitSnapshot } from '@/services/git/git-service'
import { useGitSnapshot } from '../useGitSnapshot'

const setCurrentProjectPath = vi.hoisted(() => vi.fn())
vi.mock('@/stores/workspace', () => ({ useWorkspaceStore: () => ({ setCurrentProjectPath }) }))
vi.mock('@/composables/useLocale', () => ({
  useLocale: () => ({ t: (key: string, params?: { path: string; reason: string }) => params ? `${params.path} ${params.reason}` : key }),
}))
vi.mock('@/services/git/git-profile-service', () => ({ ensureGitProjectProfile: vi.fn(async () => ({})) }))
vi.mock('@/services/git/git-service', () => ({ loadGitSnapshot: vi.fn(), loadGitFileStats: vi.fn(), loadGitBranches: vi.fn() }))

function snapshot(repoRoot: string, branch = 'main', status: string[] = []): GitSnapshot {
  return {
    repoRoot, repoPath: repoRoot, branch, status, stagedFiles: [], stagedDiff: '', fileStats: [], branches: [],
    repositoryState: {
      hasCommits: true, upstreamGone: false, ahead: 0, behind: 0, mergeInProgress: false, rebaseInProgress: false,
    },
  }
}

describe('Git repository snapshot', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    localStorage.clear()
    vi.mocked(loadGitFileStats).mockResolvedValue([])
  })

  it('keeps the last accepted repository when the selected directory is invalid', async () => {
    vi.mocked(loadGitSnapshot).mockResolvedValueOnce(snapshot('/valid')).mockRejectedValueOnce('Not a repository')
    const state = useGitSnapshot()
    await state.selectRepository('/valid')
    await state.selectRepository('/invalid')
    expect(state.displayRepoPath.value).toBe('/valid')
    expect(state.snapshot.value?.repoRoot).toBe('/valid')
    expect(state.error.value).toBe('/invalid Not a repository')
    expect(setCurrentProjectPath).toHaveBeenCalledExactlyOnceWith('/valid')
  })

  it('keeps rows during refresh and remaps only valid selected paths', async () => {
    vi.mocked(loadGitSnapshot).mockResolvedValueOnce(snapshot('/repo', 'main', [' M a.ts', ' M b.ts']))
    const state = useGitSnapshot()
    await state.selectRepository('/repo')
    state.selectedRaws.value = [' M a.ts', ' M b.ts']
    let finish!: (value: GitSnapshot) => void
    vi.mocked(loadGitSnapshot).mockImplementationOnce(() => new Promise(resolve => { finish = resolve }))
    const pending = state.handleRefresh()
    expect(state.snapshot.value?.status).toEqual([' M a.ts', ' M b.ts'])
    expect(state.selectedRaws.value).toEqual([' M a.ts', ' M b.ts'])
    finish(snapshot('/repo', 'main', ['M  a.ts', '?? c.ts']))
    await pending
    expect(state.selectedRaws.value).toEqual(['M  a.ts'])
    vi.mocked(loadGitSnapshot).mockResolvedValueOnce(snapshot('/repo', 'other', ['M  a.ts']))
    await state.handleRefresh()
    expect(state.selectedRaws.value).toEqual([])
  })

  it('loads branches on demand, shares the pending request, and drops stale branch results on switch', async () => {
    vi.mocked(loadGitSnapshot).mockResolvedValueOnce(snapshot('/one')).mockResolvedValueOnce(snapshot('/two'))
    let finish!: (value: Awaited<ReturnType<typeof loadGitBranches>>) => void
    vi.mocked(loadGitBranches).mockImplementationOnce(() => new Promise(resolve => { finish = resolve }))
    const state = useGitSnapshot()
    await state.selectRepository('/one')
    expect(loadGitBranches).not.toHaveBeenCalled()
    const branchesForFirst = state.ensureBranches()
    expect(state.ensureBranches()).toBe(branchesForFirst)
    await state.selectRepository('/two')
    finish([{ name: 'first', kind: 'local', current: true }])
    await branchesForFirst
    expect(state.branches.value).toBeNull()
    vi.mocked(loadGitBranches).mockResolvedValueOnce([{ name: 'second', kind: 'local', current: true }])
    await state.ensureBranches()
    expect(loadGitBranches).toHaveBeenLastCalledWith('/two')
    expect(state.branches.value?.[0].name).toBe('second')
  })
})
