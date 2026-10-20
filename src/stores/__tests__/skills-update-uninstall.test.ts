import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useSkillsStore } from '@/stores/skills'
import * as skillsService from '@/services/skills/skills-service'
import type { AgentSkill } from '@/types/skills'

vi.mock('@/services/skills/skills-service', () => ({
  scanAgentSkills: vi.fn(),
  checkSkillsUpdates: vi.fn(),
  updateAgentSkill: vi.fn(),
  uninstallAgentSkill: vi.fn(),
  openSkillFolder: vi.fn(),
}))

function createMockSkill(overrides: Partial<AgentSkill> = {}): AgentSkill {
  return {
    id: overrides.id ?? 'codex:skill-a',
    name: overrides.name ?? 'skill-a',
    description: overrides.description ?? 'Test skill',
    path: overrides.path ?? '/tmp/skills/skill-a',
    agentType: overrides.agentType ?? 'codex',
    agentLabel: overrides.agentLabel ?? 'Codex',
    associatedAgents: overrides.associatedAgents ?? ['codex'],
    hasUpdate: overrides.hasUpdate ?? false,
    commitsBehind: overrides.commitsBehind ?? 0,
    isGitRepo: overrides.isGitRepo ?? false,
    isDirty: overrides.isDirty ?? false,
    canUninstall: overrides.canUninstall ?? true,
    externallyManaged: overrides.externallyManaged ?? false,
    management: overrides.management ?? 'halowake',
    ...overrides,
  }
}

describe('useSkillsStore update/uninstall safety & scope generation (T03)', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('rejects auto-update for unmanaged and external skills without invoking IPC', async () => {
    const store = useSkillsStore()
    const unmanaged = createMockSkill({
      id: 'codex:custom-skill',
      name: 'impeccable',
      path: '/tmp/skills/impeccable',
      management: 'unmanaged',
      hasUpdate: true,
    })
    const managed = createMockSkill({
      id: 'codex:managed-skill',
      name: 'managed-skill',
      path: '/tmp/skills/managed-skill',
      management: 'halowake',
      hasUpdate: true,
    })

    store.skills = [unmanaged, managed]
    expect(store.updateCount).toBe(1)

    const unmanagedRes = await store.updateSingle(unmanaged)
    expect(unmanagedRes.success).toBe(false)
    expect(unmanagedRes.message).toContain('不支持自动更新')
    expect(skillsService.updateAgentSkill).not.toHaveBeenCalled()

    vi.mocked(skillsService.updateAgentSkill).mockResolvedValueOnce({
      skillId: managed.id,
      success: true,
      message: '已更新到最新版本',
      newCommit: 'abcdef123456',
    })

    const batchRes = await store.updateAll()
    expect(batchRes).toHaveLength(1)
    expect(batchRes[0].skillId).toBe(managed.id)
    expect(skillsService.updateAgentSkill).toHaveBeenCalledTimes(1)
    expect(skillsService.updateAgentSkill).toHaveBeenCalledWith(managed.path, false)
  })

  it('blocks concurrent update and uninstall on the same skill path', async () => {
    const store = useSkillsStore()
    const skill = createMockSkill({
      id: 'codex:concurrent-skill',
      path: '/tmp/skills/concurrent-skill',
      hasUpdate: true,
    })
    store.skills = [skill]

    let resolveUpdate!: (val: skillsService.SkillUpdateResult) => void
    vi.mocked(skillsService.updateAgentSkill).mockImplementationOnce(
      () =>
        new Promise(resolve => {
          resolveUpdate = resolve
        })
    )

    const updatePromise = store.updateSingle(skill)
    expect(store.updatingPaths).toContain(skill.path)

    await expect(store.uninstallSingle(skill)).rejects.toThrow('正在更新中')
    expect(skillsService.uninstallAgentSkill).not.toHaveBeenCalled()

    resolveUpdate({
      skillId: skill.id,
      success: true,
      message: 'ok',
    })
    await updatePromise
    expect(store.updatingPaths).not.toContain(skill.path)
  })

  it('discards stale checkUpdates results when project scope changes mid-flight (scopeGeneration)', async () => {
    const store = useSkillsStore()

    let resolveCheckProjectA!: (skills: AgentSkill[]) => void
    vi.mocked(skillsService.checkSkillsUpdates).mockImplementationOnce(
      () =>
        new Promise(resolve => {
          resolveCheckProjectA = resolve
        })
    )

    const checkAPromise = store.checkUpdates('/workspace/project-a')

    const projectBSkills = [
      createMockSkill({
        id: 'project:skill-b',
        name: 'skill-b',
        path: '/workspace/project-b/.agents/skills/skill-b',
        agentType: 'project',
      }),
    ]
    vi.mocked(skillsService.scanAgentSkills).mockResolvedValueOnce(projectBSkills)
    await store.scan('/workspace/project-b')
    expect(store.skills.map(s => s.id)).toEqual(['project:skill-b'])

    // Now stale checkUpdates from project-a resolves
    resolveCheckProjectA([
      createMockSkill({
        id: 'project:skill-a',
        name: 'skill-a',
        path: '/workspace/project-a/.agents/skills/skill-a',
        agentType: 'project',
        hasUpdate: true,
      }),
    ])
    await checkAPromise

    // Project B skills must not be overwritten by stale Project A response
    expect(store.skills.map(s => s.id)).toEqual(['project:skill-b'])
  })

  it('distinguishes updateCount, unknownCount, and unsupportedCount and preserves updateStatus across scan', async () => {
    const store = useSkillsStore()

    const checkedList: AgentSkill[] = [
      createMockSkill({
        id: 'codex:s-up',
        name: 's-up',
        path: '/tmp/s-up',
        management: 'halowake',
        hasUpdate: true,
        updateStatus: 'update_available',
        updateSummary: '来源仓库有新提交（子技能目录未必变更）',
      }),
      createMockSkill({
        id: 'codex:s-err',
        name: 's-err',
        path: '/tmp/s-err',
        management: 'halowake',
        hasUpdate: false,
        updateStatus: 'error',
        updateError: '执行命令超时 (25s): git ls-remote',
      }),
      createMockSkill({
        id: 'codex:s-unmanaged',
        name: 's-unmanaged',
        path: '/tmp/s-unmanaged',
        management: 'unmanaged',
        hasUpdate: false,
        updateStatus: 'unsupported',
      }),
    ]

    vi.mocked(skillsService.checkSkillsUpdates).mockResolvedValueOnce(checkedList)
    const res = await store.checkUpdates('/workspace/project-a')
    expect(res.updateCount).toBe(1)
    expect(res.unknownCount).toBe(1)
    expect(res.unsupportedCount).toBe(1)

    // Subsequent scan preserves updateStatus and updateError
    vi.mocked(skillsService.scanAgentSkills).mockResolvedValueOnce(
      checkedList.map(s => ({
        ...s,
        hasUpdate: false,
        updateStatus: undefined,
        updateError: undefined,
      }))
    )
    await store.scan('/workspace/project-a')
    const errSkill = store.skills.find(s => s.id === 'codex:s-err')
    expect(errSkill?.updateStatus).toBe('error')
    expect(errSkill?.updateError).toContain('超时')
  })
})
