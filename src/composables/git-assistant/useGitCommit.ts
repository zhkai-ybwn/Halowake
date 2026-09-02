import { ref, watch } from 'vue'
import { useLocale } from '@/hooks/useLocale'
import {
  buildGitCommitPrompt,
  cancelGitAiAnalysis,
  generateGitAiAnalysisFromPrompt,
  type GitCommitPromptPreview,
} from '@/services/git/git-ai-service'
import { commitGitChanges } from '@/services/git/git-service'
import { useAiSettingsStore } from '@/stores/ai-settings'
import {
  loadGitCommitHistory,
  saveGitCommitHistory as invokeSaveGitCommitHistory,
  type GitCommitHistoryRecord,
} from '@/services/git/git-history-service'
import type { GitAssistantFileView } from '@/views/git-assistant/git-assistant.types'
import { getRepoDisplayName } from './utils'

const MAX_COMMIT_MESSAGE_HISTORY = 20

export type CommitMessageHistoryEntry = GitCommitHistoryRecord

export function useGitCommit(
  getDisplayRepoPath: () => string,
  getSnapshot: () => { branch: string } | null,
  getSelectedFileViews: () => GitAssistantFileView[],
  getConflictedFiles: () => GitAssistantFileView[],
  getReviewSelectedRaws: () => string[],
  setError: (msg: string) => void,
  startGitCommand: (title: string, phase: string, nextAction?: '' | 'push' | 'pull') => void,
  finishGitCommand: (result: { command: string; stdout: string; stderr: string; message: string; suggestion?: string | null }, nextActionLabel?: string) => void,
  failGitCommand: (err: unknown) => void,
  loadSnapshotByPath: (path: string) => Promise<void>,
  clearReviewSelection: () => void,
) {
  const { t } = useLocale()
  const aiSettings = useAiSettingsStore()

  const commitTitle = ref('')
  const commitBody = ref('')
  const commitLoading = ref(false)
  const aiLoading = ref(false)
  const promptPreview = ref<GitCommitPromptPreview | null>(null)
  const promptDrawerOpen = ref(false)
  const historyDrawerOpen = ref(false)
  const promptGenerationStep = ref('')
  const autoSendPromptToApi = ref(true)
  const commitMessageHistory = ref<CommitMessageHistoryEntry[]>([])
  const commitLanguage = ref<'en' | 'zh'>(
    (localStorage.getItem('lumina.commitLanguage') as 'en' | 'zh') || 'en'
  )

  watch(commitLanguage, (val) => {
    localStorage.setItem('lumina.commitLanguage', val)
  })
  let promptProgressTimers: number[] = []
  let activeAiRequestId = ''
  let aiGenerationCancelled = false

  function createHistoryId() {
    return `commit-message-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`
  }

  async function loadCommitMessageHistory() {
    try {
      const repo = getDisplayRepoPath()
      const entries = await loadGitCommitHistory(repo || undefined, MAX_COMMIT_MESSAGE_HISTORY)
      commitMessageHistory.value = entries
    } catch (err) {
      console.error('Failed to load commit message history from SQLite:', err)
      commitMessageHistory.value = []
    }
  }

  async function saveCommitMessageHistory(source: 'ai' | 'manual') {
    const title = commitTitle.value.trim()
    const body = commitBody.value.trim()
    if (!title && !body) return

    const repo = getDisplayRepoPath()
    const entry: CommitMessageHistoryEntry = {
      id: createHistoryId(),
      repoPath: repo,
      repoName: getRepoDisplayName(repo),
      title,
      body,
      source,
      selectedFileCount: getReviewSelectedRaws().length,
      createdAt: Date.now(),
      expiresAt: null,
    }

    try {
      await invokeSaveGitCommitHistory(entry)
      await loadCommitMessageHistory()
    } catch (err) {
      console.error('Failed to save commit message history to SQLite:', err)
    }
  }

  function restoreCommitMessage(entry: CommitMessageHistoryEntry) {
    commitTitle.value = entry.title
    commitBody.value = entry.body
    historyDrawerOpen.value = false
  }

  function startPromptProgress() {
    stopPromptProgress()
    promptGenerationStep.value = t('gitAssistant.ai.progressReading')
    promptProgressTimers = [
      window.setTimeout(() => {
        promptGenerationStep.value = t('gitAssistant.ai.progressCleaning')
      }, 400),
      window.setTimeout(() => {
        promptGenerationStep.value = t('gitAssistant.ai.progressBuilding')
      }, 1200),
    ]
  }

  function setPromptProgressStep(key: string) {
    for (const timer of promptProgressTimers) {
      window.clearTimeout(timer)
    }
    promptProgressTimers = []
    promptGenerationStep.value = t(key)
  }

  function stopPromptProgress() {
    for (const timer of promptProgressTimers) {
      window.clearTimeout(timer)
    }
    promptProgressTimers = []
    promptGenerationStep.value = ''
  }

  async function handleGenerateAiAnalysis() {
    const snapshot = getSnapshot()
    const displayRepoPath = getDisplayRepoPath()
    if (!snapshot || !displayRepoPath) return

    const conflictedFiles = getConflictedFiles()
    if (conflictedFiles.length) {
      setError(t('gitAssistant.conflict.resolveBeforeCommit'))
      return
    }

    const selectedFiles = getSelectedFileViews().map(file => file.path)

    if (!selectedFiles.length) {
      setError(t('gitAssistant.ai.noSelectedFiles'))
      return
    }

    aiLoading.value = true
    aiGenerationCancelled = false
    activeAiRequestId = `commit-message-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`
    setError('')
    startPromptProgress()

    try {
      promptPreview.value = await buildGitCommitPrompt({
        repoPath: displayRepoPath,
        branch: snapshot.branch,
        selectedFiles,
        language: commitLanguage.value,
      })

      if (aiGenerationCancelled) return

      if (autoSendPromptToApi.value) {
        const model = aiSettings.getModelForTask('commit-message')
        if (!model) {
          throw new Error(t('gitAssistant.ai.noModelConfigured'))
        }
        setPromptProgressStep('gitAssistant.ai.progressCallingApi')
        const result = await generateGitAiAnalysisFromPrompt({
          requestId: activeAiRequestId,
          prompt: promptPreview.value.prompt,
          model,
        })
        if (aiGenerationCancelled) return
        commitTitle.value = result.title
        commitBody.value = result.body
        saveCommitMessageHistory('ai')
      }

      promptDrawerOpen.value = true
    } catch (err) {
      const cancelled = aiGenerationCancelled || (err instanceof Error && err.message.includes('AI_GENERATION_CANCELLED'))
      if (!cancelled) {
        console.error(err)
        setError(err instanceof Error ? err.message : t('gitAssistant.errorFallback'))
      }
    } finally {
      aiLoading.value = false
      activeAiRequestId = ''
      stopPromptProgress()
    }
  }

  async function handleCancelAiAnalysis() {
    if (!aiLoading.value) return
    aiGenerationCancelled = true
    setPromptProgressStep('gitAssistant.ai.progressStopping')
    if (!activeAiRequestId) return
    try {
      await cancelGitAiAnalysis(activeAiRequestId)
    } catch (err) {
      console.error(err)
    }
  }

  async function handleCommit() {
    const displayRepoPath = getDisplayRepoPath()
    if (!displayRepoPath || !commitTitle.value.trim()) return

    const conflictedFiles = getConflictedFiles()
    if (conflictedFiles.length) {
      setError(t('gitAssistant.conflict.resolveBeforeCommit'))
      return
    }

    const selectedFiles = getSelectedFileViews().map(file => file.path)

    if (!selectedFiles.length) {
      setError(t('gitAssistant.ai.noSelectedFiles'))
      return
    }

    commitLoading.value = true
    setError('')
    saveCommitMessageHistory('manual')
    startGitCommand(t('gitAssistant.gitCommand.commitTitle'), t('gitAssistant.gitCommand.committing'), 'push')

    try {
      const result = await commitGitChanges({
        repoPath: displayRepoPath,
        title: commitTitle.value,
        body: commitBody.value,
        selectedFiles,
      })

      commitTitle.value = ''
      commitBody.value = ''
      clearReviewSelection()
      finishGitCommand(result, t('gitAssistant.gitCommand.pushNext'))
      void loadSnapshotByPath(displayRepoPath)
    } catch (err) {
      console.error(err)
      failGitCommand(err)
      void loadSnapshotByPath(displayRepoPath)
    } finally {
      commitLoading.value = false
    }
  }

  return {
    commitTitle,
    commitBody,
    commitLoading,
    aiLoading,
    promptPreview,
    promptDrawerOpen,
    historyDrawerOpen,
    promptGenerationStep,
    autoSendPromptToApi,
    commitMessageHistory,
    commitLanguage,
    loadCommitMessageHistory,
    saveCommitMessageHistory,
    restoreCommitMessage,
    handleGenerateAiAnalysis,
    handleCancelAiAnalysis,
    handleCommit,
  }
}
