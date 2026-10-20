import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import {
  getLastSyncTime,
  syncOnlineRegistry,
  validateAndSanitizeRemoteRegistry,
} from '@/services/skills/skill-catalog-service'
import { useSkillExploreStore } from '@/stores/skill-explore'
import { setFetchHandler, setTauriInvokeHandler } from '@/test/setup'

describe('skill-catalog-service & useSkillExploreStore registry sync consistency (T05)', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    localStorage.clear()
    vi.clearAllMocks()
  })

  it('validates and sanitizes remote registry entries, rejecting traversal subPath and invalid gitUrl (F13)', () => {
    const raw = {
      version: '2.0.0',
      updatedAt: 1700000000000,
      skills: [
        {
          id: 'valid-one',
          name: 'valid-one',
          titleZh: '合法技能',
          description: 'A valid skill',
          stage: 'coding',
          gitUrl: 'https://github.com/acme/skills',
          subPath: 'skills/valid-one',
          tags: ['vue', 123],
        },
        {
          id: 'bad-traversal',
          name: 'bad-traversal',
          gitUrl: 'https://github.com/acme/skills',
          subPath: '../etc/passwd',
        },
        {
          id: 'bad-url',
          name: 'bad-url',
          gitUrl: 'javascript:alert(1)',
        },
      ],
      pipelines: [
        {
          id: 'pipe-valid',
          title: 'Valid Pipe',
          skills: ['valid-one', 'bad-traversal', 'non-existent'],
        },
      ],
    }

    const result = validateAndSanitizeRemoteRegistry(raw)
    expect(result.rejectedSkillCount).toBe(2)
    expect(result.skills).toHaveLength(1)
    expect(result.skills[0].id).toBe('valid-one')
    expect(result.skills[0].tags).toEqual(['vue'])
    expect(result.pipelines).toHaveLength(1)
    expect(result.pipelines[0].skills).toEqual(['valid-one', 'bad-traversal', 'non-existent'])
  })

  it('does not publish remote catalog or update lastSyncTime when SQLite persistence fails (F06)', async () => {
    const existingPersistedSkill = {
      id: 'persisted-old',
      name: 'persisted-old',
      titleZh: '已落盘旧技能',
      description: 'Persisted in SQLite',
      stage: 'coding',
      category: 'general',
      author: { name: 'acme' },
      gitUrl: 'https://github.com/acme/persisted',
      stars: 10,
      halowakeScore: 80,
      badge: 'official',
      metrics: {
        githubStars: 10,
        forks: 1,
        lastCommitDaysAgo: 1,
        issueCloseRate: 90,
        securityAuditPassed: true,
      },
      tags: ['old'],
      recommendedWith: [],
      compatibleAgents: ['codex'],
      howToUse: 'Use',
      isBundle: false,
    }

    setTauriInvokeHandler(async (cmd: string) => {
      if (cmd === 'load_skill_registry') {
        return {
          version: '1.0.0',
          updatedAt: 1700000000000,
          skills: [existingPersistedSkill],
          pipelines: [],
        }
      }
      if (cmd === 'save_skill_registry') {
        throw new Error('SQLite disk full or database locked')
      }
      return null
    })

    setFetchHandler(async () => {
      return new Response(
        JSON.stringify({
          version: '2.0.0',
          updatedAt: 1700000500000,
          skills: [
            {
              id: 'unpersisted-remote',
              name: 'unpersisted-remote',
              titleZh: '未落盘远端技能',
              description: 'Fetched from remote but failed to save to SQLite',
              stage: 'coding',
              gitUrl: 'https://github.com/acme/remote',
            },
          ],
          pipelines: [],
        }),
        { status: 200, headers: { 'Content-Type': 'application/json' } }
      )
    })

    const exploreStore = useSkillExploreStore()
    exploreStore.catalogSkills = [existingPersistedSkill as never]
    const initialRevision = exploreStore.catalogRevision

    const syncRes = await exploreStore.syncRegistry()
    expect(syncRes.status).toBe('persist_failed')
    expect(syncRes.success).toBe(false)
    expect(syncRes.message).toContain('写入本地索引库失败')
    expect(getLastSyncTime()).toBeNull()
    expect(exploreStore.catalogRevision).toBe(initialRevision)
    // In-memory catalogSkills must remain consistent with SQLite, not replaced by unpersisted remote data
    expect(exploreStore.catalogSkills.map(s => s.id)).toEqual(['persisted-old'])
  })

  it('publishes remote catalog, updates lastSyncTime, and increments catalogRevision only after SQLite write succeeds', async () => {
    let savedRegistryPayload: unknown = null

    setTauriInvokeHandler(async (cmd: string, args?: Record<string, unknown>) => {
      if (cmd === 'save_skill_registry') {
        savedRegistryPayload = args?.registry
        return null
      }
      if (cmd === 'load_skill_registry') {
        return savedRegistryPayload
      }
      return null
    })

    setFetchHandler(async () => {
      return new Response(
        JSON.stringify({
          version: '2.1.0',
          updatedAt: 1700000900000,
          skills: [
            {
              id: 'synced-new-skill',
              name: 'synced-new-skill',
              titleZh: '同步成功新技能',
              description: 'Persisted and published',
              stage: 'testing',
              gitUrl: 'https://github.com/acme/synced',
            },
          ],
          pipelines: [],
        }),
        { status: 200, headers: { 'Content-Type': 'application/json' } }
      )
    })

    const exploreStore = useSkillExploreStore()
    const prevRevision = exploreStore.catalogRevision

    const syncRes = await exploreStore.syncRegistry()
    expect(syncRes.status).toBe('synced')
    expect(syncRes.success).toBe(true)
    expect(syncRes.updatedCount).toBe(1)
    expect(getLastSyncTime()).not.toBeNull()
    expect(exploreStore.catalogRevision).toBe(prevRevision + 1)
    expect(exploreStore.catalogSkills.map(s => s.id)).toEqual(['synced-new-skill'])
  })

  it('returns cached_fallback when remote fetch fails but SQLite cache is available', async () => {
    setTauriInvokeHandler(async (cmd: string) => {
      if (cmd === 'load_skill_registry') {
        return {
          version: '1.0.0',
          updatedAt: 1700000000000,
          skills: [
            {
              id: 'cached-skill',
              name: 'cached-skill',
              titleZh: '缓存技能',
              description: 'From SQLite cache',
              stage: 'coding',
              gitUrl: 'https://github.com/acme/cached',
            },
          ],
          pipelines: [],
        }
      }
      return null
    })

    setFetchHandler(async () => {
      throw new Error('Network unreachable')
    })

    const res = await syncOnlineRegistry()
    expect(res.status).toBe('cached_fallback')
    expect(res.success).toBe(false)
    expect(res.totalCount).toBe(1)
    expect(res.skills[0].id).toBe('cached-skill')
  })

  it('preserves full declared pipeline dependencies and blocks installation before IPC if members are missing (R02)', async () => {
    let installIpcInvoked = false

    setTauriInvokeHandler(async (cmd: string) => {
      if (cmd === 'load_skill_registry') {
        return null
      }
      if (cmd === 'save_skill_registry') {
        return null
      }
      if (cmd === 'install_agent_skill') {
        installIpcInvoked = true
        return {
          success: true,
          status: 'installed',
          message: 'OK',
          installedPaths: ['/tmp/installed'],
        }
      }
      return null
    })

    setFetchHandler(async () => {
      return new Response(
        JSON.stringify({
          version: '2.0.0',
          updatedAt: 1700000000000,
          skills: [
            {
              id: 'skill-alpha',
              name: 'skill-alpha',
              titleZh: '技能 Alpha',
              description: 'Alpha skill',
              stage: 'coding',
              gitUrl: 'https://github.com/acme/alpha',
            },
          ],
          pipelines: [
            {
              id: 'pipe-full-pack',
              title: '完整套餐',
              skills: ['skill-alpha', 'missing-beta'],
            },
          ],
        }),
        { status: 200, headers: { 'Content-Type': 'application/json' } }
      )
    })

    const exploreStore = useSkillExploreStore()
    const syncRes = await exploreStore.syncRegistry()
    expect(syncRes.success).toBe(true)

    // Verify pipeline declared dependencies are not shortened
    const pipeline = exploreStore.pipelines.find(p => p.id === 'pipe-full-pack')
    expect(pipeline).toBeDefined()
    expect(pipeline!.skills).toEqual(['skill-alpha', 'missing-beta'])

    // Open install modal for this pipeline
    exploreStore.openPipelineInstallDialog(pipeline!)
    expect(exploreStore.pendingPipeline).toBe(pipeline)

    // Execute install
    const batchResult = await exploreStore.executeInstall({
      preference: {
        targetMode: 'global',
        targetAgents: ['codex'],
        addToGitignore: false,
        remember: false,
      },
    })

    // Batch must fail because missing-beta is missing from catalog
    expect(batchResult.success).toBe(false)
    expect(batchResult.status).toBe('failed')
    expect(batchResult.missingCount).toBe(1)
    expect(batchResult.message).toContain('missing-beta')
    const missingItem = batchResult.items.find(i => i.skillId === 'missing-beta')
    expect(missingItem).toBeDefined()
    expect(missingItem!.status).toBe('missing')

    // CRITICAL: install IPC must NOT have been called!
    expect(installIpcInvoked).toBe(false)
  })

  it('sanitizes malformed schema data, ensuring unknown scores/agents remain unknown and steps is an array (R05)', () => {
    const raw = {
      version: '2.0.0',
      updatedAt: 1700000000000,
      skills: [
        {
          id: 'skill-malformed',
          name: 'skill-malformed',
          titleZh: '格式畸形技能',
          description: 'Testing malformed fields',
          gitUrl: 'https://github.com/acme/malformed',
          // Malformed usageGuide
          usageGuide: {
            steps: 42, // malformed: number instead of string[]
            summary: 'A summary',
          },
          // Missing halowakeScore & compatibleAgents
        },
      ],
      pipelines: [],
    }

    const result = validateAndSanitizeRemoteRegistry(raw)
    expect(result.skills).toHaveLength(1)
    const skill = result.skills[0]
    expect(skill.usageGuide).toBeDefined()
    expect(Array.isArray(skill.usageGuide!.steps)).toBe(true)
    expect(skill.usageGuide!.steps).toEqual([])
    expect(skill.halowakeScore).toBeUndefined()
    expect(skill.compatibleAgents).toEqual([])
    expect(skill.badge).toBe('community')
  })

  it('handles S03: explicitly validates and preserves sourceId and pipeline.playbook', () => {
    const raw = {
      version: '2.0.0',
      updatedAt: 1700000000000,
      skills: [
        {
          id: 'community-skill',
          name: 'community-skill',
          titleZh: '社区技能',
          description: 'From agent-skills-corpus',
          gitUrl: 'https://github.com/acme/community',
          sourceId: 'agent-skills-corpus',
        },
        {
          id: 'official-skill',
          name: 'official-skill',
          titleZh: '官方技能',
          description: 'Official core',
          gitUrl: 'https://github.com/acme/official',
        },
      ],
      pipelines: [
        {
          id: 'test-pipe',
          title: '测试流水线',
          skills: ['community-skill'],
          playbook: [
            {
              stage: 'coding',
              skillId: 'community-skill',
              skillName: '社区技能',
              role: '核心实现',
              howToAct: '根据设计编写代码',
              promptExample: '请帮我实现这个功能',
            },
            {
              // Invalid step: invalid stage
              stage: 'non-existent-stage',
              skillId: 'community-skill',
              skillName: '社区技能',
            },
            {
              // Invalid step: missing skillId
              stage: 'testing',
              skillName: '缺失ID技能',
            },
            null,
            42,
          ],
        },
      ],
    }

    const result = validateAndSanitizeRemoteRegistry(raw)
    expect(result.skills).toHaveLength(2)

    const communitySkill = result.skills.find(s => s.id === 'community-skill')
    expect(communitySkill).toBeDefined()
    // S03: sourceId must be preserved so community badges and warnings do not vanish
    expect(communitySkill!.sourceId).toBe('agent-skills-corpus')

    const officialSkill = result.skills.find(s => s.id === 'official-skill')
    expect(officialSkill).toBeDefined()
    expect(officialSkill!.sourceId).toBeUndefined()

    expect(result.pipelines).toHaveLength(1)
    const pipe = result.pipelines[0]
    expect(pipe.playbook).toBeDefined()
    // S03: only the 1 valid step must be kept, 4 invalid steps filtered out
    expect(pipe.playbook).toHaveLength(1)
    expect(pipe.playbook![0]).toEqual({
      stage: 'coding',
      skillId: 'community-skill',
      skillName: '社区技能',
      role: '核心实现',
      howToAct: '根据设计编写代码',
      promptExample: '请帮我实现这个功能',
    })
  })
})
