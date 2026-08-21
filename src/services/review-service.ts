import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type {
  ReviewFindingStatus, ReviewProgressEvent, ReviewRule, ReviewSession,
  ReviewSessionSummary, StartReviewPayload,
} from '@/types/review'

export const startLocalCodeReview = (payload: StartReviewPayload) =>
  invoke<string>('start_local_code_review', { payload })
export const getLocalCodeReview = (sessionId: string) =>
  invoke<ReviewSession>('get_local_code_review', { sessionId })
export const listLocalCodeReviews = (repoRoot: string, limit = 20) =>
  invoke<ReviewSessionSummary[]>('list_local_code_reviews', { repoRoot, limit })
export const cancelLocalCodeReview = (sessionId: string) =>
  invoke<boolean>('cancel_local_code_review', { sessionId })
export const updateReviewFinding = (sessionId: string, findingId: string, status: ReviewFindingStatus, userNote?: string) =>
  invoke<ReviewSession>('update_review_finding', { payload: { sessionId, findingId, status, userNote: userNote || null } })
export const listReviewRules = () => invoke<ReviewRule[]>('list_review_rules')
export const saveReviewRule = (rule: ReviewRule) => invoke<ReviewRule[]>('save_review_rule', { rule })
export const deleteReviewRule = (id: string) => invoke<ReviewRule[]>('delete_review_rule', { id })
export const listenLocalCodeReview = (handler: (event: ReviewProgressEvent) => void): Promise<UnlistenFn> =>
  listen<ReviewProgressEvent>('local-code-review-updated', event => handler(event.payload))
