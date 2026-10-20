export type SdlcStage = 'requirement' | 'design' | 'coding' | 'testing' | 'delivery'

export type SkillBadgeType = 'verified' | 'trending' | 'security' | 'community'

export type InstallTargetMode = 'project' | 'specific_agent' | 'global'

export type SkillSourceKind =
  | 'halowake_managed'
  | 'git_remote'
  | 'unknown'
  | 'managed'
  | 'git'

export type SkillManagementKind = 'halowake' | 'git' | 'external' | 'unmanaged'

export type SkillUpdateStatus = 'update_available' | 'up_to_date' | 'unknown' | 'error' | 'unsupported'

export interface SkillSourceIdentity {
  kind: SkillSourceKind
  gitUrl?: string
  subPath?: string
  commit?: string
  contentSha256?: string
}

export interface SkillIdentity {
  catalogId?: string
  subSkillId?: string
  canonicalName: string
  source: SkillSourceIdentity
  management: SkillManagementKind
}

export interface AgentSkill {
  id: string
  catalogId?: string
  subSkillId?: string
  canonicalName?: string
  name: string
  description: string
  version?: string
  agentType: string
  agentLabel: string
  associatedAgents?: string[]
  path: string
  subPath?: string
  installedAt?: number
  isGitRepo: boolean
  gitRemoteUrl?: string
  gitBranch?: string
  currentCommit?: string
  isDirty: boolean
  dirty?: boolean | null
  canUninstall: boolean
  externallyManaged: boolean
  management?: SkillManagementKind
  source?: SkillSourceIdentity
  hasUpdate: boolean
  updateStatus?: SkillUpdateStatus
  updateError?: string
  latestCommit?: string
  latestVersion?: string
  updateSummary?: string
  commitsBehind: number
  lastCheckedAt?: number
  cleanDesc?: string
  displayAgentTag?: string
  searchIndex?: string
  stage?: SdlcStage
  customCategory?: string
}

export interface SkillUpdateResult {
  skillId: string
  success: boolean
  message: string
  previousCommit?: string
  newCommit?: string
  backupPath?: string
}

export interface SubSkillItem {
  id: string
  name: string
  titleZh: string
  description: string
  subPath?: string // e.g. "skills/canvas"
  tags?: string[]
}

export type InstallItemStatus = 'installed' | 'skipped' | 'failed' | 'missing' | 'cancelled'
export type InstallationItemStatus = InstallItemStatus | 'partial'

export type InstallationBatchStatus = 'installed' | 'skipped' | 'partial' | 'failed' | 'missing'

export interface InstallTargetOutcome {
  targetAgent: string
  targetPath: string
  status: InstallItemStatus
  error?: string
  itemKey?: string
}

export type InstallationTargetItemResult = InstallTargetOutcome

export interface InstallationItemPlan {
  itemKey: string
  skillId: string
  subSkillId?: string
  displayName: string
  gitUrl: string
  subPath?: string
  status: 'pending' | InstallItemStatus | 'partial'
  error?: string
}

export interface InstallationItemResult {
  itemKey: string
  skillId: string
  subSkillId?: string
  displayName: string
  gitUrl: string
  subPath?: string
  status: InstallItemStatus | 'partial'
  message?: string
  error?: string
  installedPaths: string[]
  skippedPaths: string[]
  targetOutcomes?: InstallTargetOutcome[]
  targetResults?: InstallTargetOutcome[]
}

export interface InstallationBatchResult {
  operationId: string
  retryOfOperationId?: string
  success: boolean
  status: InstallationBatchStatus
  message: string
  count: number
  installedCount: number
  skippedCount: number
  failedCount: number
  missingCount: number
  installedPaths: string[]
  skippedPaths: string[]
  items: InstallationItemResult[]
}

export interface InstallSkillArgs {
  skillId: string
  gitUrl: string
  subPath?: string
  targetMode: InstallTargetMode
  targetAgents: string[]
  projectCwd?: string
  addToGitignore: boolean
  subSkillId?: string // 针对合集安装时，可指定单装某个子技能 ID
  operationId?: string
  itemKey?: string
  retryOfOperationId?: string
}

export interface InstallSkillResult {
  success: boolean
  status?: InstallItemStatus | 'partial'
  message: string
  installedPaths: string[]
  skippedPaths?: string[]
  targetOutcomes?: InstallTargetOutcome[]
  targetResults?: InstallTargetOutcome[]
}

export interface SkillInstallHistoryEntry {
  id: number
  skillId: string
  displayName: string
  subSkillId?: string
  gitUrl: string
  subPath?: string
  targetMode: InstallTargetMode
  targetAgents: string[]
  projectCwd?: string
  status: 'success' | 'skipped' | 'partial' | 'failed'
  message: string
  installedPaths: string[]
  existingPaths: string[]
  createdAt: number
  operationId?: string
  itemKey?: string
  targetOutcomes?: InstallTargetOutcome[]
  retryOfOperationId?: string
}

export interface InstallSkillProgress {
  operationId?: string
  itemKey?: string
  phase: 'preparing' | 'connecting' | 'downloading' | 'retrying' | 'extracting' | 'copying' | 'saving' | 'complete' | 'failed'
  message: string
  percent: number | null
}

export interface SkillAuthor {
  name: string
  url?: string
  verified?: boolean
  avatar?: string
}

export interface SkillMetrics {
  githubStars: number
  forks: number
  lastCommitDaysAgo: number
  issueCloseRate: number // e.g. 92 (%)
  securityAuditPassed: boolean
}

export interface SkillUsageGuide {
  extractorVersion?: number
  summary: string
  trigger?: string
  steps: string[]
  example?: string
  sourceUrl: string
  sourceHash: string
  extractedAt: number
}

export interface CatalogSkill {
  id: string
  name: string
  titleZh: string
  description: string
  stage: SdlcStage
  category: string
  author: SkillAuthor
  gitUrl: string
  subPath?: string
  stars: number
  halowakeScore?: number // 0-100
  badge: SkillBadgeType
  metrics: SkillMetrics
  tags: string[]
  recommendedWith: string[] // skill IDs
  howToUse: string
  usageGuide?: SkillUsageGuide
  promptExample?: string
  compatibleAgents: string[]
  isStar?: boolean
  customCategory?: string
  isBundle?: boolean // 是否为多技能集合/Monorepo
  bundleCount?: number // 包含的子技能数量
  subSkills?: SubSkillItem[] // 包含的独立子技能列表
  sourceId?: string
}

export type SkillSearchMatchTier = 'browse' | 'exact' | 'general_fallback' | 'none'

export interface SkillSearchMatchReason {
  tier: 'exact' | 'general' | 'browse'
  summary: string
  matchedFields: string[]
  matchedTerms: string[]
}

export interface SkillSearchRequest {
  query: string
  stage?: string | null
  category?: string | null
  allowedIds?: string[] | null
  projectTechStack?: string[]
  limit?: number
  offset?: number
  sort?: 'relevance' | 'popular' | 'name' | string
  catalogRevision?: string | null
}

export interface SkillSearchHit {
  id: string
  name: string
  titleZh: string
  score: number
  coreMatched: number
  coreTotal: number
  techCoreMatched: number
  nonTechCoreMatched: number
  matchReason: SkillSearchMatchReason
  skill: CatalogSkill
}

export interface SkillSearchResponse {
  hits: SkillSearchHit[]
  totalMatched: number
  matchTier: SkillSearchMatchTier
  fallbackNotice?: string | null
  parsedTerms: string[]
  usedFallbackEngine: boolean
  elapsedMs: number
  hasMore?: boolean
  isTruncated?: boolean
  catalogRevision?: string | null
}

export interface SkillSource {
  id: string
  name: string
  repoUrl: string
  branch: string
  category: string
  enabled: boolean
  isBuiltin: boolean
  lastSyncedAt?: number
  skillCount: number
  status: 'idle' | 'syncing' | 'success' | 'error'
  errorMessage?: string
}

export interface SyncAllSummary {
  success: boolean
  message: string
  totalSkills: number
  sourcesSynced: number
  sourcesFailed: number
  syncedAt: number
}

export interface PipelineStepPlaybook {
  stage: SdlcStage
  skillId: string
  skillName: string
  role: string
  howToAct: string
  promptExample: string
}

export interface SkillPipeline {
  id: string
  title: string
  subtitle: string
  description: string
  targetScenario: string
  badge: string
  icon: string
  skills: string[] // catalog skill IDs ordered by SDLC stage
  playbook?: PipelineStepPlaybook[]
}

export interface InstallPreference {
  targetMode: InstallTargetMode
  targetAgents: string[]
  addToGitignore: boolean
  remember: boolean
  customProjectPath?: string
}

export interface ExecuteInstallOptions {
  projectCwd?: string
  preference?: InstallPreference
  persistPreference?: boolean
  onlyFailed?: boolean
}

export type RegistrySyncStatus = 'synced' | 'persist_failed' | 'cached_fallback' | 'failed'

export interface RemoteSkillsRegistry {
  version: string
  updatedAt: number
  catalogRevision?: string
  description?: string
  skills: CatalogSkill[]
  pipelines?: SkillPipeline[]
}

export interface RegistrySyncResult {
  success: boolean
  status: RegistrySyncStatus
  updatedCount: number
  totalCount: number
  lastSyncAt: number | null
  catalogRevision?: string
  message: string
  skills: CatalogSkill[]
  pipelines: SkillPipeline[]
}
