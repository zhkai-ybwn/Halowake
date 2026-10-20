import type {
  AgentSkill,
  CatalogSkill,
  SkillIdentity,
  SkillManagementKind,
  SkillSourceIdentity,
  SubSkillItem,
} from '@/types/skills'

export interface SkillIdentityCandidate {
  catalogId?: string
  subSkillId?: string
  name: string
  gitUrl: string
  subPath?: string
}

export interface SkillTargetFilter {
  agentType?: string
  projectCwd?: string
  subSkillId?: string
  subPath?: string
}

export interface InstalledTargetsSummary {
  isInstalled: boolean
  count: number
  agentTypes: string[]
  labels: string[]
  skills: AgentSkill[]
}

/**
 * Normalizes a GitHub/Git repository URL for identity comparison.
 */
export function normalizeGitUrl(url?: string | null): string {
  if (!url || typeof url !== 'string') return ''
  return url
    .trim()
    .replace(/\/+$/, '')
    .replace(/\.git$/i, '')
    .replace(/\/+$/, '')
    .toLowerCase()
}

/**
 * Validates and normalizes a skill `subPath`.
 * - Converts `\` to `/`
 * - Trims whitespace, leading `./`, and trailing `/`
 * - Rejects leading `/` or `\`, drive prefixes (`C:`), empty segments, `.`, and `..`
 * - Preserves original case (Git tree paths are case-sensitive)
 * - Returns `""` for root repository skills, or `null` if the path is invalid/unsafe.
 */
export function normalizeSkillSubPath(subPath?: string | null): string | null {
  if (subPath === undefined || subPath === null) return ''
  if (typeof subPath !== 'string') return null

  const trimmed = subPath.trim()
  if (trimmed === '') return ''

  // Reject absolute root paths, UNC paths, drive prefixes, or null bytes
  if (
    trimmed.startsWith('/') ||
    trimmed.startsWith('\\') ||
    trimmed.includes(':') ||
    trimmed.includes('\0')
  ) {
    return null
  }

  let clean = trimmed.replace(/\\/g, '/')
  while (clean.startsWith('./')) {
    clean = clean.slice(2)
  }
  clean = clean.replace(/\/+$/, '')

  if (!clean || clean === '.' || clean === '..' || clean.startsWith('/')) {
    return null
  }

  const segments = clean.split('/')
  for (const seg of segments) {
    if (!seg || seg === '.' || seg === '..') {
      return null
    }
  }

  return segments.join('/')
}

/**
 * Normalizes a canonical skill name (`kebab-case`, ASCII lowercase + digits + hyphens).
 * Returns `null` if invalid or unsafe.
 */
export function normalizeCanonicalSkillName(raw?: string | null): string | null {
  if (!raw || typeof raw !== 'string') return null
  const trimmed = raw.trim()
  if (!trimmed) return null
  if (
    trimmed.includes('/') ||
    trimmed.includes('\\') ||
    trimmed.includes('..') ||
    trimmed.startsWith('.')
  ) {
    return null
  }
  const normalized = trimmed
    .toLowerCase()
    .replace(/[_\s]+/g, '-')
    .replace(/^-+|-+$/g, '')
  if (!normalized || normalized.length > 64 || normalized.includes('..')) {
    return null
  }
  if (!/^[a-z0-9-]+$/.test(normalized)) {
    return null
  }
  return normalized
}

/**
 * Resolves the structured `SkillIdentity` for an installed `AgentSkill`.
 * Local directories without `.lumina-skill-source.json` or `.git` remote URL
 * are strictly classified as `source.kind = 'unknown'` and `management = 'unmanaged'`.
 */
export function resolveInstalledSkillIdentity(skill: AgentSkill): SkillIdentity {
  const canonicalName =
    normalizeCanonicalSkillName(skill.canonicalName) ??
    normalizeCanonicalSkillName(skill.name) ??
    skill.name.trim().toLowerCase()

  let management: SkillManagementKind = skill.management ?? 'unmanaged'
  if (!skill.management) {
    if (skill.externallyManaged) {
      management = 'external'
    } else if (skill.source?.kind === 'halowake_managed' || skill.source?.kind === 'managed') {
      management = 'halowake'
    } else if (
      skill.source?.kind === 'git_remote' ||
      skill.source?.kind === 'git' ||
      (skill.isGitRepo && skill.gitRemoteUrl)
    ) {
      management = 'git'
    } else {
      management = 'unmanaged'
    }
  }

  if (skill.source?.kind === 'unknown' || management === 'unmanaged') {
    return {
      catalogId: skill.catalogId,
      subSkillId: skill.subSkillId,
      canonicalName,
      source: { kind: 'unknown' },
      management,
    }
  }

  const rawGitUrl = skill.source?.gitUrl ?? skill.gitRemoteUrl
  const normalizedRepo = normalizeGitUrl(rawGitUrl)
  if (!normalizedRepo) {
    return {
      catalogId: skill.catalogId,
      subSkillId: skill.subSkillId,
      canonicalName,
      source: { kind: 'unknown' },
      management: skill.externallyManaged ? 'external' : 'unmanaged',
    }
  }

  const rawSubPath = skill.source?.subPath ?? skill.subPath
  const normalizedSub = rawSubPath !== undefined ? normalizeSkillSubPath(rawSubPath) : undefined

  const source: SkillSourceIdentity = {
    kind: skill.source?.kind ?? (rawSubPath !== undefined || skill.catalogId ? 'managed' : 'git'),
    gitUrl: rawGitUrl,
    subPath: normalizedSub ?? undefined,
    commit: skill.source?.commit ?? skill.currentCommit,
    contentSha256: skill.source?.contentSha256,
  }

  return {
    catalogId: skill.catalogId,
    subSkillId: skill.subSkillId,
    canonicalName,
    source,
    management,
  }
}

/**
 * Compares a catalog skill (or bundle sub-skill) candidate against an installed `AgentSkill`.
 * - Returns `'same_skill'` ONLY when the installed skill has a confirmed source (`gitUrl` + case-sensitive `subPath`)
 *   matching the candidate.
 * - Returns `'conflict'` when the installed skill occupies the same canonical name / folder name
 *   from a different repository, different `subPath`, or unknown/unmanaged local source.
 * - Returns `'unrelated'` otherwise.
 */
export function compareCandidateWithInstalledSkill(
  candidate: SkillIdentityCandidate,
  installed: AgentSkill
): 'same_skill' | 'conflict' | 'unrelated' {
  const expectedRepo = normalizeGitUrl(candidate.gitUrl)
  const expectedSub = normalizeSkillSubPath(candidate.subPath)
  const candidateCanon =
    normalizeCanonicalSkillName(candidate.name) ?? candidate.name.trim().toLowerCase()
  const folderName =
    installed.path
      .split(/[\\/]/)
      .filter(Boolean)
      .pop()
      ?.toLowerCase() ?? ''

  const identity = resolveInstalledSkillIdentity(installed)
  const sameNameOrFolder =
    Boolean(candidateCanon) &&
    (identity.canonicalName === candidateCanon || folderName === candidateCanon)

  // Unmanaged / unknown source skills can NEVER be treated as "already installed"
  if (identity.source.kind === 'unknown' || expectedSub === null || !expectedRepo) {
    return sameNameOrFolder ? 'conflict' : 'unrelated'
  }

  const installedRepo = normalizeGitUrl(identity.source.gitUrl)
  if (!installedRepo || installedRepo !== expectedRepo) {
    return sameNameOrFolder ? 'conflict' : 'unrelated'
  }

  const hasExplicitInstalledSub =
    installed.source?.subPath !== undefined || installed.subPath !== undefined

  if (hasExplicitInstalledSub) {
    const installedSub = normalizeSkillSubPath(installed.source?.subPath ?? installed.subPath)
    if (installedSub === null) {
      return sameNameOrFolder ? 'conflict' : 'unrelated'
    }
    if (installedSub === expectedSub) {
      return 'same_skill'
    }
    return sameNameOrFolder ? 'conflict' : 'unrelated'
  }

  // When installed lacks explicit subPath evidence:
  // Root repository candidates (expectedSub === '') can match if confirmed root repository with matching identity
  if (expectedSub === '') {
    if (
      sameNameOrFolder ||
      (candidate.catalogId &&
        installed.catalogId?.toLowerCase() === candidate.catalogId.toLowerCase())
    ) {
      return 'same_skill'
    }
  }

  // If candidate is a sub-skill (expectedSub !== '') but installed lacks subPath evidence,
  // we cannot verify same_skill. If folder name matches, it's a conflict; otherwise unrelated.
  return sameNameOrFolder ? 'conflict' : 'unrelated'
}

const isWindowsPlatform =
  typeof navigator !== 'undefined'
    ? /win/i.test(navigator.userAgent || navigator.platform)
    : typeof (globalThis as { process?: { platform?: string } }).process !== 'undefined'
      ? (globalThis as { process?: { platform?: string } }).process?.platform === 'win32'
      : true

export function normalizeFsPathPrefix(pathStr?: string | null): string {
  if (!pathStr) return ''
  const trimmed = pathStr
    .trim()
    .replace(/\\/g, '/')
    .replace(/\/+$/, '')
  return isWindowsPlatform ? trimmed.toLowerCase() : trimmed
}

/**
 * Checks whether an installed skill belongs to the requested installation target filter.
 */
export function doesInstalledSkillMatchTarget(
  installed: AgentSkill,
  filter?: SkillTargetFilter
): boolean {
  if (!filter) return true

  if (filter.agentType) {
    const targetAgent = filter.agentType.toLowerCase()
    const matchesAgent =
      installed.agentType.toLowerCase() === targetAgent ||
      (installed.associatedAgents?.some(a => a.toLowerCase() === targetAgent) ?? false)
    if (!matchesAgent) return false
  }

  if (filter.projectCwd) {
    const normalizedCwd = normalizeFsPathPrefix(filter.projectCwd)
    const normalizedSkillPath = normalizeFsPathPrefix(installed.path)
    if (normalizedCwd && !normalizedSkillPath.startsWith(`${normalizedCwd}/`)) {
      return false
    }
  }

  return true
}

export function buildSubSkillCandidate(
  parentSkill: CatalogSkill,
  subSkill: SubSkillItem
): SkillIdentityCandidate {
  return {
    catalogId: parentSkill.id,
    subSkillId: subSkill.id,
    name: subSkill.name || subSkill.id,
    gitUrl: parentSkill.gitUrl,
    subPath: subSkill.subPath,
  }
}

export function buildCatalogSkillCandidate(skill: CatalogSkill): SkillIdentityCandidate {
  return {
    catalogId: skill.id,
    name: skill.name || skill.id,
    gitUrl: skill.gitUrl,
    subPath: skill.subPath,
  }
}

/**
 * Checks whether a specific sub-skill of a bundle is installed (optionally scoped to a target).
 */
export function isSubSkillInstalledInTargets(
  parentSkill: CatalogSkill,
  subSkill: SubSkillItem,
  installedSkills: AgentSkill[],
  filter?: SkillTargetFilter
): boolean {
  const candidate = buildSubSkillCandidate(parentSkill, subSkill)
  return installedSkills.some(
    inst =>
      doesInstalledSkillMatchTarget(inst, filter) &&
      compareCandidateWithInstalledSkill(candidate, inst) === 'same_skill'
  )
}

/**
 * Checks whether a catalog skill (or bundle) is installed (optionally scoped to a target).
 * - For a bundle with `subSkills`, if `filter?.subSkillId` is provided, checks that specific sub-skill;
 *   otherwise returns `true` ONLY when all sub-skills in the bundle are installed in the target.
 */
export function isCatalogSkillInstalledInTargets(
  skill: CatalogSkill,
  installedSkills: AgentSkill[],
  filter?: SkillTargetFilter
): boolean {
  if (skill.isBundle && Array.isArray(skill.subSkills) && skill.subSkills.length > 0) {
    if (filter?.subSkillId) {
      const sub = skill.subSkills.find(
        s => s.id.toLowerCase() === filter.subSkillId!.toLowerCase()
      )
      if (!sub) return false
      return isSubSkillInstalledInTargets(skill, sub, installedSkills, filter)
    }
    return skill.subSkills.every(sub =>
      isSubSkillInstalledInTargets(skill, sub, installedSkills, filter)
    )
  }

  const candidate: SkillIdentityCandidate = {
    catalogId: skill.id,
    subSkillId: filter?.subSkillId,
    name: skill.name || skill.id,
    gitUrl: skill.gitUrl,
    subPath: filter?.subPath ?? skill.subPath,
  }

  return installedSkills.some(
    inst =>
      doesInstalledSkillMatchTarget(inst, filter) &&
      compareCandidateWithInstalledSkill(candidate, inst) === 'same_skill'
  )
}

/**
 * Returns the summary of installed targets (agents / project) for a catalog skill or sub-skill.
 */
export function getInstalledTargetsForCandidate(
  candidate: SkillIdentityCandidate,
  installedSkills: AgentSkill[]
): InstalledTargetsSummary {
  const matched = installedSkills.filter(
    inst => compareCandidateWithInstalledSkill(candidate, inst) === 'same_skill'
  )
  const agentTypesSet = new Set<string>()
  const labelsSet = new Set<string>()

  for (const inst of matched) {
    if (inst.agentType) agentTypesSet.add(inst.agentType)
    for (const assoc of inst.associatedAgents ?? []) {
      if (assoc) agentTypesSet.add(assoc)
    }
    const label = inst.displayAgentTag || inst.agentLabel || inst.agentType
    if (label) labelsSet.add(label)
  }

  return {
    isInstalled: matched.length > 0,
    count: matched.length,
    agentTypes: Array.from(agentTypesSet),
    labels: Array.from(labelsSet),
    skills: matched,
  }
}

/**
 * Finds the matching `CatalogSkill` for a local `AgentSkill`, verifying repository + subPath identity.
 */
export function findMatchingCatalogSkill(
  installed: AgentSkill,
  catalogSkills: CatalogSkill[]
): CatalogSkill | undefined {
  for (const cat of catalogSkills) {
    if (cat.isBundle && Array.isArray(cat.subSkills) && cat.subSkills.length > 0) {
      const matchesSub = cat.subSkills.some(
        sub =>
          compareCandidateWithInstalledSkill(buildSubSkillCandidate(cat, sub), installed) ===
          'same_skill'
      )
      if (matchesSub) return cat
    }
    if (
      compareCandidateWithInstalledSkill(buildCatalogSkillCandidate(cat), installed) ===
      'same_skill'
    ) {
      return cat
    }
  }
  return undefined
}

/**
 * Returns all candidate lookup keys for reading stars / custom categories (`catalogId`, `id`, `canonicalName`, `name`).
 */
export function getSkillLookupKeys(
  skill:
    | string
    | {
        id: string
        catalogId?: string
        canonicalName?: string
        name?: string
      },
  catalogSkills?: CatalogSkill[]
): string[] {
  const keys = new Set<string>()
  const addKey = (val?: string | null) => {
    if (!val) return
    const trimmed = val.trim()
    if (!trimmed) return
    keys.add(trimmed)
    const colonIdx = trimmed.indexOf(':')
    if (colonIdx >= 0 && colonIdx < trimmed.length - 1) {
      keys.add(trimmed.slice(colonIdx + 1))
    }
  }

  if (typeof skill === 'string') {
    addKey(skill)
    if (catalogSkills) {
      const lower = skill.trim().toLowerCase()
      const cleanAfterColon = lower.includes(':') ? lower.split(':').pop()! : lower
      const matched = catalogSkills.find(
        c =>
          c.id.toLowerCase() === lower ||
          c.id.toLowerCase() === cleanAfterColon ||
          c.name.toLowerCase() === cleanAfterColon
      )
      if (matched) {
        addKey(matched.id)
        addKey(matched.name)
      }
    }
  } else {
    addKey(skill.catalogId)
    addKey(skill.id)
    addKey(skill.canonicalName)
    addKey(skill.name)
    if (catalogSkills) {
      const lowerId = skill.catalogId?.toLowerCase() || skill.id.toLowerCase()
      const lowerName = (skill.canonicalName || skill.name || '').toLowerCase()
      const matched = catalogSkills.find(
        c =>
          c.id.toLowerCase() === lowerId ||
          (lowerName && c.name.toLowerCase() === lowerName)
      )
      if (matched) {
        addKey(matched.id)
        addKey(matched.name)
      }
    }
  }

  return Array.from(keys)
}

/**
 * Resolves the canonical stable storage key (`catalogId` when known) for persisting stars and custom categories.
 */
export function resolveStableCatalogKey(
  skillOrId:
    | string
    | {
        id: string
        catalogId?: string
        canonicalName?: string
        name?: string
      },
  catalogSkills: CatalogSkill[]
): string {
  if (typeof skillOrId !== 'string' && skillOrId.catalogId?.trim()) {
    return skillOrId.catalogId.trim()
  }
  const rawId = typeof skillOrId === 'string' ? skillOrId.trim() : skillOrId.id.trim()
  const cleanId = rawId.includes(':') ? rawId.split(':').pop()! : rawId
  const lowerClean = cleanId.toLowerCase()

  const exactIdMatch = catalogSkills.find(c => c.id.toLowerCase() === lowerClean)
  if (exactIdMatch) return exactIdMatch.id

  const nameMatches = catalogSkills.filter(c => c.name.toLowerCase() === lowerClean)
  if (nameMatches.length === 1) {
    return nameMatches[0].id
  }

  return rawId
}
