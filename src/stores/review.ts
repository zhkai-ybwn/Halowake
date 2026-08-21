import { defineStore } from 'pinia'
import {
  cancelLocalCodeReview, deleteReviewRule, getLocalCodeReview, listLocalCodeReviews,
  listReviewRules, saveReviewRule, startLocalCodeReview, updateReviewFinding,
} from '@/services/review-service'
import type {
  ReviewFindingStatus, ReviewRule, ReviewSession, ReviewSessionSummary, StartReviewPayload,
} from '@/types/review'

export const useReviewStore = defineStore('code-review', {
  state: () => ({
    activeSession: null as ReviewSession | null,
    activeSessionId: '' as string,
    lastRevision: 0,
    history: [] as ReviewSessionSummary[],
    rules: [] as ReviewRule[],
    loading: false,
    error: '',
  }),
  getters: {
    running: state => state.activeSession?.status === 'running' || (state.loading && Boolean(state.activeSessionId)),
  },
  actions: {
    async start(payload: StartReviewPayload) {
      this.loading = true
      this.error = ''
      try {
        this.activeSession = null
        this.activeSessionId = ''
        this.activeSessionId = await startLocalCodeReview(payload)
        this.lastRevision = 0
        await this.refreshActive()
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error)
        throw error
      } finally { this.loading = false }
    },
    async refreshActive() {
      if (!this.activeSessionId) return
      this.activeSession = await getLocalCodeReview(this.activeSessionId)
    },
    async open(sessionId: string) {
      this.loading = true
      this.error = ''
      try {
        this.activeSessionId = sessionId
        this.activeSession = null
        this.lastRevision = 0
        await this.refreshActive()
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error)
        throw error
      } finally { this.loading = false }
    },
    async loadHistory(repoRoot: string) { this.history = repoRoot ? await listLocalCodeReviews(repoRoot) : [] },
    async loadRules() { this.rules = await listReviewRules() },
    clearActive() {
      this.activeSession = null
      this.activeSessionId = ''
      this.lastRevision = 0
      this.error = ''
    },
    async cancel() {
      if (!this.activeSessionId) return
      await cancelLocalCodeReview(this.activeSessionId)
      await this.refreshActive()
    },
    async setFindingStatus(findingId: string, status: ReviewFindingStatus, userNote?: string) {
      if (!this.activeSessionId) return
      this.activeSession = await updateReviewFinding(this.activeSessionId, findingId, status, userNote)
    },
    async saveRule(rule: ReviewRule) { this.rules = await saveReviewRule(rule) },
    async deleteRule(id: string) { this.rules = await deleteReviewRule(id) },
  },
})
