import { flushPromises, shallowMount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { expect, it, vi } from 'vitest'
import { i18n } from '@/i18n'
import { openGitDiffWindow } from '@/services/git/git-diff-window'
import type { GitCommitDetail, GitLogEntry } from '@/services/git/git-service'
import { setTauriInvokeHandler } from '@/test/setup'
import GitLogView from '../GitLogView.vue'

const { listenMock } = vi.hoisted(() => ({ listenMock: vi.fn() }))
vi.mock('@tauri-apps/api/event', () => ({ emit: vi.fn(), listen: listenMock }))
vi.mock('@/services/git/git-diff-window', () => ({ openGitDiffWindow: vi.fn() }))

function deferred<T>() {
  let resolve!: (value: T) => void
  const promise = new Promise<T>(done => { resolve = done })
  return { promise, resolve }
}

function entry(hash: string): GitLogEntry {
  return {
    hash, shortHash: hash.slice(0, 7), subject: `Commit ${hash}`,
    authorName: 'Test Author', authorEmail: 'test@example.test', date: '2025-01-01T00:00:00Z',
  }
}

function detail(commit: GitLogEntry): GitCommitDetail {
  return {
    ...commit, body: '', shortStat: '',
    changedFiles: [{ path: `${commit.hash}.ts`, status: 'M', added: 1, removed: 0 }],
  }
}

it('shows the log before its first detail and opens the diff for the displayed commit', async () => {
  const first = entry('aaaaaaaa')
  const second = entry('bbbbbbbb')
  const firstDetail = deferred<GitCommitDetail>()
  const secondDetail = deferred<GitCommitDetail>()
  const repoPath = '/project/test'
  listenMock.mockResolvedValue(vi.fn())
  setTauriInvokeHandler((command, args) => {
    if (command === 'load_git_log') return [first, second]
    if (command === 'load_git_commit_detail') {
      const payload = args?.payload as { hash: string }
      return payload.hash === first.hash ? firstDetail.promise : secondDetail.promise
    }
    throw new Error(`Unexpected command ${command}`)
  })

  const wrapper = shallowMount(GitLogView, { global: { plugins: [createPinia(), i18n] } })
  try {
    const initialize = listenMock.mock.calls.find(([event]) => event === 'git-log-init')?.[1]
    initialize({ payload: { repoPath, filePath: '', branch: 'main' } })
    await flushPromises()
    expect(wrapper.findAll('.git-log-row')).toHaveLength(2)
    expect(wrapper.find('.git-log-file-row').exists()).toBe(false)

    await wrapper.findAll('.git-log-row')[1]!.trigger('click')
    secondDetail.resolve(detail(second))
    await flushPromises()
    firstDetail.resolve(detail(first))
    await flushPromises()

    const fileRow = wrapper.get('.git-log-file-row')
    expect(fileRow.text()).toContain('bbbbbbbb.ts')
    await fileRow.trigger('click')
    expect(openGitDiffWindow).toHaveBeenCalledWith({
      kind: 'commit', repoPath, hash: second.hash, filePath: 'bbbbbbbb.ts',
    })
  } finally {
    wrapper.unmount()
  }
})
