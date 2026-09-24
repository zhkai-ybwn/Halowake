export type SdlcStage = 'requirement' | 'design' | 'coding' | 'testing' | 'delivery'

export type SkillBadgeType = 'verified' | 'trending' | 'security' | 'community'

export type InstallTargetMode = 'project' | 'specific_agent' | 'global'

export interface AgentSkill {
  id: string
  name: string
  description: string
  version?: string
  agentType: string
  agentLabel: string
  associatedAgents?: string[]
  path: string
  isGitRepo: boolean
  gitRemoteUrl?: string
  gitBranch?: string
  currentCommit?: string
  isDirty: boolean
  hasUpdate: boolean
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
}

export interface SubSkillItem {
  id: string
  name: string
  titleZh: string
  description: string
  subPath?: string // e.g. "skills/canvas"
  tags?: string[]
}

export interface InstallSkillArgs {
  skillId: string
  gitUrl: string
  targetMode: InstallTargetMode
  targetAgents: string[]
  projectCwd?: string
  addToGitignore: boolean
  subSkillId?: string // 针对合集安装时，可指定单装某个子技能 ID
  subPath?: string // 针对 Monorepo/合集仓库，指定相对子目录路径
}

export interface InstallSkillResult {
  success: boolean
  message: string
  installedPaths: string[]
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

export interface CatalogSkill {
  id: string
  name: string
  titleZh: string
  description: string
  stage: SdlcStage
  category: string
  author: SkillAuthor
  gitUrl: string
  stars: number
  halowakeScore: number // 0-100
  badge: SkillBadgeType
  metrics: SkillMetrics
  tags: string[]
  recommendedWith: string[] // skill IDs
  howToUse: string
  promptExample?: string
  compatibleAgents: string[]
  isStar?: boolean
  userRating?: number
  userFeedback?: 'up' | 'down' | null
  customCategory?: string
  isBundle?: boolean // 是否为多技能集合/Monorepo
  bundleCount?: number // 包含的子技能数量
  subSkills?: SubSkillItem[] // 包含的独立子技能列表
  sourceId?: string
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

export interface RemoteSkillsRegistry {
  version: string
  updatedAt: number
  description?: string
  skills: CatalogSkill[]
  pipelines?: SkillPipeline[]
}
