import type { AiModelConfig } from '@/types/ai-settings'

export type ReviewBudgetMode = 'compact' | 'standard' | 'deep'
export type ReviewFindingStatus = 'open' | 'confirmed' | 'ignored' | 'fixed'

export interface ScoreBreakdownItem { factor: string; delta: number; evidence: string }
export interface ReviewFileRecord {
  path: string
  changeKind: string
  attentionScore: number
  scoreCategories: string[]
  scoreBreakdown: ScoreBreakdownItem[]
  selected: boolean
  reviewStatus: string
  batchId: string | null
  limitation: string | null
}
export interface ReviewFinding {
  id: string
  fingerprint: string
  source: string
  ruleId: string | null
  category: string
  severity: 'critical' | 'major' | 'minor' | 'suggestion'
  confidence: number
  filePath: string
  startLine: number
  endLine: number
  title: string
  problem: string
  impact: string
  triggerScenario: string
  evidence: string
  suggestion: string | null
  verified: boolean
  status: ReviewFindingStatus
  userNote: string | null
}
export interface ReviewOverview {
  critical: number
  major: number
  minor: number
  suggestion: number
  appliedRules: number
  triggeredRules: number
}
export interface AiCallUsage {
  batchId: string
  files: string[]
  inputTokens: number
  outputTokens: number
  estimated: boolean
  durationMs: number
  status: string
  error: string | null
}
export interface ReviewSession {
  id: string
  repoRoot: string
  diffFingerprint: string
  status: string
  phase: string
  progressDone: number
  progressTotal: number
  currentFile: string | null
  budgetMode: string
  modelId: string
  selectedFiles: string[]
  overview: ReviewOverview
  limitations: string[]
  inputTokens: number
  outputTokens: number
  usageEstimated: boolean
  errorMessage: string | null
  createdAt: number
  updatedAt: number
  completedAt: number | null
  isPinned: boolean
  files: ReviewFileRecord[]
  findings: ReviewFinding[]
  aiCalls: AiCallUsage[]
}
export interface ReviewSessionSummary {
  id: string
  repoRoot: string
  status: string
  phase: string
  selectedFileCount: number
  overview: ReviewOverview
  inputTokens: number
  outputTokens: number
  usageEstimated: boolean
  createdAt: number
  updatedAt: number
  isPinned: boolean
}
export interface ReviewRule {
  id: string
  name: string
  description: string | null
  kind: 'deterministic' | 'semantic'
  enabled: boolean
  severity: 'critical' | 'major' | 'minor' | 'suggestion'
  category: string
  includeGlobs: string[]
  excludeGlobs: string[]
  languages: string[]
  definition: Record<string, unknown>
  source: string
  version: number
}
export interface StartReviewPayload {
  repoPath: string
  selectedFiles: string[]
  model: AiModelConfig
  budgetMode: ReviewBudgetMode
  language?: string
}
export interface ReviewProgressEvent {
  sessionId: string
  revision: number
  status: string
  phase: string
  completed: number
  total: number
  currentFile: string | null
}
