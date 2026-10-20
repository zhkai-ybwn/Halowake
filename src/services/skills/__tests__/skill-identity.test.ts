import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { i18n } from '@/i18n'
import SkillCard from '@/components/skills/explore/SkillCard.vue'
import SkillCompactRow from '@/components/skills/explore/SkillCompactRow.vue'
import {
  compareCandidateWithInstalledSkill,
  isCatalogSkillInstalledInTargets,
  isSubSkillInstalledInTargets,
  normalizeGitUrl,
  normalizeSkillSubPath,
  resolveInstalledSkillIdentity,
} from '@/services/skills/skill-identity'
import { useSkillExploreStore } from '@/stores/skill-explore'
import { useSkillsStore } from '@/stores/skills'
import type { AgentSkill, CatalogSkill } from '@/types/skills'

function createCatalogSkill(overrides: Partial<CatalogSkill> & { id: string; name: string }): CatalogSkill {
  return {
    id: overrides.id,
    name: overrides.name,
    titleZh: overrides.titleZh ?? overrides.name,
    description: overrides.description ?? 'Skill description',
    stage: overrides.stage ?? 'coding',
    category: overrides.category ?? 'general',
    author: overrides.author ?? { name: 'Author' },
    gitUrl: overrides.gitUrl ?? 'https://github.com/acme/skills',
    subPath: overrides.subPath,
    stars: overrides.stars ?? 10,
    halowakeScore: overrides.halowakeScore ?? 85,
    badge: overrides.badge ?? 'verified',
    metrics: overrides.metrics ?? {
      githubStars: 10,
      forks: 2,
      lastCommitDaysAgo: 1,
      issueCloseRate: 90,
      securityAuditPassed: true,
    },
    tags: overrides.tags ?? ['test'],
    recommendedWith: overrides.recommendedWith ?? [],
    howToUse: overrides.howToUse ?? 'Use it',
    compatibleAgents: overrides.compatibleAgents ?? ['codex', 'antigravity'],
    isBundle: overrides.isBundle ?? false,
    bundleCount: overrides.bundleCount,
    subSkills: overrides.subSkills,
  }
}

function createInstalledSkill(
  overrides: Partial<AgentSkill> & { id: string; name: string; path: string }
): AgentSkill {
  return {
    id: overrides.id,
    catalogId: overrides.catalogId,
    subSkillId: overrides.subSkillId,
    canonicalName: overrides.canonicalName,
    name: overrides.name,
    description: overrides.description ?? 'Installed skill',
    version: overrides.version ?? '1.0.0',
    agentType: overrides.agentType ?? 'codex',
    agentLabel: overrides.agentLabel ?? 'Codex',
    associatedAgents: overrides.associatedAgents ?? [overrides.agentType ?? 'codex'],
    path: overrides.path,
    subPath: overrides.subPath,
    installedAt: overrides.installedAt ?? 1700000000000,
    isGitRepo: overrides.isGitRepo ?? true,
    gitRemoteUrl: overrides.gitRemoteUrl,
    gitBranch: overrides.gitBranch ?? 'main',
    currentCommit: overrides.currentCommit ?? '1234567',
    isDirty: overrides.isDirty ?? false,
    dirty: overrides.dirty ?? false,
    canUninstall: overrides.canUninstall ?? true,
    externallyManaged: overrides.externallyManaged ?? false,
    management: overrides.management,
    source: overrides.source,
    hasUpdate: overrides.hasUpdate ?? false,
    commitsBehind: overrides.commitsBehind ?? 0,
  }
}

describe('skill-identity (T02: F01, F08)', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('normalizes subPath preserving case and rejects traversal, absolute, and drive-prefixed paths', () => {
    expect(normalizeSkillSubPath('')).toBe('')
    expect(normalizeSkillSubPath(undefined)).toBe('')
    expect(normalizeSkillSubPath('  ./skills/canvas/  ')).toBe('skills/canvas')
    expect(normalizeSkillSubPath('skills\\canvas')).toBe('skills/canvas')
    expect(normalizeSkillSubPath('.agents/skills/impeccable')).toBe('.agents/skills/impeccable')

    // Case-sensitive: Skills/Canvas !== skills/canvas
    expect(normalizeSkillSubPath('Skills/Canvas')).toBe('Skills/Canvas')
    expect(normalizeSkillSubPath('Skills/Canvas')).not.toBe(normalizeSkillSubPath('skills/canvas'))

    // Reject traversal, empty segments, root paths, and drive letters
    expect(normalizeSkillSubPath('../evil')).toBeNull()
    expect(normalizeSkillSubPath('skills/../evil')).toBeNull()
    expect(normalizeSkillSubPath('skills/./canvas')).toBeNull()
    expect(normalizeSkillSubPath('skills//canvas')).toBeNull()
    expect(normalizeSkillSubPath('/abs/path')).toBeNull()
    expect(normalizeSkillSubPath('\\abs\\path')).toBeNull()
    expect(normalizeSkillSubPath('C:\\Users\\evil')).toBeNull()
    expect(normalizeSkillSubPath('.')).toBeNull()
    expect(normalizeSkillSubPath('./')).toBeNull()
  })

  it('does not deduplicate bundle sub-skill B against installed sub-skill A from the same catalogId and gitUrl', () => {
    const bundle = createCatalogSkill({
      id: 'acme-design-suite',
      name: 'acme-design-suite',
      gitUrl: 'https://github.com/acme/design-suite.git',
      isBundle: true,
      bundleCount: 2,
      subSkills: [
        {
          id: 'sub-canvas',
          name: 'canvas-design',
          titleZh: '画布设计',
          description: 'Canvas sub-skill',
          subPath: 'skills/canvas',
        },
        {
          id: 'sub-tokens',
          name: 'token-audit',
          titleZh: '令牌审计',
          description: 'Tokens sub-skill',
          subPath: 'skills/tokens',
        },
      ],
    })

    const installedOnlyA: AgentSkill[] = [
      createInstalledSkill({
        id: 'codex:canvas-design',
        catalogId: 'acme-design-suite',
        subSkillId: 'sub-canvas',
        canonicalName: 'canvas-design',
        name: 'canvas-design',
        path: 'C:/tmp/.codex/skills/canvas-design',
        gitRemoteUrl: 'https://github.com/acme/design-suite',
        subPath: 'skills/canvas',
        management: 'halowake',
        source: {
          kind: 'managed',
          gitUrl: 'https://github.com/acme/design-suite',
          subPath: 'skills/canvas',
          commit: 'abcdef1',
          contentSha256: 'hash-a',
        },
      }),
    ]

    expect(isSubSkillInstalledInTargets(bundle, bundle.subSkills![0], installedOnlyA)).toBe(true)
    expect(isSubSkillInstalledInTargets(bundle, bundle.subSkills![1], installedOnlyA)).toBe(false)
    // Bundle as a whole is NOT fully installed when only sub-skill A is installed
    expect(isCatalogSkillInstalledInTargets(bundle, installedOnlyA)).toBe(false)

    const exploreStore = useSkillExploreStore()
    exploreStore.catalogSkills = [bundle]
    expect(exploreStore.isSkillInstalled('acme-design-suite', installedOnlyA)).toBe(false)
    expect(
      exploreStore.isSkillInstalled('acme-design-suite', installedOnlyA, {
        subSkillId: 'sub-canvas',
      })
    ).toBe(true)
    expect(
      exploreStore.isSkillInstalled('acme-design-suite', installedOnlyA, {
        subSkillId: 'sub-tokens',
      })
    ).toBe(false)
  })

  it('recognizes same-name cross-source skills and case-mismatched subPaths as conflict, not installed', () => {
    const candidate = {
      catalogId: 'owner-a-linter',
      name: 'code-review',
      gitUrl: 'https://github.com/owner-a/skills',
      subPath: 'skills/code-review',
    }

    const crossRepoInstalled = createInstalledSkill({
      id: 'codex:code-review',
      catalogId: 'owner-a-linter',
      name: 'code-review',
      path: 'C:/tmp/.codex/skills/code-review',
      gitRemoteUrl: 'https://github.com/owner-b/skills',
      subPath: 'skills/code-review',
    })

    expect(compareCandidateWithInstalledSkill(candidate, crossRepoInstalled)).toBe('conflict')

    const caseMismatchInstalled = createInstalledSkill({
      id: 'codex:code-review',
      catalogId: 'owner-a-linter',
      name: 'code-review',
      path: 'C:/tmp/.codex/skills/code-review',
      gitRemoteUrl: 'https://github.com/owner-a/skills',
      subPath: 'Skills/Code-Review',
    })

    expect(compareCandidateWithInstalledSkill(candidate, caseMismatchInstalled)).toBe('conflict')
  })

  it('matches legacy catalogId-named directory when gitUrl and subPath match', () => {
    const catalogId = 'corpus-vercel-labs-native-skill-data-native-ui-4089298a'
    const skill = createCatalogSkill({
      id: catalogId,
      name: 'native-ui',
      gitUrl: 'https://github.com/vercel-labs/native',
      subPath: 'skill-data/native-ui',
    })

    const legacyInstalled = createInstalledSkill({
      id: `codex:${catalogId}`,
      catalogId,
      name: 'native-ui',
      path: `C:/tmp/.codex/skills/${catalogId}`,
      gitRemoteUrl: 'https://github.com/vercel-labs/native.git',
      subPath: 'skill-data/native-ui',
    })

    expect(isCatalogSkillInstalledInTargets(skill, [legacyInstalled])).toBe(true)
    expect(normalizeGitUrl('https://github.com/vercel-labs/native.git/')).toBe(
      'https://github.com/vercel-labs/native'
    )
  })

  it('never treats an unmanaged / unknown-source local directory as same_skill', () => {
    const skill = createCatalogSkill({
      id: 'impeccable',
      name: 'impeccable',
      gitUrl: 'https://github.com/pbakaus/impeccable',
      subPath: '.agents/skills/impeccable',
    })

    const unmanagedLocal = createInstalledSkill({
      id: 'codex:impeccable',
      name: 'impeccable',
      path: 'C:/tmp/.codex/skills/impeccable',
      isGitRepo: false,
      gitRemoteUrl: undefined,
      management: 'unmanaged',
      source: { kind: 'unknown' },
    })

    const identity = resolveInstalledSkillIdentity(unmanagedLocal)
    expect(identity.source.kind).toBe('unknown')
    expect(identity.management).toBe('unmanaged')
    expect(
      compareCandidateWithInstalledSkill(
        {
          catalogId: skill.id,
          name: skill.name,
          gitUrl: skill.gitUrl,
          subPath: skill.subPath,
        },
        unmanagedLocal
      )
    ).toBe('conflict')
    expect(isCatalogSkillInstalledInTargets(skill, [unmanagedLocal])).toBe(false)
  })

  it('evaluates installation per target so installing in Agent A still allows installing to Agent B or Project P', async () => {
    const skill = createCatalogSkill({
      id: 'react-doctor',
      name: 'react-doctor',
      gitUrl: 'https://github.com/acme/react-doctor',
      subPath: 'skills/react-doctor',
    })

    const installedInCodex = createInstalledSkill({
      id: 'codex:react-doctor',
      catalogId: 'react-doctor',
      name: 'react-doctor',
      agentType: 'codex',
      agentLabel: 'Codex',
      associatedAgents: ['codex'],
      path: 'C:/tmp/.codex/skills/react-doctor',
      gitRemoteUrl: 'https://github.com/acme/react-doctor',
      subPath: 'skills/react-doctor',
    })

    const exploreStore = useSkillExploreStore()
    exploreStore.catalogSkills = [skill]

    // Installed globally (across any target) is true for badge display
    expect(exploreStore.isSkillInstalled('react-doctor', [installedInCodex])).toBe(true)
    // Installed in Codex specifically is true
    expect(
      exploreStore.isSkillInstalled('react-doctor', [installedInCodex], { agentType: 'codex' })
    ).toBe(true)
    // Installed in Antigravity or Project P is FALSE -> still installable there!
    expect(
      exploreStore.isSkillInstalled('react-doctor', [installedInCodex], {
        agentType: 'antigravity',
      })
    ).toBe(false)
    expect(
      exploreStore.isSkillInstalled('react-doctor', [installedInCodex], {
        agentType: 'project',
        projectCwd: 'C:/tmp/project-p',
      })
    ).toBe(false)

    const targets = exploreStore.getSkillInstalledTargets('react-doctor', [installedInCodex])
    expect(targets.isInstalled).toBe(true)
    expect(targets.agentTypes).toEqual(['codex'])
    expect(targets.labels).toEqual(['Codex'])

    // Verify SkillCard and SkillCompactRow buttons are NOT disabled when installed in Codex,
    // allowing user to open the install wizard for Antigravity or Project P
    const skillsStore = useSkillsStore()
    skillsStore.skills = [installedInCodex]

    const cardWrapper = mount(SkillCard, {
      props: { skill },
      global: { plugins: [i18n] },
    })
    const cardInstallBtn = cardWrapper.find('.card-action-btn--installed')
    expect(cardInstallBtn.exists()).toBe(true)
    expect(cardInstallBtn.attributes('disabled')).toBeUndefined()
    await cardInstallBtn.trigger('click')
    expect(exploreStore.installModalVisible).toBe(true)
    exploreStore.closeInstallDialog()

    const rowWrapper = mount(SkillCompactRow, {
      props: { skill },
      global: { plugins: [i18n] },
    })
    const rowInstallBtn = rowWrapper.find('.skill-row-installed-pill')
    expect(rowInstallBtn.exists()).toBe(true)
    await rowInstallBtn.trigger('click')
    expect(exploreStore.installModalVisible).toBe(true)
  })

  it('maintains dual-key compatibility (catalogId and canonicalName) for stars and custom categories with idempotent writes', () => {
    localStorage.setItem(
      'lumina.skills.stars',
      JSON.stringify(['native-ui'])
    )
    localStorage.setItem(
      'lumina.skills.userCustomCategories',
      JSON.stringify({
        'native-ui': ['UI精选'],
        'codex:native-ui': ['前端组件'],
      })
    )

    setActivePinia(createPinia())
    const exploreStore = useSkillExploreStore()
    const catalogId = 'corpus-vercel-labs-native-skill-data-native-ui-4089298a'
    exploreStore.catalogSkills = [
      createCatalogSkill({
        id: catalogId,
        name: 'native-ui',
        gitUrl: 'https://github.com/vercel-labs/native',
        subPath: 'skill-data/native-ui',
      }),
    ]

    // Reading custom categories merges legacy canonicalName and agentType:name keys
    const tags = exploreStore.getSkillCustomCategories({
      id: `codex:${catalogId}`,
      catalogId,
      name: 'native-ui',
    })
    expect(tags).toEqual(['UI精选', '前端组件'])

    // Writing a custom category normalizes to stable catalogId and deduplicates
    exploreStore.addSkillCustomCategory('codex:native-ui', 'UI精选')
    exploreStore.addSkillCustomCategory('native-ui', 'React')
    const afterAdd = exploreStore.getSkillCustomCategories(catalogId)
    expect(afterAdd).toEqual(['UI精选', '前端组件', 'React'])
    expect(exploreStore.userCustomCategories[catalogId]).toEqual(['UI精选', '前端组件', 'React'])
    expect(exploreStore.userCustomCategories['native-ui']).toBeUndefined()
    expect(exploreStore.userCustomCategories['codex:native-ui']).toBeUndefined()
  })

  it('strictly rejects same_skill when candidate is a sub-skill but installed lacks subPath evidence (R06)', () => {
    const candidate = {
      catalogId: 'bundle-pack',
      subSkillId: 'sub-canvas',
      name: 'canvas',
      gitUrl: 'https://github.com/acme/bundle-pack',
      subPath: 'skills/canvas',
    }

    const installedWithoutSubPath = createInstalledSkill({
      id: 'codex:canvas',
      catalogId: 'bundle-pack',
      subSkillId: 'sub-canvas',
      name: 'canvas',
      path: 'C:/tmp/.codex/skills/canvas',
      gitRemoteUrl: 'https://github.com/acme/bundle-pack',
      subPath: undefined,
      source: undefined,
    })

    expect(compareCandidateWithInstalledSkill(candidate, installedWithoutSubPath)).toBe('conflict')

    const installedOtherSubPath = createInstalledSkill({
      id: 'codex:canvas',
      catalogId: 'bundle-pack',
      subSkillId: 'sub-tokens',
      name: 'canvas',
      path: 'C:/tmp/.codex/skills/canvas',
      gitRemoteUrl: 'https://github.com/acme/bundle-pack',
      subPath: 'skills/tokens',
      source: {
        kind: 'halowake_managed',
        gitUrl: 'https://github.com/acme/bundle-pack',
        subPath: 'skills/tokens',
      },
    })
    expect(compareCandidateWithInstalledSkill(candidate, installedOtherSubPath)).toBe('conflict')
  })

  it('correctly maps Rust source.kind strings halowake_managed and git_remote to management kinds (R06)', () => {
    const halowakeManaged = createInstalledSkill({
      id: 'codex:test-skill',
      name: 'test-skill',
      path: 'C:/tmp/.codex/skills/test-skill',
      source: {
        kind: 'halowake_managed',
        gitUrl: 'https://github.com/acme/skills',
        subPath: 'skills/test-skill',
      },
    })
    const resolvedHalowake = resolveInstalledSkillIdentity(halowakeManaged)
    expect(resolvedHalowake.management).toBe('halowake')
    expect(resolvedHalowake.source.kind).toBe('halowake_managed')

    const gitRemoteManaged = createInstalledSkill({
      id: 'codex:test-git',
      name: 'test-git',
      path: 'C:/tmp/.codex/skills/test-git',
      source: {
        kind: 'git_remote',
        gitUrl: 'https://github.com/acme/repo',
        subPath: '',
      },
    })
    const resolvedGit = resolveInstalledSkillIdentity(gitRemoteManaged)
    expect(resolvedGit.management).toBe('git')
    expect(resolvedGit.source.kind).toBe('git_remote')
  })
})
