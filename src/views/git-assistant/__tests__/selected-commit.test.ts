import { flushPromises, shallowMount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { describe, expect, it, vi } from 'vitest'
import { i18n } from '@/i18n'
import { GIT_REPO_STORAGE_KEY } from '@/constants/git'
import { useAiSettingsStore } from '@/stores/ai-settings'
import { invokeMock, setTauriInvokeHandler } from '@/test/setup'
import GitAssistantView from '../GitAssistantView.vue'
import GitCommandDialog from '../components/GitCommandDialog.vue'

vi.mock('naive-ui', async importOriginal => ({
  ...await importOriginal<typeof import('naive-ui')>(),
  useDialog: () => ({}),
}))
const { listenMock } = vi.hoisted(() => ({ listenMock: vi.fn() }))
vi.mock('@tauri-apps/api/event', () => ({ listen: listenMock }))

describe('Git Assistant selected-file commit', () => {
  it('previews one file while generating and committing only the checked file', async () => {
    let finishListening!: () => void
    listenMock.mockImplementation(() => new Promise<() => void>(resolve => {
      finishListening = () => resolve(() => {})
    }))
    const repoPath = '/projects/example'
    localStorage.setItem(GIT_REPO_STORAGE_KEY, repoPath)
    const pinia = createPinia()
    useAiSettingsStore(pinia).$patch({
      defaultModelId: 'local',
      models: [{ id: 'local', name: 'Local', provider: 'ollama', model: 'local', baseUrl: 'http://localhost', enabled: true }],
    })
    setTauriInvokeHandler(command => {
      switch (command) {
        case 'load_git_snapshot':
          return {
            repoPath, repoRoot: repoPath, branch: 'main', branches: [],
            status: [' M checked.ts', 'M  staged.ts'], stagedFiles: ['staged.ts'],
            stagedDiff: '', fileStats: [], repositoryState: { hasCommits: true },
          }
        case 'load_git_file_stats': return []
        case 'load_git_file_diff': return { diff: 'file diff' }
        case 'build_git_commit_prompt':
          return { prompt: 'selected changes', trace: { selectedFiles: [], rules: [], evidenceCount: 0 } }
        case 'generate_git_ai_analysis_from_prompt': return { title: 'Update checked file', body: 'One change' }
        case 'commit_git_changes': return { command: 'git commit', stdout: '', stderr: '', message: 'Committed' }
        case 'load_git_commit_history': return []
        case 'save_git_commit_history':
        case 'ensure_git_project_profile': return null
        default: throw new Error(`Unexpected command: ${command}`)
      }
    })
    const wrapper = shallowMount(GitAssistantView, {
      global: { plugins: [pinia, i18n], stubs: { GitChangeExplorer: false, GitCommitAssistant: false } },
    })
    try {
      await flushPromises()
      const rows = wrapper.findAll('.file-row')
      expect(rows).toHaveLength(2)
      const generate = wrapper.get('.ai-gen-btn--initial')
      expect(generate.attributes('disabled')).toBeDefined()
      await rows[0]!.get('input[type="checkbox"]').setValue(true)
      await rows[1]!.trigger('click')
      await flushPromises()
      expect(invokeMock).toHaveBeenCalledWith('load_git_file_diff', {
        payload: { repoPath, filePath: 'staged.ts', staged: true },
      })
      await generate.trigger('click')
      await flushPromises()
      expect(invokeMock).toHaveBeenCalledWith('build_git_commit_prompt', {
        payload: expect.objectContaining({ selectedFiles: ['checked.ts'] }),
      })
      expect(wrapper.get<HTMLInputElement>('.field-input').element.value).toBe('Update checked file')
      await wrapper.get('.commit-submit-btn').trigger('click')
      await flushPromises()
      expect(invokeMock).not.toHaveBeenCalledWith('commit_git_changes', expect.anything())
      expect(wrapper.getComponent(GitCommandDialog).props('running')).toBe(true)
      finishListening()
      await flushPromises()
      expect(invokeMock).toHaveBeenCalledWith('commit_git_changes', {
        payload: { repoPath, title: 'Update checked file', body: 'One change', selectedFiles: ['checked.ts'] },
      })
      expect(wrapper.get('.commit-submit-btn').attributes('disabled')).toBeDefined()
    } finally {
      wrapper.unmount()
    }
  })
})

describe('Git command progress', () => {
  it('shows activity while progress is unknown and switches to actual percent when available', async () => {
    const wrapper = shallowMount(GitCommandDialog, {
      global: { plugins: [i18n] },
      props: {
        visible: true, title: 'Commit', repoPath: '/projects/example', phase: 'Validating files',
        running: true, success: null, command: '', activeCommand: '', stdout: '', stderr: '',
        message: '', suggestion: '', startedAt: Date.now(), finishedAt: null,
        progressPercent: null, progressPhase: '', transfer: '', nextActionLabel: '', closeLabel: 'Close',
      },
    })
    try {
      const bar = wrapper.get('[role="progressbar"]')
      expect(bar.attributes('aria-valuenow')).toBeUndefined()
      expect(bar.get('.progress-bar').classes()).toContain('indeterminate')
      expect(wrapper.get('[role="status"]').text()).toBe('Validating files')

      await wrapper.setProps({ progressPhase: 'Receiving objects', progressPercent: 42 })
      expect(bar.attributes('aria-valuenow')).toBe('42')
      expect(bar.get('.progress-bar').classes()).not.toContain('indeterminate')
      expect(wrapper.get('[role="status"]').text()).toBe('Receiving objects')
    } finally {
      wrapper.unmount()
    }
  })
})
