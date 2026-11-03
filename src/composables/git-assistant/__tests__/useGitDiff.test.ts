import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useGitDiff } from '../useGitDiff'
import { loadGitFileDiff, loadGitFileHeadDiff } from '@/services/git/git-service'
import type { GitFileStatus } from '@/types/git'

vi.mock('@/composables/useLocale', () => ({ useLocale: () => ({ t: (key: string) => key }) }))
vi.mock('@/services/git/git-service', () => ({
  loadGitFileDiff: vi.fn(),
  loadGitFileHeadDiff: vi.fn(),
}))

const file: GitFileStatus = {
  raw: 'UU translations.json', x: 'U', y: 'U', path: 'translations.json',
  type: 'updated-but-unmerged', staged: true, unstaged: true,
}

describe('Git diff loading', () => {
  beforeEach(() => vi.resetAllMocks())

  it('does not load any diff while both diff views are hidden', async () => {
    const state = useGitDiff(() => 'repo', vi.fn(), () => false)
    await state.loadDiffForFile(file, 'unstaged')
    await state.loadDiffForFile(file, 'head')
    expect(loadGitFileDiff).not.toHaveBeenCalled()
    expect(loadGitFileHeadDiff).not.toHaveBeenCalled()
    expect(state.diffLoading.value).toBe(false)
  })

  it('loads on explicit modal open even when the inline panel is hidden', async () => {
    vi.mocked(loadGitFileDiff).mockResolvedValue({ filePath: file.path, staged: false, diff: 'requested diff' })
    const state = useGitDiff(() => 'repo', vi.fn(), () => false)
    state.showDiff.value = true
    await state.loadDiffForFile(file, 'unstaged')
    expect(state.currentDiff.value).toBe('requested diff')
    expect(loadGitFileDiff).toHaveBeenCalledOnce()
  })

  it('does not restore a pending result after the view is hidden', async () => {
    let finish!: (value: Awaited<ReturnType<typeof loadGitFileDiff>>) => void
    vi.mocked(loadGitFileDiff).mockReturnValue(new Promise(resolve => { finish = resolve }))
    let visible = true
    const state = useGitDiff(() => 'repo', vi.fn(), () => visible)
    const pending = state.loadDiffForFile(file, 'unstaged')
    visible = false
    await state.loadDiffForFile(file, 'unstaged')
    finish({ filePath: file.path, staged: false, diff: 'obsolete diff' })
    await pending
    expect(state.currentDiff.value).toBe('')
    expect(state.diffLoading.value).toBe(false)
  })

  it('keeps the most recent selection when requests finish out of order', async () => {
    let finish!: (value: Awaited<ReturnType<typeof loadGitFileDiff>>) => void
    vi.mocked(loadGitFileDiff)
      .mockReturnValueOnce(new Promise(resolve => { finish = resolve }))
      .mockResolvedValueOnce({ filePath: 'next.json', staged: false, diff: 'latest diff' })
    const state = useGitDiff(() => 'repo', vi.fn())
    const first = state.loadDiffForFile(file, 'unstaged')
    await state.loadDiffForFile({ ...file, path: 'next.json' }, 'unstaged')
    finish({ filePath: file.path, staged: false, diff: 'obsolete diff' })
    await first
    expect(state.currentDiff.value).toBe('latest diff')
  })
})
