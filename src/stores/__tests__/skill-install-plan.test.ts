import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useSkillExploreStore } from '@/stores/skill-explore'
import { useWorkspaceStore } from '@/stores/workspace'
import * as skillsService from '@/services/skills/skills-service'
import type { CatalogSkill } from '@/types/skills'

vi.mock('@/services/skills/skills-service', () => ({
  installAgentSkill: vi.fn(),
  scanAgentSkills: vi.fn(),
  checkSkillsUpdates: vi.fn(),
  updateAgentSkill: vi.fn(),
  uninstallAgentSkill: vi.fn(),
  openSkillFolder: vi.fn(),
  listSkillInstallHistory: vi.fn(),
}))

function createCatalogSkill(overrides: Partial<CatalogSkill> & { id: string; name: string }): CatalogSkill {
  return {
    id: overrides.id,
    name: overrides.name,
    titleZh: overrides.titleZh ?? overrides.name,
    description: overrides.description ?? 'Desc',
    stage: overrides.stage ?? 'coding',
    category: overrides.category ?? 'general',
    author: overrides.author ?? { name: 'acme' },
    gitUrl: overrides.gitUrl ?? 'https://github.com/acme/skills',
    subPath: overrides.subPath,
    stars: 0,
    halowakeScore: 85,
    badge: 'verified',
    metrics: {
      githubStars: 100,
      forks: 10,
      lastCommitDaysAgo: 2,
      issueCloseRate: 95,
      securityAuditPassed: true,
    },
    tags: ['test'],
    recommendedWith: [],
    howToUse: 'Use',
    compatibleAgents: ['codex', 'claude'],
    isBundle: overrides.isBundle ?? false,
    subSkills: overrides.subSkills,
  }
}

describe('useSkillExploreStore install plan, batch semantics, and workspace sync (T04)', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    localStorage.clear()
    vi.clearAllMocks()
  })

  it('reports status=skipped and count=0 when all targets already exist (F03)', async () => {
    const exploreStore = useSkillExploreStore()
    const skill = createCatalogSkill({
      id: 'acme-skill',
      name: 'acme-skill',
      gitUrl: 'https://github.com/acme/skills',
    })
    exploreStore.openInstallDialog(skill)

    vi.mocked(skillsService.installAgentSkill).mockResolvedValueOnce({
      success: true,
      status: 'skipped',
      message: '已跳过已存在目录 (1 项)',
      installedPaths: [],
      skippedPaths: ['/mock/home/.codex/skills/acme-skill'],
    })

    const res = await exploreStore.executeInstall('/workspace/demo')
    expect(res.status).toBe('skipped')
    expect(res.success).toBe(true)
    expect(res.count).toBe(0)
    expect(res.installedCount).toBe(0)
    expect(res.skippedCount).toBe(1)
    expect(res.message).toContain('无需重复安装')
    expect(exploreStore.installModalVisible).toBe(false)
  })

  it('blocks pipeline installation before IPC when any member skill is missing from catalog (F04)', async () => {
    const exploreStore = useSkillExploreStore()
    exploreStore.catalogSkills = [
      createCatalogSkill({
        id: 'existing-skill',
        name: 'existing-skill',
      }),
    ]
    exploreStore.openPipelineInstallDialog({
      id: 'pipe-1',
      title: 'Broken Pipeline',
      subtitle: 'Sub',
      description: 'Desc',
      targetScenario: 'Test',
      badge: 'Core',
      icon: 'solar:rocket-2-linear',
      skills: ['existing-skill', 'ghost-skill-id'],
    })

    const res = await exploreStore.executeInstall('/workspace/demo')
    expect(res.status).toBe('failed')
    expect(res.success).toBe(false)
    expect(res.count).toBe(0)
    expect(res.missingCount).toBe(1)
    expect(res.message).toContain('ghost-skill-id')
    expect(skillsService.installAgentSkill).not.toHaveBeenCalled()
    expect(exploreStore.installModalVisible).toBe(true)
  })

  it('keeps modal open on partial bundle install and retries only failed items on retry', async () => {
    const exploreStore = useSkillExploreStore()
    const bundle = createCatalogSkill({
      id: 'acme-bundle',
      name: 'acme-bundle',
      isBundle: true,
      subSkills: [
        {
          id: 'sub-ok',
          name: 'sub-ok',
          titleZh: '子技能一',
          description: 'OK sub',
          subPath: 'skills/sub-ok',
        },
        {
          id: 'sub-fail',
          name: 'sub-fail',
          titleZh: '子技能二',
          description: 'Fail sub',
          subPath: 'skills/sub-fail',
        },
      ],
    })
    exploreStore.openInstallDialog(bundle)

    vi.mocked(skillsService.installAgentSkill)
      .mockResolvedValueOnce({
        success: true,
        status: 'installed',
        message: 'installed sub-ok',
        installedPaths: ['/mock/home/.codex/skills/sub-ok'],
      })
      .mockResolvedValueOnce({
        success: false,
        message: 'network timeout while cloning sub-fail',
        installedPaths: [],
      })

    const firstBatch = await exploreStore.executeInstall('/workspace/demo')
    expect(firstBatch.status).toBe('partial')
    expect(firstBatch.success).toBe(false)
    expect(firstBatch.count).toBe(1)
    expect(firstBatch.installedCount).toBe(1)
    expect(firstBatch.failedCount).toBe(1)
    expect(firstBatch.installedPaths).toEqual(['/mock/home/.codex/skills/sub-ok'])
    // Modal must stay open so user can inspect partial paths and retry failed items
    expect(exploreStore.installModalVisible).toBe(true)

    // Retry only failed items
    vi.mocked(skillsService.installAgentSkill).mockClear()
    vi.mocked(skillsService.installAgentSkill).mockResolvedValueOnce({
      success: true,
      status: 'installed',
      message: 'installed sub-fail',
      installedPaths: ['/mock/home/.codex/skills/sub-fail'],
    })

    const retryBatch = await exploreStore.executeInstall({
      projectCwd: '/workspace/demo',
      onlyFailed: true,
    })
    expect(skillsService.installAgentSkill).toHaveBeenCalledTimes(1)
    expect(skillsService.installAgentSkill).toHaveBeenCalledWith(
      expect.objectContaining({ subSkillId: 'sub-fail' }),
      expect.any(Function)
    )
    expect(retryBatch.status).toBe('installed')
    expect(retryBatch.success).toBe(true)
    expect(retryBatch.count).toBe(2)
    expect(retryBatch.installedPaths).toEqual([
      '/mock/home/.codex/skills/sub-ok',
      '/mock/home/.codex/skills/sub-fail',
    ])
    expect(exploreStore.installModalVisible).toBe(false)
  })

  it('pre-populates history reinstall override without mutating saved installPreference and syncs workspaceStore (F11, F12)', () => {
    const exploreStore = useSkillExploreStore()
    const workspaceStore = useWorkspaceStore()

    exploreStore.updateInstallPreference(
      {
        targetMode: 'global',
        targetAgents: ['codex'],
        addToGitignore: false,
        remember: true,
      },
      true
    )

    workspaceStore.setCurrentProjectPath('/workspace/project-new')
    expect(workspaceStore.currentProjectPath).toBe('/workspace/project-new')

    exploreStore.openReinstallFromHistory({
      id: 42,
      skillId: 'acme-skill',
      subSkillId: 'sub-alpha',
      displayName: 'Alpha Sub',
      gitUrl: 'https://github.com/acme/skills',
      subPath: 'skills/sub-alpha',
      targetMode: 'project',
      targetAgents: ['claude', 'cursor'],
      projectCwd: '/workspace/history-project',
      status: 'failed',
      message: 'clone failed',
      installedPaths: [],
      existingPaths: [],
      createdAt: Date.now(),
    })

    expect(exploreStore.installModalVisible).toBe(true)
    expect(exploreStore.reinstallPreferenceOverride?.targetMode).toBe('project')
    expect(exploreStore.reinstallPreferenceOverride?.targetAgents).toEqual(['claude', 'cursor'])
    expect(exploreStore.reinstallPreferenceOverride?.customProjectPath).toBe('/workspace/history-project')
    // Saved preference must remain untouched
    expect(exploreStore.installPreference.targetMode).toBe('global')
    expect(exploreStore.installPreference.targetAgents).toEqual(['codex'])
  })

  it('handles S01: initial partial -> retry IPC rejects -> final retry succeeds (preserves outcomes, does not false-succeed or close modal)', async () => {
    const exploreStore = useSkillExploreStore()
    const skill = createCatalogSkill({
      id: 'multi-agent-skill',
      name: 'multi-agent-skill',
      gitUrl: 'https://github.com/acme/multi-agent',
    })
    exploreStore.openInstallDialog(skill)
    exploreStore.updateInstallPreference({
      targetMode: 'specific_agent',
      targetAgents: ['codex', 'claude'],
      addToGitignore: false,
      remember: false,
    })

    // Attempt 1: Codex succeeds, Claude fails -> partial result
    vi.mocked(skillsService.installAgentSkill).mockResolvedValueOnce({
      success: false,
      status: 'partial',
      message: 'Claude 安装失败：目录不可写',
      installedPaths: ['/mock/home/.codex/skills/multi-agent-skill'],
      skippedPaths: [],
      targetOutcomes: [
        {
          targetAgent: 'codex',
          targetPath: '/mock/home/.codex/skills/multi-agent-skill',
          status: 'installed',
        },
        {
          targetAgent: 'claude',
          targetPath: '/mock/home/.claude/skills/multi-agent-skill',
          status: 'failed',
          error: 'Claude 安装失败：目录不可写',
        },
      ],
    })

    const firstBatch = await exploreStore.executeInstall('/workspace/demo')
    expect(firstBatch.status).toBe('partial')
    expect(firstBatch.success).toBe(false)
    expect(firstBatch.installedCount).toBe(1)
    expect(firstBatch.failedCount).toBe(1)
    expect(firstBatch.installedPaths).toEqual(['/mock/home/.codex/skills/multi-agent-skill'])
    expect(firstBatch.operationId).toMatch(/^op-[0-9a-f-]+/)
    expect(firstBatch.retryOfOperationId).toBeUndefined()
    expect(skillsService.installAgentSkill).toHaveBeenLastCalledWith(
      expect.objectContaining({
        operationId: firstBatch.operationId,
        retryOfOperationId: undefined,
      }),
      expect.any(Function)
    )
    expect(exploreStore.installModalVisible).toBe(true)

    // Attempt 2: Retry with onlyFailed: true. Target should only be Claude.
    // In this attempt, the IPC call throws (e.g. backend disconnected / crash / reject)
    vi.mocked(skillsService.installAgentSkill).mockClear()
    vi.mocked(skillsService.installAgentSkill).mockRejectedValueOnce(
      new Error('IPC transport disconnected')
    )

    const secondBatch = await exploreStore.executeInstall({
      projectCwd: '/workspace/demo',
      onlyFailed: true,
    })

    // Verify Attempt 2 called with only failed agent (claude)
    expect(skillsService.installAgentSkill).toHaveBeenCalledTimes(1)
    expect(skillsService.installAgentSkill).toHaveBeenCalledWith(
      expect.objectContaining({
        targetAgents: ['claude'],
        operationId: secondBatch.operationId,
        retryOfOperationId: firstBatch.operationId,
      }),
      expect.any(Function)
    )

    // S01 requirement: MUST NOT report success: true, MUST NOT report status: 'installed', MUST keep modal open!
    expect(secondBatch.status).toBe('partial')
    expect(secondBatch.success).toBe(false)
    expect(secondBatch.installedCount).toBe(1)
    expect(secondBatch.failedCount).toBe(1)
    expect(secondBatch.installedPaths).toEqual(['/mock/home/.codex/skills/multi-agent-skill'])
    expect(secondBatch.operationId).toMatch(/^op-[0-9a-f-]+/)
    expect(secondBatch.operationId).not.toBe(firstBatch.operationId)
    expect(secondBatch.retryOfOperationId).toBe(firstBatch.operationId)
    expect(secondBatch.items[0].targetOutcomes).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ targetAgent: 'codex', status: 'installed' }),
        expect.objectContaining({ targetAgent: 'claude', status: 'failed', error: 'IPC transport disconnected' }),
      ])
    )
    expect(exploreStore.installModalVisible).toBe(true)

    // Attempt 3: Retry again. Target should STILL only be Claude.
    // In this attempt, Claude succeeds!
    vi.mocked(skillsService.installAgentSkill).mockClear()
    vi.mocked(skillsService.installAgentSkill).mockResolvedValueOnce({
      success: true,
      status: 'installed',
      message: 'Claude 安装成功',
      installedPaths: ['/mock/home/.claude/skills/multi-agent-skill'],
      skippedPaths: [],
      targetOutcomes: [
        {
          targetAgent: 'claude',
          targetPath: '/mock/home/.claude/skills/multi-agent-skill',
          status: 'installed',
        },
      ],
    })

    const thirdBatch = await exploreStore.executeInstall({
      projectCwd: '/workspace/demo',
      onlyFailed: true,
    })

    expect(skillsService.installAgentSkill).toHaveBeenCalledTimes(1)
    expect(skillsService.installAgentSkill).toHaveBeenCalledWith(
      expect.objectContaining({
        targetAgents: ['claude'],
        operationId: thirdBatch.operationId,
        retryOfOperationId: secondBatch.operationId,
      }),
      expect.any(Function)
    )

    // Attempt 3 succeeds: combined results from Codex (Attempt 1) and Claude (Attempt 3)
    expect(thirdBatch.status).toBe('installed')
    expect(thirdBatch.success).toBe(true)
    expect(thirdBatch.installedCount).toBe(2)
    expect(thirdBatch.failedCount).toBe(0)
    expect(thirdBatch.installedPaths).toEqual([
      '/mock/home/.codex/skills/multi-agent-skill',
      '/mock/home/.claude/skills/multi-agent-skill',
    ])
    expect(thirdBatch.operationId).toMatch(/^op-[0-9a-f-]+/)
    expect(thirdBatch.operationId).not.toBe(secondBatch.operationId)
    expect(thirdBatch.operationId).not.toBe(firstBatch.operationId)
    expect(thirdBatch.retryOfOperationId).toBe(secondBatch.operationId)
    expect(exploreStore.installModalVisible).toBe(false)
  })

  it('handles A-ID: generates distinct operationIds across independent store sessions (no op-1 collision)', async () => {
    vi.mocked(skillsService.installAgentSkill).mockResolvedValue({
      success: true,
      status: 'installed',
      message: 'installed',
      installedPaths: ['/mock/skills/s1'],
    })

    // Session A (independent Pinia)
    setActivePinia(createPinia())
    const storeA = useSkillExploreStore()
    storeA.openInstallDialog(createCatalogSkill({ id: 's1', name: 's1', gitUrl: 'https://example.com/s1' }))
    const batchA = await storeA.executeInstall('/workspace/demo')

    // Session B (fresh Pinia instance simulating app restart / another window)
    setActivePinia(createPinia())
    const storeB = useSkillExploreStore()
    storeB.openInstallDialog(createCatalogSkill({ id: 's1', name: 's1', gitUrl: 'https://example.com/s1' }))
    const batchB = await storeB.executeInstall('/workspace/demo')

    expect(batchA.operationId).toBeTruthy()
    expect(batchB.operationId).toBeTruthy()
    expect(batchA.operationId).not.toBe(batchB.operationId)
    expect(batchA.operationId).toMatch(/^op-[0-9a-f-]+/)
    expect(batchB.operationId).toMatch(/^op-[0-9a-f-]+/)
  })

  it('handles A-ID: uses the same operationId across all items, progress and batchResult in a single batch', async () => {
    const progressOps: (string | undefined)[] = []
    vi.mocked(skillsService.installAgentSkill).mockImplementation(async (_args, onProgress) => {
      onProgress({
        phase: 'downloading',
        message: 'downloading...',
        percent: 50,
      })
      const store = useSkillExploreStore()
      progressOps.push(store.installProgress?.operationId)
      return {
        success: true,
        status: 'installed',
        message: `installed ${_args.subSkillId}`,
        installedPaths: [`/mock/path/${_args.subSkillId}`],
      }
    })

    const exploreStore = useSkillExploreStore()
    const bundleSkill = createCatalogSkill({
      id: 'bundle-x',
      name: 'bundle-x',
      gitUrl: 'https://github.com/acme/bundle-x',
      isBundle: true,
      subSkills: [
        { id: 'sub-a', name: 'sub-a', titleZh: 'Sub A', description: 'Desc A', subPath: 'skills/sub-a' },
        { id: 'sub-b', name: 'sub-b', titleZh: 'Sub B', description: 'Desc B', subPath: 'skills/sub-b' },
      ],
    })

    exploreStore.openInstallDialog(bundleSkill)
    exploreStore.selectedBundleSubSkillIds = ['sub-a', 'sub-b']

    const batch = await exploreStore.executeInstall('/workspace/demo')

    expect(skillsService.installAgentSkill).toHaveBeenCalledTimes(2)
    const call1Args = vi.mocked(skillsService.installAgentSkill).mock.calls[0][0]
    const call2Args = vi.mocked(skillsService.installAgentSkill).mock.calls[1][0]

    expect(call1Args.operationId).toBe(batch.operationId)
    expect(call2Args.operationId).toBe(batch.operationId)
    expect(batch.operationId).toMatch(/^op-[0-9a-f-]+/)
    expect(progressOps.length).toBe(2)
    expect(progressOps[0]).toBe(batch.operationId)
    expect(progressOps[1]).toBe(batch.operationId)
  })
})
