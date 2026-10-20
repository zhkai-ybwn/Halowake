<template>
  <div class="installed-skills-view">
    <!-- 顶部双层工具栏：第1层搜索与维度/排序，第2层优雅分类胶囊 -->
    <div class="installed-toolbar-panel">
      <div class="toolbar-primary-row">
        <div class="macos-search-box" @click="focusSearch">
          <Icon icon="solar:magnifer-linear" class="search-glyph" />
          <input
            ref="searchInputRef"
            :value="localSearch"
            type="text"
            class="macos-search-field"
            :placeholder="t('skills.searchPlaceholder')"
            autocomplete="off"
            spellcheck="false"
            @input="handleSearchInput"
            @click.stop
            @mousedown.stop
            @keydown.stop="handleInputKeydown"
          />
          <kbd v-if="!localSearch" class="search-kbd">{{ isMac ? '⌘F' : 'Ctrl+F' }}</kbd>
          <button
            v-else
            type="button"
            class="search-clear-glyph"
            :title="t('common.clear', '清空')"
            @click.stop="clearSearch"
          >
            ×
          </button>
        </div>

        <div class="toolbar-controls-right">
          <!-- 分类维度切换：按 Agent / 按 SDLC 阶段 -->
          <div class="category-dimension-bar" role="radiogroup" :aria-label="t('skills.review.category')">
            <button
              type="button"
              class="dimension-toggle-btn"
              :class="{ active: categoryDimension === 'agent' }"
              @click="switchDimension('agent')"
            >
              <Icon icon="solar:users-group-rounded-linear" class="dim-icon" />
              <span>{{ t('skills.review.byAgent') }}</span>
            </button>
            <button
              type="button"
              class="dimension-toggle-btn"
              :class="{ active: categoryDimension === 'sdlc' }"
              @click="switchDimension('sdlc')"
            >
              <Icon icon="solar:layers-minimalistic-linear" class="dim-icon" />
              <span>{{ t('skills.review.byStage') }}</span>
            </button>
          </div>

          <!-- 排序下拉 -->
          <label class="installed-sort-control">
            <Icon icon="solar:sort-vertical-linear" class="sort-icon" />
            <select v-model="sortMode" :aria-label="t('skills.review.installedSortAria')">
              <option value="recent">{{ t('skills.review.recent') }}</option>
              <option value="name">{{ t('skills.review.nameAscending') }}</option>
            </select>
          </label>
        </div>
      </div>

      <!-- 次级过滤胶囊栏：全部、待更新、以及当前维度下的所有分类 -->
      <div class="toolbar-filter-strip" role="tablist">
        <button
          type="button"
          role="tab"
          class="segmented-pill"
          :class="{ active: filterState.kind === 'all' }"
          @click="setFilter('all')"
        >
          <span>{{ t('skills.categoryAll') }}</span>
          <span class="tab-badge">{{ store.skills.length }}</span>
        </button>

        <button
          v-if="store.updateCount > 0"
          type="button"
          role="tab"
          class="segmented-pill segmented-pill--warning"
          :class="{ active: filterState.kind === 'updates' }"
          @click="setFilter('updates')"
        >
          <span class="dot-indicator"></span>
          <span>{{ t('skills.categoryUpdates') }}</span>
          <span class="tab-badge tab-badge--warn">{{ store.updateCount }}</span>
        </button>

        <!-- A. Agent 维度标签 -->
        <template v-if="categoryDimension === 'agent'">
          <button
            v-for="cat in store.availableCategories"
            :key="cat"
            type="button"
            role="tab"
            class="segmented-pill"
            :class="{ active: filterState.kind === 'agent' && filterState.value === cat }"
            @click="setFilter('agent', cat)"
          >
            <span>{{ getCategoryLabel(cat) }}</span>
            <span class="tab-badge">{{ store.categoryCounts[cat] ?? 0 }}</span>
          </button>
        </template>

        <!-- B. SDLC 阶段维度标签 -->
        <template v-else-if="categoryDimension === 'sdlc'">
          <button
            v-for="stage in availableStages"
            :key="stage.key"
            type="button"
            role="tab"
            class="segmented-pill"
            :class="{ active: filterState.kind === 'stage' && filterState.value === stage.key }"
            @click="setFilter('stage', stage.key)"
          >
            <span>{{ t(stage.label) }}</span>
            <span class="tab-badge">{{ stageCounts[stage.key] || 0 }}</span>
          </button>
        </template>

        <!-- C. 用户自定义分类标签 (若存在) -->
        <button
          v-for="cat in exploreStore.allCustomCategories"
          :key="cat"
          type="button"
          role="tab"
          class="segmented-pill segmented-pill--custom"
          :class="{ active: filterState.kind === 'tag' && filterState.value === cat }"
          @click="setFilter('tag', cat)"
        >
          <span>#{{ cat }}</span>
        </button>
      </div>
    </div>

    <!-- Content Area: Structured Inset Table with Strict Vertical Column Alignment -->
    <main class="macos-content-viewport">
      <div v-if="store.loading" class="macos-empty-state">
        <Icon icon="solar:refresh-linear" class="spin-glyph" />
        <p>{{ t('common.loading') }}</p>
      </div>

      <div v-else-if="displaySkills.length === 0" class="macos-empty-state">
        <Icon icon="solar:box-minimalistic-linear" class="empty-glyph" />
        <p v-if="store.skills.length === 0">{{ t('skills.noSkillsFound') }}</p>
        <p v-else>{{ t('skills.noMatchingSkills') }}</p>
      </div>

      <div v-else class="installed-list-shell">
        <div class="installed-list-heading">
          <div class="installed-list-heading__title">
            <strong>{{ t('skills.review.localSkills') }}</strong>
            <span class="count-pill">{{ t('skills.review.visibleCount', { visible: displaySkills.length, total: store.skills.length }) }}</span>
          </div>
          <div class="installed-list-hint">
            <Icon icon="solar:info-circle-linear" />
            <span>{{ t('skills.review.rowClickHint') }}</span>
          </div>
        </div>

        <div class="installed-list-scroll">
          <article
            v-for="skill in displaySkills"
            :key="skill.path"
            class="installed-skill-row"
            :class="{ 'installed-skill-row--attention': skill.hasUpdate }"
            role="button"
            tabindex="0"
            :aria-label="`${skill.name} - ${t('skills.review.rowClickHint')}`"
            @click="openSkillDetails(skill)"
            @keydown.enter="openSkillDetails(skill)"
          >
            <!-- 基于职能与领域的个性化图标 -->
            <span
              class="installed-skill-icon"
              :style="{
                color: getCachedSkillVisuals(skill).color,
                backgroundColor: getCachedSkillVisuals(skill).bg,
                borderColor: getCachedSkillVisuals(skill).border,
              }"
              aria-hidden="true"
            >
              <Icon :icon="getCachedSkillVisuals(skill).icon" />
            </span>

            <div class="installed-skill-main">
              <div class="installed-skill-title">
                <strong :title="skill.name">{{ skill.name }}</strong>
                <span v-if="skill.version" class="skill-ver-pill">v{{ skill.version }}</span>
                <span v-if="isRecentlyAdded(skill)" class="installed-new-badge">{{ t('skills.review.recent') }}</span>
                <span
                  v-if="skill.hasUpdate && skill.management !== 'unmanaged' && skill.management !== 'external'"
                  class="installed-status installed-status--update"
                  :title="skill.updateSummary || undefined"
                >
                  <Icon icon="solar:cloud-download-linear" />
                  {{ t('skills.review.updateReady') }}<template v-if="skill.latestVersion"> (v{{ skill.latestVersion }})</template>
                </span>
                <span
                  v-else-if="skill.updateStatus === 'error' || skill.updateStatus === 'unknown'"
                  class="installed-status installed-status--dirty"
                  :title="skill.updateError || skill.updateSummary || undefined"
                >
                  <Icon icon="solar:danger-triangle-linear" />
                  {{ skill.updateStatus === 'error' ? '检查失败' : '状态未知' }}
                </span>
                <span
                  v-if="skill.isDirty"
                  class="installed-status installed-status--dirty"
                >
                  <Icon icon="solar:pen-linear" />
                  {{ t('skills.review.locallyModified') }}
                </span>
                <span
                  v-if="skill.management === 'unmanaged'"
                  class="skill-ver-pill"
                  title="本地自定义或手动放入的技能，不支持自动覆盖更新"
                >
                  手动管理
                </span>
              </div>
              <p class="installed-skill-description" :title="skill.cleanDesc || cleanDescription(skill.description) || undefined">
                {{ skill.cleanDesc || cleanDescription(skill.description) || t('skills.review.noDescription') }}
              </p>
              <div class="installed-skill-meta">
                <span class="installed-agent" :title="skill.displayAgentTag || getPrimaryAgentTag(skill)">
                  {{ skill.displayAgentTag || getPrimaryAgentTag(skill) }}
                </span>
                <SkillStageBadge :stage="getSkillSdlcStage(skill)" />
                <span v-for="tag in getCustomTags(skill)" :key="tag" class="custom-tag-micro">#{{ tag }}</span>
                <input
                  v-if="editingTagSkillId === skill.path"
                  ref="tagInputRef"
                  v-model="tagDraft"
                  class="tag-inline-input"
                  maxlength="40"
                  :placeholder="t('skills.review.tagPlaceholder')"
                  :aria-label="t('skills.review.addTag', { name: skill.name })"
                  @click.stop
                  @keydown.enter.prevent="saveTag(skill)"
                  @keydown.esc.prevent="cancelTag"
                  @blur="saveTag(skill)"
                />
                <button
                  v-else
                  type="button"
                  class="add-tag-micro-btn"
                  :title="t('skills.review.addTag', { name: skill.name })"
                  :aria-label="t('skills.review.addTag', { name: skill.name })"
                  @click.stop="startTagEdit(skill)"
                >
                  <Icon icon="solar:tag-linear" />
                  <span>标签</span>
                </button>
                <span v-if="skill.installedAt" class="installed-added-date">
                  {{ t('skills.review.addedAt', { date: formatAddedDate(skill.installedAt) }) }}
                </span>
              </div>
            </div>

            <div class="installed-skill-actions" @click.stop>
              <button
                v-if="skill.hasUpdate && skill.management !== 'unmanaged' && skill.management !== 'external' && !skill.externallyManaged"
                type="button"
                class="installed-update-btn"
                :disabled="store.updatingPaths.includes(skill.path) || store.uninstallingPaths.includes(skill.path)"
                @click.stop="handleUpdateSingle(skill)"
              >
                <Icon :icon="store.updatingPaths.includes(skill.path) ? 'solar:refresh-linear' : 'solar:cloud-download-linear'" :class="{ 'spin-glyph': store.updatingPaths.includes(skill.path) }" />
                <span>{{ store.updatingPaths.includes(skill.path) ? t('skills.updating') : t('skills.updateSingle') }}</span>
              </button>
              <button
                type="button"
                class="installed-icon-btn"
                :title="`${t('skills.openFolder')}：${skill.path}`"
                :aria-label="`${t('skills.openFolder')} ${skill.name}`"
                @click.stop="store.openFolder(skill.path)"
              >
                <Icon icon="solar:folder-with-files-linear" />
              </button>
              <button
                v-if="skill.canUninstall"
                type="button"
                class="installed-icon-btn installed-icon-btn--danger"
                :disabled="store.uninstallingPaths.includes(skill.path) || store.updatingPaths.includes(skill.path)"
                :title="t('skills.uninstall')"
                :aria-label="t('skills.uninstallSkill', { name: skill.name })"
                @click.stop="confirmUninstall(skill)"
              >
                <Icon :icon="store.uninstallingPaths.includes(skill.path) ? 'solar:refresh-linear' : 'solar:trash-bin-minimalistic-linear'" :class="{ 'spin-glyph': store.uninstallingPaths.includes(skill.path) }" />
              </button>
              <span v-else-if="skill.externallyManaged" class="installed-cli-label" :title="t('skills.externallyManaged')">CLI</span>
            </div>

            <!-- 点击查看详情外显箭头 -->
            <div class="installed-detail-chevron" aria-hidden="true" title="查看详情">
              <Icon icon="solar:alt-arrow-right-linear" />
            </div>
          </article>
        </div>
      </div>
    </main>

    <!-- macOS Inset Status Footer Bar -->
    <footer class="macos-window-footer">
      <div class="footer-leading">
        <span class="status-summary-text">
          <Icon icon="solar:check-circle-linear" class="footer-check-icon" />
          <span>{{ t('skills.review.readyCount', { count: store.skills.length }) }}</span>
          <span class="footer-dot-sep">·</span>
          <span>{{ t('skills.review.coverageAgents', { count: store.availableCategories.length }) }}</span>
          <template v-if="store.updateCount > 0">
            <span class="footer-dot-sep">·</span>
            <span class="highlight-warn">{{ t('skills.pillUpdatesAvailable', { count: store.updateCount }) }}</span>
          </template>
        </span>
      </div>

      <div class="footer-trailing">
        <span v-if="formattedLastChecked" class="footer-timestamp">
          {{ t('skills.lastChecked') }} {{ formattedLastChecked }}
        </span>
      </div>
    </footer>
  </div>
</template>

<script setup lang="ts">
import { computed, h, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { useDialog, useMessage } from 'naive-ui'
import { Icon } from '@iconify/vue'
import { useI18n } from 'vue-i18n'
import { useSkillsStore } from '@/stores/skills'
import { useSkillExploreStore } from '@/stores/skill-explore'
import { useWorkspaceStore } from '@/stores/workspace'
import { inferSkillStage } from '@/services/skills/skill-catalog-service'
import { findMatchingCatalogSkill } from '@/services/skills/skill-identity'
import type { AgentSkill, CatalogSkill, SdlcStage } from '@/types/skills'
import { getSkillVisuals, type SkillVisual } from '@/utils/skill-visuals'
import SkillStageBadge from '../common/SkillStageBadge.vue'

const { locale, t } = useI18n()
const message = useMessage()
const dialog = useDialog()
const store = useSkillsStore()
const exploreStore = useSkillExploreStore()
const workspaceStore = useWorkspaceStore()

const searchInputRef = ref<HTMLInputElement | null>(null)
const localSearch = ref('')
let searchTimer: ReturnType<typeof setTimeout> | null = null

export type InstalledFilterKind = 'all' | 'updates' | 'agent' | 'stage' | 'tag'

export interface InstalledFilterState {
  kind: InstalledFilterKind
  value?: string
}

const availableStages: Array<{ key: SdlcStage; label: string }> = [
  { key: 'requirement', label: 'skills.stageRequirement' },
  { key: 'design', label: 'skills.stageDesign' },
  { key: 'coding', label: 'skills.stageCoding' },
  { key: 'testing', label: 'skills.stageTesting' },
  { key: 'delivery', label: 'skills.stageDelivery' },
]

const categoryDimension = ref<'agent' | 'sdlc'>('agent')
const filterState = ref<InstalledFilterState>({ kind: 'all' })

function setFilter(kind: InstalledFilterKind, value?: string) {
  filterState.value = { kind, value }
}

function switchDimension(dim: 'agent' | 'sdlc') {
  categoryDimension.value = dim
  // 切换维度时清除不适用的旧条件，避免隐藏的残留筛选
  if (filterState.value.kind === 'agent' || filterState.value.kind === 'stage') {
    filterState.value = { kind: 'all' }
  }
}

const selectedFilter = computed({
  get: () => {
    if (filterState.value.kind === 'all') return 'all'
    if (filterState.value.kind === 'updates') return 'updates'
    return filterState.value.value || 'all'
  },
  set: (val: string) => {
    if (val === 'all') {
      filterState.value = { kind: 'all' }
    } else if (val === 'updates') {
      filterState.value = { kind: 'updates' }
    } else if (exploreStore.allCustomCategories.includes(val)) {
      filterState.value = { kind: 'tag', value: val }
    } else if (categoryDimension.value === 'sdlc' || availableStages.some(s => s.key === val)) {
      filterState.value = { kind: 'stage', value: val }
    } else {
      filterState.value = { kind: 'agent', value: val }
    }
  },
})
const sortMode = ref<'recent' | 'name'>('recent')
const editingTagSkillId = ref<string | null>(null)
const tagDraft = ref('')
const tagInputRef = ref<HTMLInputElement | null>(null)

const isMac = computed(() => {
  if (typeof navigator === 'undefined') return false
  return /Mac|iPod|iPhone|iPad/.test(navigator.platform)
})

function confirmUninstall(skill: AgentSkill) {
  dialog.warning({
    title: t('skills.uninstallSkill', { name: skill.name }),
    content: () => h('div', { style: { display: 'grid', gap: '8px' } }, [
      h('p', { style: { margin: 0 } }, t('skills.uninstallWarning')),
      h('code', { style: { overflowWrap: 'anywhere' } }, skill.path),
      h('p', { style: { margin: 0 } }, t('skills.uninstallAgents', { agents: skill.displayAgentTag || skill.agentLabel })),
      ...(skill.isDirty ? [h('p', { style: { color: 'var(--lumina-danger)', margin: 0 } }, t('skills.uninstallDirty'))] : []),
    ]),
    positiveText: t('skills.uninstall'),
    negativeText: t('common.cancel'),
    positiveButtonProps: { type: 'error' },
    onPositiveClick: async () => {
      try {
        const projectCwd = workspaceStore.refreshFromStorage() || undefined
        await store.uninstallSingle(skill, projectCwd)
        message.success(t('skills.uninstallSuccess', { name: skill.name }))
      } catch (error) {
        message.error(t('skills.uninstallFailed', { error: error instanceof Error ? error.message : String(error) }))
        return false
      }
    },
  })
}



const sdlcStageCache = new WeakMap<AgentSkill, SdlcStage>()
function getSkillSdlcStage(skill: AgentSkill): SdlcStage {
  const cached = sdlcStageCache.get(skill)
  if (cached) return cached
  const match = findMatchingCatalogSkill(skill, exploreStore.catalogSkills)
  const stage = match ? match.stage : inferSkillStage(skill.name, skill.description)
  sdlcStageCache.set(skill, stage)
  return stage
}

const skillVisualCache = new WeakMap<AgentSkill, SkillVisual>()
function getCachedSkillVisuals(skill: AgentSkill): SkillVisual {
  const cached = skillVisualCache.get(skill)
  if (cached) return cached
  const res = getSkillVisuals(skill)
  skillVisualCache.set(skill, res)
  return res
}

function getCustomTags(skill: AgentSkill | string): string[] {
  return exploreStore.getSkillCustomCategories(skill)
}

function openSkillDetails(skill: AgentSkill) {
  const match = findMatchingCatalogSkill(skill, exploreStore.catalogSkills)
  if (match) {
    exploreStore.openDetail(match)
  } else {
    const fallback: CatalogSkill = {
      id: skill.catalogId || skill.id || skill.name,
      name: skill.name,
      titleZh: skill.name,
      description: skill.cleanDesc || cleanDescription(skill.description) || t('skills.review.noDescription'),
      category: skill.customCategory || 'general',
      stage: getSkillSdlcStage(skill),
      author: { name: skill.displayAgentTag || getPrimaryAgentTag(skill) || 'Local' },
      gitUrl: skill.gitRemoteUrl || skill.path,
      subPath: skill.subPath,
      tags: getCustomTags(skill),
      stars: 0,
      halowakeScore: 0,
      badge: 'community',
      metrics: {
        githubStars: 0,
        forks: 0,
        lastCommitDaysAgo: 0,
        issueCloseRate: 0,
        securityAuditPassed: false,
      },
      recommendedWith: [],
      compatibleAgents: skill.associatedAgents || [skill.agentType],
      howToUse: skill.path,
      isBundle: false,
    }
    exploreStore.openDetail(fallback)
  }
}

function startTagEdit(skill: AgentSkill) {
  editingTagSkillId.value = skill.path
  tagDraft.value = ''
  void nextTick(() => tagInputRef.value?.focus())
}

function cancelTag() {
  editingTagSkillId.value = null
  tagDraft.value = ''
}

function saveTag(skill: AgentSkill) {
  if (editingTagSkillId.value !== skill.path) return
  const tag = tagDraft.value.trim()
  cancelTag()
  if (!tag) return
  exploreStore.addSkillCustomCategory(skill.catalogId || skill.id, tag)
  message.success(t('skills.review.tagAdded', { name: skill.name, tag }))
}

const stageCounts = computed<Record<string, number>>(() => {
  const counts: Record<string, number> = {}
  store.skills.forEach(s => {
    const st = getSkillSdlcStage(s)
    counts[st] = (counts[st] || 0) + 1
  })
  return counts
})

const displaySkills = computed(() => {
  let list = store.skills

  // 1. 过滤分类
  const filter = filterState.value
  if (filter.kind === 'updates') {
    list = list.filter(s => s.hasUpdate)
  } else if (filter.kind === 'agent' && filter.value) {
    const agentVal = filter.value
    list = list.filter(
      s =>
        s.agentType === agentVal ||
        (s.associatedAgents && s.associatedAgents.includes(agentVal))
    )
  } else if (filter.kind === 'stage' && filter.value) {
    const stageVal = filter.value
    list = list.filter(s => getSkillSdlcStage(s) === stageVal)
  } else if (filter.kind === 'tag' && filter.value) {
    const tagVal = filter.value
    list = list.filter(s => getCustomTags(s).includes(tagVal))
  }

  // 2. 搜索查询
  const q = localSearch.value.trim().toLowerCase()
  if (q) {
    list = list.filter(s => {
      const tags = getCustomTags(s).join(' ').toLowerCase()
      return (
        s.name.toLowerCase().includes(q) ||
        s.description.toLowerCase().includes(q) ||
        s.agentLabel.toLowerCase().includes(q) ||
        tags.includes(q)
      )
    })
  }

  return [...list].sort((a, b) => sortMode.value === 'recent'
    ? (b.installedAt || 0) - (a.installedAt || 0) || a.name.localeCompare(b.name)
    : a.name.localeCompare(b.name))
})

function isRecentlyAdded(skill: AgentSkill): boolean {
  return !!skill.installedAt && Date.now() - skill.installedAt < 7 * 24 * 60 * 60 * 1000
}

function formatAddedDate(timestamp: number): string {
  return new Date(timestamp).toLocaleDateString(locale.value, { month: 'numeric', day: 'numeric' })
}

function handleSearchInput(e: Event) {
  const val = (e.target as HTMLInputElement).value
  localSearch.value = val
  if (searchTimer) clearTimeout(searchTimer)
  searchTimer = setTimeout(() => {
    store.searchQuery = val
  }, 100)
}

function clearSearch() {
  localSearch.value = ''
  store.searchQuery = ''
  searchInputRef.value?.focus()
}

function focusSearch() {
  searchInputRef.value?.focus()
}

function handleInputKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    if (localSearch.value) {
      clearSearch()
      e.stopPropagation()
    }
  }
}

function handleFindShortcut(e: KeyboardEvent) {
  if (!store.modalVisible || exploreStore.activeMainTab !== 'installed') return
  if ((e.metaKey || e.ctrlKey) && (e.key === 'f' || e.key === 'F')) {
    e.preventDefault()
    searchInputRef.value?.focus()
    searchInputRef.value?.select()
  }
}

watch(() => store.installationRevealCount, () => {
  localSearch.value = store.installationRevealQuery
  store.searchQuery = store.installationRevealQuery
  selectedFilter.value = 'all'
  categoryDimension.value = 'agent'
})

onMounted(() => {
  window.addEventListener('keydown', handleFindShortcut, true)
})

onUnmounted(() => {
  if (searchTimer) clearTimeout(searchTimer)
  window.removeEventListener('keydown', handleFindShortcut, true)
})

watch(
  () => store.modalVisible,
  visible => {
    if (visible) {
      void nextTick(() => {
        searchInputRef.value?.focus()
        searchInputRef.value?.select()
      })
    }
  }
)

function getCategoryLabel(agentType: string): string {
  switch (agentType) {
    case 'agents':
      return 'Agents'
    case 'codex':
      return 'Codex'
    case 'antigravity':
      return 'Antigravity'
    case 'cursor':
      return 'Cursor'
    case 'trae':
      return 'Trae'
    case 'claude':
      return 'Claude Code'
    case 'qoder':
      return 'Qoder'
    case 'opencode':
      return 'OpenCode'
    case 'windsurf':
      return 'Windsurf'
    case 'cline':
      return 'Cline'
    case 'project':
      return t('skills.categoryProject')
    default:
      return agentType
  }
}

function getPrimaryAgentTag(skill: AgentSkill): string {
  if (skill.associatedAgents && skill.associatedAgents.length > 1) {
    return skill.associatedAgents.map(a => getCategoryLabel(a)).join(' / ')
  }
  return skill.agentLabel || getCategoryLabel(skill.agentType)
}

function cleanDescription(desc?: string): string {
  if (!desc) return ''
  const trimmed = desc.trim()
  if (['>-', '>', '|-', '|', '>+', '|+'].includes(trimmed)) return ''
  return trimmed
}

const formattedLastChecked = computed(() => {
  if (!store.lastCheckTime) return ''
  const d = new Date(store.lastCheckTime)
  const pad = (n: number) => n.toString().padStart(2, '0')
  return `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
})

async function handleUpdateSingle(skill: AgentSkill) {
  if (skill.management === 'unmanaged' || skill.management === 'external' || skill.externallyManaged) {
    message.warning('未受管或外部管理技能不支持自动更新，请打开目录手动管理')
    return
  }

  if (skill.isDirty) {
    dialog.warning({
      title: `${skill.name} — ${t('skills.review.locallyModified')}`,
      content: `${t('skills.dirtyWarning')} 确认强制更新后，原本地修改将自动备份到同级备份目录或 Git stash。`,
      positiveText: '备份并强制更新',
      negativeText: t('common.cancel'),
      positiveButtonProps: { type: 'warning' },
      onPositiveClick: async () => {
        const res = await store.updateSingle(skill, true)
        if (res.success) {
          message.success(
            res.backupPath
              ? `${skill.name}: ${res.message || t('skills.updateSuccess')}（已备份至 ${res.backupPath}）`
              : `${skill.name}: ${res.message || t('skills.updateSuccess')}`
          )
        } else {
          message.error(`${skill.name}: ${res.message || t('skills.updateFailed')}`)
          return false
        }
      },
    })
    return
  }

  const res = await store.updateSingle(skill)
  if (res.success) {
    message.success(`${skill.name}: ${t('skills.updateSuccess')}`)
  } else {
    message.error(`${skill.name}: ${res.message || t('skills.updateFailed')}`)
  }
}
</script>

<style scoped lang="scss">
.installed-skills-view {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

.installed-toolbar-panel {
  background: color-mix(in srgb, var(--lumina-surface-base) 80%, transparent);
  border-bottom: 0.5px solid var(--lumina-separator);
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  gap: 8px;
  padding: 10px 18px;
}

.toolbar-primary-row {
  align-items: center;
  display: flex;
  gap: 12px;
  justify-content: space-between;
}

.macos-search-box {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-fill-quaternary) 75%, transparent);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-md, 7px);
  cursor: text;
  display: flex;
  flex-shrink: 0;
  gap: 6px;
  height: 28px;
  padding: 0 10px;
  transition: all 120ms ease;
  width: 350px;
  max-width: 48vw;

  &:focus-within {
    background: var(--lumina-surface-elevated);
    border-color: var(--lumina-primary);
    box-shadow: 0 0 0 2px var(--lumina-primary-soft);
  }

  .search-glyph {
    color: var(--lumina-text-tertiary);
    flex-shrink: 0;
    font-size: 13px;
  }

  .macos-search-field {
    background: transparent;
    border: none;
    color: var(--lumina-text);
    flex: 1;
    font-size: 11.5px;
    min-width: 0;
    outline: none;

    &::placeholder {
      color: var(--lumina-text-tertiary);
    }
  }

  .search-kbd {
    background: var(--lumina-control-bg);
    border: 0.5px solid var(--lumina-separator);
    border-radius: 4px;
    color: var(--lumina-text-tertiary);
    font-family: inherit;
    font-size: 10px;
    font-weight: 500;
    padding: 1px 4px;
  }

  .search-clear-glyph {
    background: transparent;
    border: none;
    color: var(--lumina-text-tertiary);
    cursor: pointer;
    font-size: 14px;
    line-height: 1;
    padding: 0;

    &:hover {
      color: var(--lumina-text);
    }
  }
}

.toolbar-controls-right {
  align-items: center;
  display: flex;
  gap: 10px;
}

.category-dimension-bar {
  background: var(--lumina-control-bg);
  border: 0.5px solid var(--lumina-separator);
  border-radius: 6px;
  display: flex;
  flex-shrink: 0;
  gap: 2px;
  padding: 2px;
}

.dimension-toggle-btn {
  align-items: center;
  background: transparent;
  border: none;
  border-radius: 4px;
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: inline-flex;
  font-size: 11px;
  font-weight: 500;
  gap: 4px;
  padding: 2px 8px;
  transition: all 120ms ease;

  .dim-icon {
    font-size: 12px;
  }

  &:hover {
    color: var(--lumina-text);
  }

  &.active {
    background: var(--lumina-surface-elevated);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.08);
    color: var(--lumina-primary);
    font-weight: 600;
  }
}

.installed-sort-control {
  align-items: center;
  background: var(--lumina-surface-elevated);
  border: 0.5px solid var(--lumina-separator);
  border-radius: 6px;
  color: var(--lumina-text-tertiary);
  cursor: pointer;
  display: flex;
  font-size: 11px;
  gap: 4px;
  height: 26px;
  padding: 0 6px 0 8px;
  transition: all 120ms ease;

  &:hover {
    background: var(--lumina-control-hover);
    color: var(--lumina-text);
  }

  .sort-icon {
    font-size: 12px;
  }

  select {
    background: transparent;
    border: none;
    color: var(--lumina-text);
    cursor: pointer;
    font-size: 11px;
    font-weight: 500;
    outline: none;
  }
}

.toolbar-filter-strip {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-fill-quaternary) 60%, transparent);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-md, 7px);
  display: flex;
  gap: 2px;
  overflow-x: auto;
  padding: 2px;

  &::-webkit-scrollbar {
    display: none;
  }

  .segmented-pill {
    align-items: center;
    background: transparent;
    border: none;
    border-radius: 5px;
    color: var(--lumina-text-secondary);
    cursor: pointer;
    display: inline-flex;
    font-size: 11px;
    font-weight: 500;
    gap: 5px;
    height: 24px;
    padding: 0 8px;
    transition: all 120ms ease;
    white-space: nowrap;

    &:hover {
      color: var(--lumina-text);
    }

    &.active {
      background: var(--lumina-surface-elevated);
      box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
      color: var(--lumina-primary);
      font-weight: 600;
    }

    .tab-badge {
      background: color-mix(in srgb, var(--lumina-fill-tertiary) 70%, transparent);
      border-radius: 9999px;
      font-size: 9.5px;
      font-weight: 600;
      padding: 0.5px 5px;

      &--warn {
        background: color-mix(in srgb, var(--lumina-warning, #b76500) 18%, transparent);
        color: var(--lumina-warning, #b76500);
      }
    }

    .dot-indicator {
      background: var(--lumina-warning, #b76500);
      border-radius: 50%;
      height: 6px;
      width: 6px;
    }
  }
}

.macos-content-viewport {
  background: var(--lumina-surface-base);
  display: flex;
  flex: 1;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
}

.macos-empty-state {
  align-items: center;
  color: var(--lumina-text-tertiary);
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 12px;
  justify-content: center;
  padding: 48px;

  .empty-glyph,
  .spin-glyph {
    font-size: 32px;
  }
}

.custom-tag-micro {
  background: var(--lumina-primary-soft);
  border-radius: 3px;
  color: var(--lumina-primary);
  font-size: 11px;
  font-weight: 600;
  padding: 1px 5px;
}

.tag-inline-input {
  background: var(--lumina-surface-secondary);
  border: 1px solid var(--lumina-primary);
  border-radius: 4px;
  color: var(--lumina-text);
  font-size: 11.5px;
  max-width: 150px;
  padding: 2px 5px;
}

.add-tag-micro-btn {
  align-items: center;
  background: transparent;
  border: 0.5px dashed var(--lumina-separator);
  border-radius: 4px;
  color: var(--lumina-text-tertiary);
  cursor: pointer;
  display: inline-flex;
  font-size: 10.5px;
  gap: 3px;
  line-height: 1;
  padding: 2px 5px;
  transition: all 120ms ease;

  svg {
    font-size: 11px;
  }

  &:hover {
    border-color: var(--lumina-primary);
    color: var(--lumina-primary);
  }
}

.skill-ver-pill {
  background: color-mix(in srgb, var(--lumina-fill-tertiary) 70%, transparent);
  border-radius: 3px;
  color: var(--lumina-text-secondary);
  font-size: 11px;
  font-weight: 500;
  margin-left: 4px;
  padding: 1px 5px;
}

.installed-list-shell {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-height: 0;
}

.installed-list-heading {
  align-items: center;
  background: var(--lumina-surface-base);
  border-bottom: 0.5px solid var(--lumina-separator);
  display: flex;
  flex-shrink: 0;
  justify-content: space-between;
  min-height: 38px;
  padding: 6px 20px;

  &__title {
    align-items: center;
    display: flex;
    gap: 8px;

    strong {
      color: var(--lumina-text);
      font-size: 12px;
      font-weight: 600;
    }

    .count-pill {
      background: color-mix(in srgb, var(--lumina-fill-quaternary) 80%, transparent);
      border-radius: 9999px;
      color: var(--lumina-text-secondary);
      font-size: 10.5px;
      font-weight: 500;
      padding: 1px 7px;
    }
  }

  .installed-list-hint {
    align-items: center;
    color: var(--lumina-text-tertiary);
    display: flex;
    font-size: 11px;
    gap: 5px;

    svg {
      font-size: 13px;
    }
  }
}

.installed-list-scroll {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
}

.installed-skill-row {
  align-items: center;
  border-bottom: 0.5px solid var(--lumina-separator);
  cursor: pointer;
  display: flex;
  gap: 14px;
  min-width: 0;
  outline: none;
  padding: 12px 20px;
  position: relative;
  transition: all 140ms ease;

  &:hover,
  &:focus-visible {
    background: color-mix(in srgb, var(--lumina-primary-soft) 25%, var(--lumina-surface-elevated));
  }

  &:focus-visible {
    box-shadow: inset 0 0 0 1.5px var(--lumina-primary);
  }

  &--attention {
    background: color-mix(in srgb, var(--lumina-warning, #b76500) 4%, var(--lumina-surface-base));
  }
}

.installed-skill-icon {
  align-items: center;
  border: 1px solid transparent;
  border-radius: 9px;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.04);
  display: flex;
  flex-shrink: 0;
  font-size: 17px;
  height: 36px;
  justify-content: center;
  transition: transform 140ms ease;
  width: 36px;
}

.installed-skill-row:hover .installed-skill-icon {
  transform: scale(1.05);
}

.installed-skill-main {
  flex: 1;
  min-width: 0;
}

.installed-skill-title {
  align-items: center;
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  min-height: 20px;

  strong {
    color: var(--lumina-text);
    font-size: 13px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
}

.installed-skill-description {
  color: var(--lumina-text-secondary);
  display: -webkit-box;
  font-size: 11.5px;
  line-height: 1.45;
  margin: 3px 0 6px;
  overflow: hidden;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
}

.installed-skill-meta {
  align-items: center;
  display: flex;
  flex-wrap: wrap;
  gap: 5px 8px;
}

.installed-agent {
  background: var(--lumina-fill-quaternary);
  border-radius: 4px;
  color: var(--lumina-text-secondary);
  font-size: 10px;
  font-weight: 500;
  max-width: 240px;
  overflow: hidden;
  padding: 1.5px 6px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.installed-added-date {
  color: var(--lumina-text-tertiary);
  font-size: 10px;
  margin-left: 2px;
}

.installed-new-badge {
  color: var(--lumina-primary);
  font-size: 10px;
  font-weight: 600;
}

.installed-status {
  align-items: center;
  border-radius: 4px;
  display: inline-flex;
  font-size: 10px;
  gap: 3px;
  padding: 2px 6px;

  &--update {
    background: color-mix(in srgb, var(--lumina-warning, #b76500) 14%, transparent);
    color: var(--lumina-warning, #b76500);
  }

  &--dirty {
    background: color-mix(in srgb, #8b5cf6 12%, transparent);
    color: #8b5cf6;
  }
}

.installed-skill-actions {
  align-items: center;
  display: flex;
  flex-shrink: 0;
  gap: 5px;
}

.installed-icon-btn {
  align-items: center;
  background: transparent;
  border: 0.5px solid transparent;
  border-radius: 6px;
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: inline-flex;
  font-size: 15px;
  height: 28px;
  justify-content: center;
  transition: all 120ms ease;
  width: 28px;

  &:hover:not(:disabled),
  &:focus-visible {
    background: var(--lumina-control-hover);
    color: var(--lumina-text);
  }

  &--danger:hover:not(:disabled),
  &--danger:focus-visible {
    background: color-mix(in srgb, var(--lumina-danger) 12%, transparent);
    color: var(--lumina-danger);
  }

  &:disabled {
    cursor: default;
    opacity: 0.45;
  }
}

.installed-update-btn {
  align-items: center;
  background: var(--lumina-primary-soft);
  border: 0;
  border-radius: 6px;
  color: var(--lumina-primary);
  cursor: pointer;
  display: inline-flex;
  font-size: 11px;
  font-weight: 600;
  gap: 5px;
  height: 27px;
  padding: 0 9px;
  transition: all 120ms ease;

  &:hover:not(:disabled) {
    filter: brightness(1.07);
  }

  &:disabled {
    cursor: default;
    opacity: 0.45;
  }
}

.installed-cli-label {
  color: var(--lumina-text-tertiary);
  font-size: 10px;
  padding: 0 4px;
}

.installed-detail-chevron {
  align-items: center;
  color: var(--lumina-text-tertiary);
  display: flex;
  flex-shrink: 0;
  font-size: 15px;
  justify-content: center;
  opacity: 0.22;
  transform: translateX(-3px);
  transition: all 140ms ease;
}

.installed-skill-row:hover .installed-detail-chevron {
  color: var(--lumina-primary);
  opacity: 0.85;
  transform: translateX(0);
}

@media (max-width: 760px) {
  .installed-skill-row {
    gap: 9px;
    padding: 12px;
  }

  .installed-skill-icon {
    display: none;
  }

  .installed-skill-description {
    -webkit-line-clamp: 1;
  }

  .installed-added-date {
    display: none;
  }
}

.macos-window-footer {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-surface-elevated) 70%, transparent);
  border-top: 0.5px solid var(--lumina-separator);
  display: flex;
  flex-shrink: 0;
  font-size: 11px;
  height: 32px;
  justify-content: space-between;
  padding: 0 18px;

  .status-summary-text {
    align-items: center;
    color: var(--lumina-text-secondary);
    display: inline-flex;
    gap: 6px;

    .footer-check-icon {
      color: var(--lumina-primary);
      font-size: 13px;
    }

    .footer-dot-sep {
      color: var(--lumina-separator-strong);
    }
  }
}

.highlight-warn {
  color: var(--lumina-warning, #b76500);
  font-weight: 600;
}

.footer-timestamp {
  color: var(--lumina-text-tertiary);
}

.spin-glyph {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}
</style>
