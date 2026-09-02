import {
  stageGitFiles,
  unstageGitFiles,
  type GitCommandResult,
} from '@/services/git/git-service'
import type { GitAssistantFileView } from '@/views/git-assistant/git-assistant.types'

interface UseGitStagingOptions {
  getRepositoryPath: () => string
  getFiles: () => GitAssistantFileView[]
  setError: (message: string) => void
  translate: (key: string) => string
  loadSnapshot: (path: string) => Promise<void>
  startGitCommand: (title: string, phase: string) => void
  finishGitCommand: (result: GitCommandResult) => void
  failGitCommand: (error: unknown) => void
}

export function useGitStaging(options: UseGitStagingOptions) {
  async function handleStageFiles(raws: string[], stage: boolean) {
    const repoPath = options.getRepositoryPath()
    if (!repoPath) return
    const paths = raws
      .map(raw => options.getFiles().find(file => file.raw === raw))
      .filter((file): file is GitAssistantFileView => Boolean(file && (stage ? file.unstaged : file.staged)))
      .map(file => file.path)
    if (!paths.length) return

    options.setError('')
    options.startGitCommand(
      stage ? options.translate('gitAssistant.files.stageVisible') : options.translate('gitAssistant.files.unstageVisible'),
      stage ? 'Staging files' : 'Unstaging files',
    )
    try {
      const result = stage
        ? await stageGitFiles(repoPath, paths)
        : await unstageGitFiles(repoPath, paths)
      options.finishGitCommand(result)
      await options.loadSnapshot(repoPath)
    } catch (error) {
      console.error(error)
      options.failGitCommand(error)
    }
  }

  return { handleStageFiles }
}
