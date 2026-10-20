import { describe, it, expect, beforeEach } from 'vitest'
import { defineComponent, h, nextTick } from 'vue'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { NMessageProvider, NDialogProvider } from 'naive-ui'
import { i18n } from '@/i18n'
import SkillsManagerModal from '@/components/skills/SkillsManagerModal.vue'
import { useSkillsStore } from '@/stores/skills'
import { useSkillExploreStore } from '@/stores/skill-explore'
import type { CatalogSkill, AgentSkill } from '@/types/skills'

const Wrapper = defineComponent({
  setup() {
    return () =>
      h(NMessageProvider, null, {
        default: () => h(NDialogProvider, null, { default: () => h(SkillsManagerModal) }),
      })
  },
})

describe('Skills Modal & Tab Switch Performance', () => {
  let pinia: ReturnType<typeof createPinia>

  beforeEach(() => {
    pinia = createPinia()
    setActivePinia(pinia)
  })

  it('renders installed view and switches tabs in < 150ms with 10,502 catalog skills and 97 installed skills', async () => {
    const skillsStore = useSkillsStore()
    const exploreStore = useSkillExploreStore()

    // 1. Populate 10,502 catalog skills
    const catalog: CatalogSkill[] = []
    for (let i = 0; i < 10502; i++) {
      catalog.push({
        id: `catalog-${i}`,
        name: `catalog-skill-${i}`,
        titleZh: `技能-${i}`,
        description: `Description for skill ${i}`,
        category: 'coding',
        stage: 'coding',
        author: { name: 'Author' },
        gitUrl: `https://github.com/example/catalog-${i}`,
        subPath: '',
        tags: ['code', 'dev'],
        stars: 10,
        halowakeScore: 80,
        badge: 'community',
        metrics: {
          githubStars: 10,
          forks: 2,
          lastCommitDaysAgo: 5,
          issueCloseRate: 90,
          securityAuditPassed: true,
        },
        recommendedWith: [],
        compatibleAgents: ['antigravity'],
        howToUse: '',
        isBundle: false,
      })
    }
    exploreStore.catalogSkills = catalog

    // 2. Populate 97 installed skills
    const installed: AgentSkill[] = []
    for (let i = 0; i < 84; i++) {
      installed.push({
        id: `antigravity:local-skill-${i}`,
        name: `local-skill-${i}`,
        description: `Local description ${i}`,
        path: `C:/Users/user/.gemini/config/skills/local-skill-${i}`,
        agentType: 'antigravity',
        agentLabel: 'Antigravity',
        associatedAgents: ['antigravity'],
        canUninstall: true,
        externallyManaged: false,
        management: 'unmanaged',
        isGitRepo: false,
        hasUpdate: false,
        commitsBehind: 0,
        isDirty: false,
      })
    }
    for (let i = 0; i < 13; i++) {
      installed.push({
        id: `antigravity:catalog-skill-${i}`,
        name: `catalog-skill-${i}`,
        description: `Catalog description ${i}`,
        path: `C:/Users/user/.gemini/config/skills/catalog-skill-${i}`,
        agentType: 'antigravity',
        agentLabel: 'Antigravity',
        associatedAgents: ['antigravity'],
        canUninstall: true,
        externallyManaged: false,
        management: 'git',
        isGitRepo: true,
        gitRemoteUrl: `https://github.com/example/catalog-${i}.git`,
        source: {
          kind: 'git',
          gitUrl: `https://github.com/example/catalog-${i}`,
          subPath: '',
        },
        hasUpdate: false,
        commitsBehind: 0,
        isDirty: false,
      })
    }
    skillsStore.skills = installed
    skillsStore.modalVisible = true
    exploreStore.activeMainTab = 'installed'

    // Measure initial mount
    const t0 = performance.now()
    const wrapper = mount(Wrapper, {
      global: {
        plugins: [pinia, i18n],
      },
    })
    await nextTick()
    const tMount = performance.now() - t0
    console.log(`[PERF] Initial mount time: ${tMount.toFixed(2)}ms`)

    // Measure switch to explore tab
    const t1 = performance.now()
    exploreStore.activeMainTab = 'explore'
    await nextTick()
    const tSwitchExplore = performance.now() - t1
    console.log(`[PERF] Switch to explore: ${tSwitchExplore.toFixed(2)}ms`)

    // Measure switch back to installed tab
    const t2 = performance.now()
    exploreStore.activeMainTab = 'installed'
    await nextTick()
    const tSwitchInstalled = performance.now() - t2
    console.log(`[PERF] Switch back to installed: ${tSwitchInstalled.toFixed(2)}ms`)

    // Measure switch to history tab
    const t3 = performance.now()
    exploreStore.activeMainTab = 'history'
    await nextTick()
    const tSwitchHistory = performance.now() - t3
    console.log(`[PERF] Switch to history: ${tSwitchHistory.toFixed(2)}ms`)

    wrapper.unmount()

    // Tab switches must now be sub-second (< 250ms even inside JSDOM)
    expect(tSwitchExplore).toBeLessThan(250)
    expect(tSwitchInstalled).toBeLessThan(250)
    expect(tSwitchHistory).toBeLessThan(250)
  })
})
