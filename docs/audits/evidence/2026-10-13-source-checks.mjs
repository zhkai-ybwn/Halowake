// Source-level review evidence. Run from the repository root:
// node docs/audits/evidence/2026-10-13-source-checks.mjs
// No browser, network, IPC, real installations, or persistent writes.
// This harness targets the reviewed source layout; convert its cases to normal
// unit tests during refactoring rather than treating the harness as a CI suite.
import fs from 'node:fs'
import path from 'node:path'
import vm from 'node:vm'
import ts from 'typescript'
import * as vue from 'vue'
import * as pinia from 'pinia'

function loadModule(file, requireMock, globals = {}) {
  const exports = {}
  const code = ts.transpileModule(fs.readFileSync(file, 'utf8'), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
  }).outputText
  vm.runInNewContext(code, { exports, require: requireMock, console, setTimeout, clearTimeout, ...globals })
  return exports
}

async function checkSkills() {
  pinia.setActivePinia(pinia.createPinia())
  const catalog = {
    FALLBACK_SEED_SKILLS: [], FALLBACK_SEED_PIPELINES: [],
    getLocalStars: () => new Set(), getUserCustomCategories: () => ({}),
    getLastSyncTime: () => Date.now(),
    getLocalInstallPreference: () => ({ targetMode: 'global', targetAgents: ['codex'] }),
    loadHarvestedCatalogSkills: async () => [], loadHarvestedPipelines: async () => [],
  }
  const exports = loadModule('src/stores/skill-explore.ts', id => {
    if (id === 'pinia') return pinia
    if (id === 'vue') return vue
    if (id.includes('skill-catalog-service')) return catalog
    if (id.includes('skills-service')) return {
      installAgentSkill: async () => ({ success: true, message: 'already exists', installedPaths: [] }),
    }
    if (id.includes('skill-category-label')) return { skillCategoryLabel: x => x }
    if (id.includes('skill-synonyms')) return { expandSearchTerms: q => [[q]] }
    throw new Error(`Unmocked import: ${id}`)
  })
  const store = exports.useSkillExploreStore()
  store.catalogSkills = [{ id: 'repo-a', name: 'same', gitUrl: 'https://github.com/a/skills', subPath: 'skills/one' }]
  console.log('F08 identity, mismatched source:', store.isSkillInstalled('repo-a', [{
    id: 'codex:same', name: 'same', path: 'C:/skills/same', catalogId: 'repo-a',
    gitRemoteUrl: 'https://github.com/b/skills',
  }]))
  store.pendingPipeline = { id: 'pipeline', skills: ['repo-a', 'missing-skill'] }
  console.log('F04 missing pipeline member + skipped install:', JSON.stringify(await store.executeInstall()))

  const installed = fs.readFileSync('src/components/skills/installed/InstalledSkillsView.vue', 'utf8')
  const body = installed.match(/const displaySkills = computed\(\(\) => \{([\s\S]*?)\n\}\)/)[1]
  const filter = new Function('store', 'selectedFilter', 'categoryDimension', 'exploreStore',
    'getCustomTags', 'getSkillSdlcStage', 'localSearch', 'sortMode', body)
  console.log('F07 matching custom-tag count:', filter(
    { skills: [{ id: 'codex:test', name: 'test', agentType: 'codex', associatedAgents: ['codex'] }] },
    { value: 'my-tag' }, { value: 'agent' }, { allCustomCategories: ['my-tag'] },
    () => ['my-tag'], () => 'coding', { value: '' }, { value: 'name' },
  ).length)

  const explore = fs.readFileSync('src/components/skills/explore/SkillExploreView.vue', 'utf8')
  const sortBody = explore.match(/return \[\.\.\.skills\]\.sort\(\(a, b\) => \{([\s\S]*?)\n  \}\)/)[1]
  const comparator = new Function('a', 'b', 'sortBy', 'query', 'rawTerms', 'expandedGroups',
    'hasBackendRanking', 'exploreStore', 'computeRelevanceScore', sortBody)
  const a = { name: 'a', stars: 0, halowakeScore: 60, metrics: { githubStars: 100 } }
  const b = { name: 'b', stars: 0, halowakeScore: 76, metrics: { githubStars: 10 } }
  const c = { name: 'c', stars: 0, halowakeScore: 92, metrics: { githubStars: 1 } }
  const compare = (x, y) => comparator(x, y, { value: 'popular' }, '', [], [], false, {}, () => 0)
  console.log('F09 A/B, B/C, C/A comparator:', compare(a, b), compare(b, c), compare(c, a))
}

function checkThemeAndDraft() {
  pinia.setActivePinia(pinia.createPinia())
  let onChange
  const attrs = {}
  const media = { matches: false, addEventListener: (_name, cb) => { onChange = cb } }
  const exports = loadModule('src/stores/preferences.ts', id => {
    if (id === 'pinia') return pinia
    if (id === '@/i18n') return { i18n: { global: { locale: { value: 'zh-CN' } } } }
    throw new Error(`Unmocked import: ${id}`)
  }, {
    localStorage: { getItem: () => null, setItem: () => {} },
    window: { matchMedia: () => media },
    document: { documentElement: { setAttribute: (key, value) => { attrs[key] = value } } },
  })
  const preferences = exports.usePreferencesStore()
  preferences.initPreferences()
  console.log('U03 system theme initial:', preferences.resolvedTheme, attrs['data-theme'])
  media.matches = true
  onChange({ matches: true })
  console.log('U03 system changed to dark:', preferences.resolvedTheme, attrs['data-theme'])

  // Equivalent dependency/array semantics from SkillInstallModal.vue.
  let storedCwd = 'A'
  const currentCwd = vue.computed(() => storedCwd)
  const first = currentCwd.value
  storedCwd = 'B'
  console.log('F11 nonreactive computed, A -> B:', first, currentCwd.value)
  const preference = { targetAgents: ['codex'] }
  const draft = { ...preference }
  draft.targetAgents.push('claude')
  console.log('F11 shallow draft changes original:', JSON.stringify(preference))
}

function checkTokensAndContrast() {
  const files = []
  function walk(dir) {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      const file = path.join(dir, entry.name)
      if (entry.isDirectory()) walk(file)
      else if (/\.(scss|vue|ts)$/.test(file)) files.push(file)
    }
  }
  walk('src')
  const definitions = new Set()
  const references = new Map()
  for (const file of files) {
    const source = fs.readFileSync(file, 'utf8')
    for (const match of source.matchAll(/(--lumina-[\w-]+)\s*:/g)) definitions.add(match[1])
    for (const match of source.matchAll(/var\(\s*(--lumina-[\w-]+)\s*([,)])/g)) {
      const ref = references.get(match[1]) || { count: 0, noFallback: 0 }
      ref.count++
      ref.noFallback += match[2] === ')' ? 1 : 0
      references.set(match[1], ref)
    }
  }
  console.log('U02 undefined source tokens:', JSON.stringify([...references]
    .filter(([key]) => !definitions.has(key)).map(([token, ref]) => ({ token, ...ref }))))
  function luminance(hex) {
    return hex.match(/\w\w/g).map(v => parseInt(v, 16) / 255)
      .map(v => v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4)
      .reduce((sum, v, index) => sum + v * [0.2126, 0.7152, 0.0722][index], 0)
  }
  for (const [fg, bg] of [['8e8e93', 'ffffff'], ['8e8e93', 'f5f5f7'],
    ['77777d', '222224'], ['ffffff', '4fa397'], ['10201e', '64b5aa']]) {
    const a = luminance(fg), b = luminance(bg)
    console.log(`U04 contrast #${fg}/#${bg}:`, ((Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05)).toFixed(2))
  }
}

async function main() {
  await checkSkills()
  checkThemeAndDraft()
  checkTokensAndContrast()
}
main().catch(error => { console.error(error); process.exitCode = 1 })
