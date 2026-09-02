import { computed, ref, watch } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { scoreGitReviewFiles } from '@/services/git/git-service'
import { listenLocalCodeReview } from '@/services/review-service'
import { useReviewStore } from '@/stores/review'
import type { AiModelConfig } from '@/types/ai-settings'
import type { ReviewBudgetMode } from '@/types/review'
import type { GitAssistantFileView } from '@/views/git-assistant/git-assistant.types'
import { normalizePath } from './utils'

interface ReviewScore {
  score: number
  categories: string[]
  scoreBreakdown: Array<{ factor: string; delta: number; evidence: string }>
  eligible: boolean
  skipped: boolean
}

interface UseGitReviewOptions {
  getRepositoryPath: () => string
  getRepositoryRoot: () => string
  getFiles: () => GitAssistantFileView[]
  getSnapshotStatus: () => string[] | undefined
  getModel: () => AiModelConfig | undefined
  reviewSelectedRaws: { value: string[] }
  onError: (message: string) => void
  openFileDiff: (raw: string) => Promise<void>
  errorFallback: () => string
}

export function useGitReview(options: UseGitReviewOptions) {
  const reviewStore = useReviewStore()
  const reviewScores = ref(new Map<string, ReviewScore>())
  const reviewScoring = ref(false)
  const reviewPanelOpen = ref(false)
  const reviewPanelRevision = ref(0)
  const reviewScoreProgress = ref({ completed: 0, total: 0, phase: '', filePath: '' })
  const reviewModel = computed(() => options.getModel())
  const selectedFileViews = computed(() => options.getFiles().filter(file => options.reviewSelectedRaws.value.includes(file.raw)))
  let reviewScoreRequestId = 0
  let unlistenReviewScoreProgress: UnlistenFn | null = null
  let unlistenLocalReview: UnlistenFn | null = null

  function toggleReviewSelection(payload: { raw: string; checked: boolean }) {
    if (payload.checked) {
      if (!options.reviewSelectedRaws.value.includes(payload.raw)) options.reviewSelectedRaws.value = [...options.reviewSelectedRaws.value, payload.raw]
      return
    }
    options.reviewSelectedRaws.value = options.reviewSelectedRaws.value.filter(raw => raw !== payload.raw)
  }

  function setReviewSelection(raws: string[]) {
    const valid = new Set(options.getFiles().map(file => file.raw))
    options.reviewSelectedRaws.value = [...new Set(raws.filter(raw => valid.has(raw)))]
  }

  async function loadReviewScores() {
    const repoPath = options.getRepositoryPath()
    const files = options.getFiles().map(file => file.path)
    const requestId = ++reviewScoreRequestId
    reviewScores.value = new Map()
    if (!repoPath || !files.length) return
    reviewScoring.value = true
    reviewScoreProgress.value = { completed: 0, total: files.length, phase: 'preparing', filePath: '' }
    try {
      const result = await scoreGitReviewFiles(repoPath, files)
      if (requestId !== reviewScoreRequestId) return
      reviewScores.value = new Map(result.files.map(file => [normalizePath(file.path), file]))
      reviewScoreProgress.value = { completed: files.length, total: files.length, phase: 'complete', filePath: '' }
    } catch (error) {
      console.error(error)
      if (requestId === reviewScoreRequestId) options.onError(error instanceof Error ? error.message : options.errorFallback())
    } finally {
      if (requestId === reviewScoreRequestId) reviewScoring.value = false
    }
  }

  async function openReviewPanel() {
    const preferNewReview = selectedFileViews.value.length > 0
    await Promise.allSettled([reviewStore.loadHistory(options.getRepositoryRoot()), reviewStore.loadRules()])
    if (!preferNewReview) {
      const latest = reviewStore.history[0]
      if (latest) await reviewStore.open(latest.id).catch(error => console.error(error))
      else reviewStore.clearActive()
    }
    reviewPanelRevision.value += 1
    reviewPanelOpen.value = true
  }

  async function startCodeReview(budgetMode: ReviewBudgetMode) {
    const model = reviewModel.value
    const repoPath = options.getRepositoryPath()
    if (!model || !repoPath || !selectedFileViews.value.length) return
    try {
      await reviewStore.start({ repoPath, selectedFiles: selectedFileViews.value.map(file => file.path), model: { ...model }, budgetMode, language: 'zh-CN' })
    } catch (error) { console.error(error) }
  }

  async function openReviewFindingFile(path: string) {
    const file = options.getFiles().find(item => normalizePath(item.path) === normalizePath(path))
    if (file) await options.openFileDiff(file.raw)
  }

  async function startReviewListeners() {
    unlistenLocalReview = await listenLocalCodeReview(async event => {
      if (event.sessionId !== reviewStore.activeSessionId || event.revision <= reviewStore.lastRevision) return
      reviewStore.lastRevision = event.revision
      try {
        await reviewStore.refreshActive()
        if (event.status !== 'running') await reviewStore.loadHistory(options.getRepositoryRoot())
      } catch (error) { console.error(error) }
    })
    unlistenReviewScoreProgress = await listen<{ repoPath: string; completed: number; total: number; phase: string; filePath?: string | null }>('git-review-score-progress', event => {
      if (!reviewScoring.value) return
      if (normalizePath(event.payload.repoPath).toLowerCase() !== normalizePath(options.getRepositoryPath()).toLowerCase()) return
      reviewScoreProgress.value = { completed: event.payload.completed, total: event.payload.total, phase: event.payload.phase, filePath: event.payload.filePath ?? '' }
    })
  }

  function stopReviewListeners() {
    unlistenLocalReview?.()
    unlistenReviewScoreProgress?.()
  }

  watch(options.getSnapshotStatus, () => {
    reviewScoreRequestId += 1
    reviewScores.value = new Map()
    reviewScoring.value = false
    reviewScoreProgress.value = { completed: 0, total: 0, phase: '', filePath: '' }
  }, { immediate: true })

  return {
    reviewStore, reviewScores, reviewScoring, reviewPanelOpen, reviewPanelRevision, reviewScoreProgress, reviewModel, selectedFileViews,
    toggleReviewSelection, setReviewSelection, loadReviewScores, openReviewPanel, startCodeReview, openReviewFindingFile,
    startReviewListeners, stopReviewListeners,
  }
}
