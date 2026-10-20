import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { i18n } from '@/i18n'
import { useSkillExploreStore } from '@/stores/skill-explore'
import { useSkillsStore } from '@/stores/skills'
import { usePreferencesStore } from '@/stores/preferences'
import InstalledSkillsView from '@/components/skills/installed/InstalledSkillsView.vue'
import SkillExploreView from '@/components/skills/explore/SkillExploreView.vue'
import SkillInstallModal from '@/components/skills/explore/SkillInstallModal.vue'
import { GIT_REPO_STORAGE_KEY } from '@/constants/git'
import { setTauriInvokeHandler } from './setup'
import type { AgentSkill, CatalogSkill } from '@/types/skills'

vi.mock('naive-ui', async () => {
  const actual = await vi.importActual<typeof import('naive-ui')>('naive-ui')
  const { defineComponent, h } = await import('vue')
  return {
    ...actual,
    NModal: defineComponent({
      name: 'NModal',
      props: { show: { type: Boolean, default: false } },
      setup(props, { slots }) {
        return () => (props.show ? h('div', { class: 'mock-n-modal' }, slots.default?.()) : null)
      },
    }),
    useMessage: () => ({
      success: vi.fn(),
      error: vi.fn(),
      warning: vi.fn(),
      info: vi.fn(),
    }),
    useDialog: () => ({
      warning: vi.fn(),
      error: vi.fn(),
      info: vi.fn(),
    }),
  }
})

function makeCatalogSkill(overrides: Partial<CatalogSkill> & { id: string; name: string }): CatalogSkill {
  return {
    id: overrides.id,
    name: overrides.name,
    titleZh: overrides.titleZh ?? overrides.name,
    description: overrides.description ?? 'Test skill description',
    stage: overrides.stage ?? 'coding',
    category: overrides.category ?? 'general',
    author: overrides.author ?? { name: 'tester' },
    gitUrl: overrides.gitUrl ?? 'https://github.com/a/skills',
    subPath: overrides.subPath,
    stars: overrides.stars ?? 0,
    halowakeScore: overrides.halowakeScore ?? 80,
    badge: overrides.badge ?? 'community',
    metrics: overrides.metrics ?? {
      githubStars: 10,
      forks: 1,
      lastCommitDaysAgo: 5,
      issueCloseRate: 90,
      securityAuditPassed: true,
    },
    tags: overrides.tags ?? ['test'],
    recommendedWith: overrides.recommendedWith ?? [],
    howToUse: overrides.howToUse ?? 'Use in agent',
    compatibleAgents: overrides.compatibleAgents ?? ['codex'],
    isBundle: overrides.isBundle ?? false,
    bundleCount: overrides.bundleCount,
    subSkills: overrides.subSkills,
    sourceId: overrides.sourceId,
  }
}

function makeInstalledSkill(overrides: Partial<AgentSkill> & { id: string; name: string; path: string }): AgentSkill {
  return {
    id: overrides.id,
    catalogId: overrides.catalogId,
    name: overrides.name,
    description: overrides.description ?? 'Installed skill',
    version: overrides.version ?? '1.0.0',
    agentType: overrides.agentType ?? 'codex',
    agentLabel: overrides.agentLabel ?? 'Codex',
    associatedAgents: overrides.associatedAgents ?? ['codex'],
    path: overrides.path,
    installedAt: overrides.installedAt ?? 1700000000000,
    isGitRepo: overrides.isGitRepo ?? true,
    gitRemoteUrl: overrides.gitRemoteUrl,
    gitBranch: overrides.gitBranch ?? 'main',
    currentCommit: overrides.currentCommit ?? '1234567',
    isDirty: overrides.isDirty ?? false,
    canUninstall: overrides.canUninstall ?? true,
    externallyManaged: overrides.externallyManaged ?? false,
    hasUpdate: overrides.hasUpdate ?? false,
    commitsBehind: overrides.commitsBehind ?? 0,
  }
}

describe('Audit Evidence Baseline (converted from 2026-10-13-source-checks.mjs)', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('F08: does not treat a skill from a different repository as installed even if catalogId matches', () => {
    const exploreStore = useSkillExploreStore()
    exploreStore.catalogSkills = [
      makeCatalogSkill({
        id: 'repo-a',
        name: 'same',
        gitUrl: 'https://github.com/a/skills',
        subPath: 'skills/one',
      }),
    ]

    const installed: AgentSkill[] = [
      makeInstalledSkill({
        id: 'codex:same',
        name: 'same',
        path: 'C:/skills/same',
        catalogId: 'repo-a',
        gitRemoteUrl: 'https://github.com/b/skills',
      }),
    ]

    expect(exploreStore.isSkillInstalled('repo-a', installed)).toBe(false)
  })

  it('F04: does not report newly installed count > 0 when pipeline has missing member or skipped target', async () => {
    setTauriInvokeHandler(async (cmd: string) => {
      if (cmd === 'install_agent_skill') {
        return {
          success: true,
          message: 'already exists',
          installedPaths: [],
        }
      }
      return []
    })

    const exploreStore = useSkillExploreStore()
    exploreStore.catalogSkills = [
      makeCatalogSkill({
        id: 'repo-a',
        name: 'same',
        gitUrl: 'https://github.com/a/skills',
        subPath: 'skills/one',
      }),
    ]
    exploreStore.openPipelineInstallDialog({
      id: 'pipeline-test',
      title: 'Test Pipeline',
      subtitle: 'Sub',
      description: 'Desc',
      targetScenario: 'Scenario',
      badge: 'Core',
      icon: 'solar:rocket-2-linear',
      skills: ['repo-a', 'missing-skill'],
    })

    const res = await exploreStore.executeInstall('C:/tmp/project-a')
    expect(res.count).toBe(0)
    expect(res.success).toBe(false)
  })

  it('F11: reflects switched project context and does not mutate saved targetAgents when draft is cancelled', async () => {
    localStorage.setItem(GIT_REPO_STORAGE_KEY, 'C:/projects/project-a')
    const exploreStore = useSkillExploreStore()
    exploreStore.updateInstallPreference({
      targetMode: 'specific_agent',
      targetAgents: ['codex'],
      addToGitignore: true,
      remember: true,
    })

    const skill = makeCatalogSkill({
      id: 'skill-1',
      name: 'skill-one',
      gitUrl: 'https://github.com/a/skills',
    })
    exploreStore.openInstallDialog(skill)

    const wrapper = mount(SkillInstallModal, {
      global: {
        plugins: [i18n],
        stubs: {
          NModal: {
            template: '<div v-if="show"><slot /></div>',
            props: ['show'],
          },
        },
      },
    })

    // Toggle 'claude' in the modal draft, then cancel without confirming
    const agentCheckboxes = wrapper.findAll('.agents-checkbox-row input[type="checkbox"]')
    expect(agentCheckboxes.length).toBeGreaterThan(1)
    await agentCheckboxes[1].trigger('change')
    exploreStore.closeInstallDialog()
    await nextTick()

    expect(exploreStore.installPreference.targetAgents).toEqual(['codex'])
  })

  it('F07 (Batch B / T06): selecting a custom tag filter in InstalledSkillsView matches skills across agent dimension', async () => {
    const skillsStore = useSkillsStore()
    const exploreStore = useSkillExploreStore()

    skillsStore.skills = [
      makeInstalledSkill({
        id: 'codex:test',
        name: 'test',
        path: 'C:/skills/test',
        agentType: 'codex',
        associatedAgents: ['codex'],
      }),
    ]
    exploreStore.addSkillCustomCategory('codex:test', 'my-tag')

    const wrapper = mount(InstalledSkillsView, {
      global: {
        plugins: [i18n],
      },
    })
    await nextTick()

    const customPill = wrapper.find('.segmented-pill--custom')
    expect(customPill.exists()).toBe(true)
    await customPill.trigger('click')
    await nextTick()

    const rows = wrapper.findAll('.installed-skill-row')
    expect(rows.length).toBe(1)
  })

  it('F09 (Batch B / T06): default popular comparator in SkillExploreView is transitive across input permutations', async () => {
    const exploreStore = useSkillExploreStore()
    const a = makeCatalogSkill({
      id: 'skill-a',
      name: 'a',
      stars: 0,
      halowakeScore: 60,
      metrics: { githubStars: 100, forks: 0, lastCommitDaysAgo: 1, issueCloseRate: 90, securityAuditPassed: true },
    })
    const b = makeCatalogSkill({
      id: 'skill-b',
      name: 'b',
      stars: 0,
      halowakeScore: 76,
      metrics: { githubStars: 10, forks: 0, lastCommitDaysAgo: 1, issueCloseRate: 90, securityAuditPassed: true },
    })
    const c = makeCatalogSkill({
      id: 'skill-c',
      name: 'c',
      stars: 0,
      halowakeScore: 92,
      metrics: { githubStars: 1, forks: 0, lastCommitDaysAgo: 1, issueCloseRate: 90, securityAuditPassed: true },
    })

    exploreStore.catalogSkills = [a, b, c]
    const wrapper1 = mount(SkillExploreView, { global: { plugins: [i18n] } })
    await nextTick()
    const order1 = wrapper1.findAll('.skill-row-title strong, .skill-code-name').map(el => el.text())

    exploreStore.catalogSkills = [c, a, b]
    await nextTick()
    const order2 = wrapper1.findAll('.skill-row-title strong, .skill-code-name').map(el => el.text())

    expect(order1).toEqual(order2)
  })

  it('U03 (Batch C / T09): system theme change event updates resolvedTheme and data-theme attribute', () => {
    let changeListener: ((e: { matches: boolean }) => void) | undefined
    const mediaMock = {
      matches: false,
      addEventListener: vi.fn((_event: string, cb: (e: { matches: boolean }) => void) => {
        changeListener = cb
      }),
      removeEventListener: vi.fn(),
    }
    vi.spyOn(window, 'matchMedia').mockImplementation(() => mediaMock as unknown as MediaQueryList)

    const preferences = usePreferencesStore()
    preferences.initPreferences()
    expect(preferences.resolvedTheme).toBe('light')
    expect(document.documentElement.getAttribute('data-theme')).toBe('light')

    mediaMock.matches = true
    if (changeListener) {
      changeListener({ matches: true })
    }

    expect(preferences.resolvedTheme).toBe('dark')
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark')
  })

  it('Review: response catalog revision is checked before accepting hits', async () => {
    setActivePinia(createPinia())
    const store = useSkillExploreStore()
    await nextTick()
    await new Promise(resolve => setTimeout(resolve, 10))
    setTauriInvokeHandler(async cmd => cmd === 'search_skill_registry' ? {
      hits: [{id:'old',score:1,skill:makeCatalogSkill({id:'old',name:'old'}),matchReason:{tier:'exact',summary:'',matchedFields:[],matchedTerms:[]}}],
      totalMatched:1,matchTier:'exact',usedFallbackEngine:false,elapsedMs:1,catalogRevision:'stale-version'
    } : null)
    store.searchQuery = 'old'
    await nextTick()
    await new Promise(resolve => setTimeout(resolve, 210))
    expect(store.ftsSearchSkills?.map(s => s.id) ?? []).not.toContain('old')
  })

  it('Review: equal-score backend relevance order remains unchanged in view', async () => {
    setActivePinia(createPinia())
    const store = useSkillExploreStore()
    await nextTick()
    await new Promise(resolve => setTimeout(resolve, 10))
    store.searchQuery = 'test'
    store.activeSort = 'relevance'
    const a = makeCatalogSkill({id:'skill-a',name:'test-a',metrics:{githubStars:1,forks:0,lastCommitDaysAgo:1,issueCloseRate:0,securityAuditPassed:false}})
    const b = makeCatalogSkill({id:'skill-b',name:'test-b',metrics:{githubStars:100,forks:0,lastCommitDaysAgo:1,issueCloseRate:0,securityAuditPassed:false}})
    store.ftsSearchSkills = [a, b]
    store.skillSearchScores = {'skill-a':100,'skill-b':100}
    const wrapper = mount(SkillExploreView,{global:{plugins:[i18n]}})
    await nextTick()
    expect(wrapper.findAll('.skill-row-title strong, .skill-code-name').map(el=>el.text())).toEqual(['test-a','test-b'])
    wrapper.unmount()
    store.searchQuery = ''
    await nextTick()
  })

  it('ReleaseReview: every advertised result is reachable after first 500 hits', async () => {
    setActivePinia(createPinia())
    const store = useSkillExploreStore()
    await nextTick()
    await new Promise(resolve => setTimeout(resolve, 10))
    store.searchQuery = 'utility'
    store.ftsSearchSkills = Array.from({ length: 500 }, (_, i) =>
      makeCatalogSkill({ id: `s-${i}`, name: `utility-${i}` })
    )
    store.totalMatched = 520
    store.hasMore = true
    store.isTruncated = false
    const wrapper = mount(SkillExploreView, { global: { plugins: [i18n] } })
    await nextTick()
    const jump = wrapper.find('.page-jump input')
    await jump.setValue('22')
    await jump.trigger('keydown.enter')
    await nextTick()
    const active = wrapper.find('.page-controls button[aria-current="page"]').text()
    expect(active).toBe('22')
    wrapper.unmount()
  })

  it('ReleaseReview: revision mismatch terminates or recovers loading', async () => {
    setActivePinia(createPinia())
    const store = useSkillExploreStore()
    await nextTick()
    await new Promise(resolve => setTimeout(resolve, 10))
    setTauriInvokeHandler(async cmd =>
      cmd === 'search_skill_registry'
        ? {
            hits: [],
            totalMatched: 0,
            matchTier: 'none',
            usedFallbackEngine: false,
            elapsedMs: 1,
            catalogRevision: 'different-server-version',
          }
        : null
    )
    store.searchQuery = 'revision-probe'
    await nextTick()
    await new Promise(resolve => setTimeout(resolve, 250))
    expect(store.isSearching).toBe(false)
  })

  it('Round3: first real snapshot revision search returns backend hits', async () => {
    setActivePinia(createPinia())
    const skill = makeCatalogSkill({ id: 'first-hit', name: 'first-hit' })
    setTauriInvokeHandler(async cmd =>
      cmd === 'load_skill_registry'
        ? { version: '1.4.1', updatedAt: 123, skills: [skill], pipelines: [] }
        : cmd === 'search_skill_registry'
          ? {
              hits: [
                {
                  id: skill.id,
                  score: 1,
                  skill,
                  matchReason: { tier: 'exact', summary: '', matchedFields: [], matchedTerms: [] },
                },
              ],
              totalMatched: 1,
              hasMore: false,
              isTruncated: false,
              matchTier: 'exact',
              usedFallbackEngine: false,
              elapsedMs: 1,
              catalogRevision: '1.4.1:123',
            }
          : null
    )
    const store = useSkillExploreStore()
    await new Promise(resolve => setTimeout(resolve, 20))
    store.searchQuery = 'first'
    await nextTick()
    await new Promise(resolve => setTimeout(resolve, 220))
    expect(store.ftsSearchSkills?.map(s => s.id)).toEqual(['first-hit'])
  })

  it('Round3: jumping beyond one incremental request actually renders target results', async () => {
    setActivePinia(createPinia())
    const store = useSkillExploreStore()
    await new Promise(resolve => setTimeout(resolve, 20))
    store.searchQuery = 'utility'
    await nextTick()
    await new Promise(resolve => setTimeout(resolve, 200))
    const items = Array.from({ length: 2200 }, (_, i) =>
      makeCatalogSkill({ id: `s-${i}`, name: `utility-${i}` })
    )
    store.ftsSearchSkills = items.slice(0, 500)
    store.totalMatched = 2200
    store.hasMore = true
    store.isTruncated = false
    setTauriInvokeHandler(async (cmd, args) => {
      if (cmd !== 'search_skill_registry') return null
      const req = (args as { request?: { offset?: number; limit?: number } })?.request ?? {}
      const offset = req.offset ?? 0
      const limit = req.limit ?? 500
      return {
        hits: items.slice(offset, offset + limit).map(skill => ({
          id: skill.id,
          skill,
          score: 1,
          matchReason: { tier: 'exact', summary: '', matchedFields: [], matchedTerms: [] },
        })),
        totalMatched: 2200,
        hasMore: offset + limit < 2200,
        isTruncated: false,
        matchTier: 'exact',
        usedFallbackEngine: false,
        elapsedMs: 1,
      }
    })
    const wrapper = mount(SkillExploreView, { global: { plugins: [i18n] } })
    await nextTick()
    const jump = wrapper.find('.page-jump input')
    await jump.setValue('92')
    await jump.trigger('keydown.enter')
    await new Promise(resolve => setTimeout(resolve, 100))
    await nextTick()
    const count = wrapper.findAll('.skill-row-title strong, .skill-code-name').length
    expect(count).toBe(16)
    wrapper.unmount()
  })

  it('Round4: newer page target is fulfilled while an earlier load is pending', async () => {
    setActivePinia(createPinia())
    const store = useSkillExploreStore()
    await new Promise(r => setTimeout(r, 20))
    store.searchQuery = 'utility'
    await nextTick()
    await new Promise(r => setTimeout(r, 200))

    const items = Array.from({ length: 2200 }, (_, i) =>
      makeCatalogSkill({ id: `s-${i}`, name: `utility-${i}` })
    )
    store.ftsSearchSkills = items.slice(0, 500)
    store.totalMatched = 2200
    store.hasMore = true

    let resolveFirst: ((value: unknown) => void) | undefined
    const calls: { offset?: number; limit?: number }[] = []
    const response = (offset: number, limit: number) => ({
      hits: items.slice(offset, offset + limit).map(skill => ({
        id: skill.id,
        skill,
        score: 1,
        matchReason: { tier: 'exact', summary: '', matchedFields: [], matchedTerms: [] },
      })),
      totalMatched: 2200,
      hasMore: offset + limit < 2200,
      isTruncated: false,
      matchTier: 'exact',
      usedFallbackEngine: false,
      elapsedMs: 1,
    })

    setTauriInvokeHandler(async (cmd, args) => {
      if (cmd !== 'search_skill_registry') return null
      const req = (args as { request?: { offset?: number; limit?: number } })?.request ?? {}
      calls.push(req)
      if (calls.length === 1) {
        return await new Promise(resolve => {
          resolveFirst = resolve
        })
      }
      return response(req.offset ?? 0, req.limit ?? 500)
    })

    const wrapper = mount(SkillExploreView, { global: { plugins: [i18n] } })
    await nextTick()
    const jump = wrapper.find('.page-jump input')
    await jump.setValue('22')
    await jump.trigger('keydown.enter')
    await nextTick()
    await jump.setValue('92')
    await jump.trigger('keydown.enter')
    await nextTick()

    resolveFirst?.(response(500, 28))
    await new Promise(r => setTimeout(r, 80))
    await nextTick()

    const rows = wrapper.findAll('.skill-row-title strong, .skill-code-name').length
    expect(calls[0]).toMatchObject({ offset: 500, limit: 28 })
    expect(calls[1]).toMatchObject({ offset: 528, limit: 1680 })
    expect(store.ftsSearchSkills?.length).toBe(2200)
    expect(store.isLoadingMore).toBe(false)
    expect(rows).toBe(16)
    wrapper.unmount()
  })

  it('Round4: changing query during pending page load does not write back old results', async () => {
    setActivePinia(createPinia())
    const store = useSkillExploreStore()
    await new Promise(r => setTimeout(r, 20))
    store.searchQuery = 'utility'
    await nextTick()
    await new Promise(r => setTimeout(r, 200))

    const items = Array.from({ length: 600 }, (_, i) =>
      makeCatalogSkill({ id: `s-${i}`, name: `utility-${i}` })
    )
    store.ftsSearchSkills = items.slice(0, 500)
    store.totalMatched = 600
    store.hasMore = true

    let resolvePending: ((value: unknown) => void) | undefined
    setTauriInvokeHandler(async cmd => {
      if (cmd === 'search_skill_registry') {
        return await new Promise(resolve => {
          resolvePending = resolve
        })
      }
      return null
    })

    const wrapper = mount(SkillExploreView, { global: { plugins: [i18n] } })
    await nextTick()
    const jump = wrapper.find('.page-jump input')
    await jump.setValue('22')
    await jump.trigger('keydown.enter')
    await nextTick()

    // Change search query while request is pending
    store.searchQuery = 'different'
    await nextTick()

    // Resolve old pagination response
    resolvePending?.({
      hits: items.slice(500, 528).map(skill => ({
        id: skill.id,
        skill,
        score: 1,
        matchReason: { tier: 'exact', summary: '', matchedFields: [], matchedTerms: [] },
      })),
      totalMatched: 600,
      hasMore: true,
      isTruncated: false,
      matchTier: 'exact',
      usedFallbackEngine: false,
      elapsedMs: 1,
    })
    await new Promise(r => setTimeout(r, 60))
    await nextTick()

    expect(store.ftsSearchSkills?.some(s => s.id === 's-500')).toBe(false)
    expect(store.ftsSearchSkills?.length).toBe(500)
    wrapper.unmount()
  })

  it('Round4: failed subsequent page exposes a visible retry action and clicking retry succeeds', async () => {
    setActivePinia(createPinia())
    const store = useSkillExploreStore()
    await new Promise(r => setTimeout(r, 20))
    store.searchQuery = 'utility'
    await nextTick()
    await new Promise(r => setTimeout(r, 200))

    const items = Array.from({ length: 520 }, (_, i) =>
      makeCatalogSkill({ id: `s-${i}`, name: `utility-${i}` })
    )
    store.ftsSearchSkills = items.slice(0, 500)
    store.totalMatched = 520
    store.hasMore = true

    let shouldFail = true
    setTauriInvokeHandler(async (cmd, args) => {
      if (cmd !== 'search_skill_registry') return null
      if (shouldFail) {
        throw new Error('pagination-offline-probe')
      }
      const req = (args as { request?: { offset?: number; limit?: number } })?.request ?? {}
      const offset = req.offset ?? 0
      const limit = req.limit ?? 20
      return {
        hits: items.slice(offset, offset + limit).map(skill => ({
          id: skill.id,
          skill,
          score: 1,
          matchReason: { tier: 'exact', summary: '', matchedFields: [], matchedTerms: [] },
        })),
        totalMatched: 520,
        hasMore: offset + limit < 520,
        isTruncated: false,
        matchTier: 'exact',
        usedFallbackEngine: false,
        elapsedMs: 1,
      }
    })

    const wrapper = mount(SkillExploreView, { global: { plugins: [i18n] } })
    await nextTick()
    const jump = wrapper.find('.page-jump input')
    await jump.setValue('22')
    await jump.trigger('keydown.enter')
    await new Promise(r => setTimeout(r, 40))
    await nextTick()

    expect(store.loadMoreError).toBe('pagination-offline-probe')
    const retryButtons = wrapper.findAll('button').filter(b => /重试|retry/i.test(b.text()))
    expect(retryButtons.length).toBeGreaterThan(0)

    // Now succeed on retry
    shouldFail = false
    await retryButtons[0].trigger('click')
    await new Promise(r => setTimeout(r, 60))
    await nextTick()

    expect(store.loadMoreError).toBeNull()
    expect(store.isLoadingMore).toBe(false)
    expect(store.ftsSearchSkills?.length).toBe(520)
    const rows = wrapper.findAll('.skill-row-title strong, .skill-code-name').length
    expect(rows).toBe(16) // 520 - 504 = 16 items on page 22
    wrapper.unmount()
  })

  it('Round4: changing filters clears old pagination error', async () => {
    setActivePinia(createPinia())
    const store = useSkillExploreStore()
    await new Promise(r => setTimeout(r, 20))
    store.searchQuery = 'utility'
    await nextTick()
    await new Promise(r => setTimeout(r, 200))
    store.ftsSearchSkills = Array.from({ length: 500 }, (_, i) =>
      makeCatalogSkill({ id: `s-${i}`, name: `utility-${i}` })
    )
    store.totalMatched = 520
    store.hasMore = true

    setTauriInvokeHandler(async cmd => {
      if (cmd === 'search_skill_registry') throw new Error('filter-error-probe')
      return null
    })

    const wrapper = mount(SkillExploreView, { global: { plugins: [i18n] } })
    await nextTick()
    const jump = wrapper.find('.page-jump input')
    await jump.setValue('22')
    await jump.trigger('keydown.enter')
    await new Promise(r => setTimeout(r, 40))
    await nextTick()

    expect(store.loadMoreError).toBe('filter-error-probe')

    // Change category filter
    store.selectedCategory = 'testing'
    await nextTick()
    expect(store.loadMoreError).toBeNull()
    wrapper.unmount()
  })
})


