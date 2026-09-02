import { ref } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { cloneGitRepository, initGitRepository } from '@/services/git/git-service'

interface UseGitRepositorySetupOptions {
  repoPath: { value: string }
  setError: (message: string) => void
  translate: (key: string) => string
  loadSnapshot: (path: string) => Promise<void>
  finishGitCommand: (result: { command: string; stdout: string; stderr: string; message: string; suggestion?: string | null }) => void
}

export function useGitRepositorySetup(options: UseGitRepositorySetupOptions) {
  const repositorySetupOpen = ref(false)
  const repositorySetupMode = ref<'init' | 'clone'>('init')
  const repositoryLoading = ref(false)
  const repositoryPathDraft = ref('')
  const cloneUrlDraft = ref('')

  function openRepositorySetup(mode: 'init' | 'clone') {
    repositorySetupMode.value = mode
    repositoryPathDraft.value = ''
    cloneUrlDraft.value = ''
    repositorySetupOpen.value = true
  }

  async function pickRepositoryTarget() {
    const selected = await open({
      directory: true,
      multiple: false,
      title: options.translate('gitAssistant.repo.chooseDirectory'),
    })
    if (selected && !Array.isArray(selected)) repositoryPathDraft.value = selected
  }

  async function handleRepositorySetup() {
    if (!repositoryPathDraft.value) return
    repositoryLoading.value = true
    options.setError('')
    try {
      const result = repositorySetupMode.value === 'clone'
        ? await cloneGitRepository(cloneUrlDraft.value.trim(), repositoryPathDraft.value)
        : await initGitRepository(repositoryPathDraft.value)
      options.finishGitCommand(result)
      repositorySetupOpen.value = false
      options.repoPath.value = repositoryPathDraft.value
      await options.loadSnapshot(repositoryPathDraft.value)
    } catch (error) {
      console.error(error)
      options.setError(error instanceof Error ? error.message : options.translate('gitAssistant.errorFallback'))
    } finally {
      repositoryLoading.value = false
    }
  }

  return {
    repositorySetupOpen,
    repositorySetupMode,
    repositoryLoading,
    repositoryPathDraft,
    cloneUrlDraft,
    openRepositorySetup,
    pickRepositoryTarget,
    handleRepositorySetup,
  }
}
