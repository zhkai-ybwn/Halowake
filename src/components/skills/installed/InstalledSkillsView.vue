<template>
  <div class="installed-skills-view">
    <!-- macOS Toolbar: Search & View Mode Switcher -->
    <div class="macos-window-toolbar">
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

      <!-- 分类维度切换：按 Agent / 按 SDLC 阶段 / 自定义分类 (Question 5) -->
      <div class="category-dimension-bar">
        <button
          type="button"
          class="dimension-toggle-btn"
          :class="{ active: categoryDimension === 'agent' }"
          @click="categoryDimension = 'agent'"
        >
          按 Agent
        </button>
        <button
          type="button"
          class="dimension-toggle-btn"
          :class="{ active: categoryDimension === 'sdlc' }"
          @click="categoryDimension = 'sdlc'"
        >
          按 SDLC 阶段
        </button>
      </div>

      <!-- Segmented Tabs for the active dimension -->
      <div class="macos-segmented-bar" role="tablist">
        <button
          type="button"
          role="tab"
          class="segmented-tab"
          :class="{ active: selectedFilter === 'all' }"
          @click="selectedFilter = 'all'"
        >
          {{ t('skills.categoryAll') }}
          <span class="tab-badge">{{ store.skills.length }}</span>
        </button>

        <button
          v-if="store.updateCount > 0"
          type="button"
          role="tab"
          class="segmented-tab segmented-tab--warning"
          :class="{ active: selectedFilter === 'updates' }"
          @click="selectedFilter = 'updates'"
        >
          <span class="dot-indicator"></span>
          {{ t('skills.categoryUpdates') }}
          <span class="tab-badge tab-badge--warn">{{ store.updateCount }}</span>
        </button>

        <!-- A. Agent 维度标签 -->
        <template v-if="categoryDimension === 'agent'">
          <button
            v-for="cat in store.availableCategories"
            :key="cat"
            type="button"
            role="tab"
            class="segmented-tab"
            :class="{ active: selectedFilter === cat }"
            @click="selectedFilter = cat"
          >
            {{ getCategoryLabel(cat) }}
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
            class="segmented-tab"
            :class="{ active: selectedFilter === stage.key }"
            @click="selectedFilter = stage.key"
          >
            {{ stage.label }}
            <span class="tab-badge">{{ stageCounts[stage.key] || 0 }}</span>
          </button>
        </template>

        <!-- C. 用户自定义分类标签 (若存在) -->
        <button
          v-for="cat in exploreStore.allCustomCategories"
          :key="cat"
          type="button"
          role="tab"
          class="segmented-tab segmented-tab--custom"
          :class="{ active: selectedFilter === cat }"
          @click="selectedFilter = cat"
        >
          #{{ cat }}
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

      <div v-else class="macos-table-container">
        <!-- Column Header Row -->
        <div class="macos-table-header">
          <div class="col col-agent">{{ t('skills.colAgent') }}</div>
          <div class="col col-stage">阶段 / 自定义分类</div>
          <div class="col col-name">{{ t('skills.colName') }}</div>
          <div class="col col-status">{{ t('skills.colStatus') }}</div>
          <div class="col col-type">{{ t('skills.colType') }}</div>
          <div class="col col-desc">{{ t('skills.colDesc') }}</div>
          <div class="col col-actions">{{ t('skills.colActions') }}</div>
        </div>

        <!-- Data Rows -->
        <div class="macos-table-body">
          <div
            v-for="skill in displaySkills"
            :key="skill.id"
            class="macos-list-row"
            :class="{ 'row-has-update': skill.hasUpdate, 'row-dirty': skill.isDirty }"
          >
            <!-- 1. Agent 归属 -->
            <div class="col col-agent">
              <span class="macos-agent-badge" :data-agent="skill.agentType" :title="skill.displayAgentTag || getPrimaryAgentTag(skill)">
                {{ skill.displayAgentTag || getPrimaryAgentTag(skill) }}
              </span>
            </div>

            <!-- 2. SDLC 阶段分类 + 自定义标签 (Question 5) -->
            <div class="col col-stage">
              <div class="stage-tag-cell">
                <SkillStageBadge :stage="getSkillSdlcStage(skill)" />
                <span
                  v-for="tag in getCustomTags(skill.id)"
                  :key="tag"
                  class="custom-tag-micro"
                  :title="`自定义分类: ${tag}`"
                >
                  #{{ tag }}
                </span>
                <button
                  type="button"
                  class="add-tag-micro-btn"
                  title="为该技能添加自定义分类"
                  @click.stop="promptAddTag(skill)"
                >
                  +
                </button>
              </div>
            </div>

            <!-- 3. 技能名称与版本 -->
            <div class="col col-name" :title="skill.name">
              <span class="skill-title">{{ skill.name }}</span>
              <span v-if="skill.version" class="skill-ver-pill">v{{ skill.version }}</span>
            </div>

            <!-- 4. 版本检查与更新状态 -->
            <div class="col col-status">
              <span v-if="skill.hasUpdate" class="apple-capsule apple-capsule--warn" :title="skill.updateSummary || undefined">
                <span class="capsule-dot"></span>
                {{ t('skills.updateAvailable') }}
              </span>
              <span v-else-if="skill.isDirty" class="apple-capsule apple-capsule--dirty">
                <span class="capsule-dot"></span>
                {{ t('skills.modifiedLocally') }}
              </span>
              <span v-else class="apple-capsule apple-capsule--ok">
                <span class="capsule-dot"></span>
                {{ t('skills.upToDate') }}
              </span>
            </div>

            <!-- 5. 本地技能与 Git 来源标识 -->
            <div class="col col-type">
              <span v-if="skill.isGitRepo" class="source-tag source-tag--git" :title="skill.gitRemoteUrl || undefined">
                <Icon icon="solar:code-file-linear" />
                <span class="git-branch-text">{{ skill.gitBranch || 'git' }}</span>
                <span v-if="skill.currentCommit" class="git-hash-text">@{{ skill.currentCommit }}</span>
              </span>
              <span v-else class="source-tag source-tag--local">
                <Icon icon="solar:folder-linear" />
                <span>{{ t('skills.localOnly') }}</span>
              </span>
            </div>

            <!-- 6. 功能说明与更新摘要 -->
            <div class="col col-desc" :title="skill.updateSummary || skill.cleanDesc || cleanDescription(skill.description) || ''">
              <span v-if="skill.updateSummary" class="desc-update-text">
                <Icon icon="solar:arrow-up-linear" />
                {{ skill.updateSummary }}
              </span>
              <span v-else-if="skill.cleanDesc || cleanDescription(skill.description)" class="desc-normal-text">
                {{ skill.cleanDesc || cleanDescription(skill.description) }}
              </span>
              <span v-else class="desc-empty-text">—</span>
            </div>

            <!-- 7. 操作按钮 -->
            <div class="col col-actions">
              <button
                v-if="skill.hasUpdate && skill.isGitRepo"
                type="button"
                class="macos-row-action-btn macos-row-action-btn--primary"
                :disabled="store.updatingPaths.includes(skill.path)"
                @click="handleUpdateSingle(skill)"
              >
                <Icon
                  :icon="store.updatingPaths.includes(skill.path) ? 'solar:refresh-linear' : 'solar:cloud-download-linear'"
                  :class="{ 'spin-glyph': store.updatingPaths.includes(skill.path) }"
                />
                <span>{{ store.updatingPaths.includes(skill.path) ? t('skills.updating') : t('skills.updateSingle') }}</span>
              </button>

              <button
                v-else
                type="button"
                class="macos-row-action-btn macos-row-action-btn--ghost"
                :title="t('skills.openFolder')"
                @click="store.openFolder(skill.path)"
              >
                <Icon icon="solar:folder-with-files-linear" />
                <span>{{ t('skills.openFolder') }}</span>
              </button>
            </div>
          </div>
        </div>
      </div>
    </main>

    <!-- macOS Inset Status Footer Bar -->
    <footer class="macos-window-footer">
      <div class="footer-leading">
        <span class="status-summary-text">
          {{ t('skills.categoryAll') }} {{ store.skills.length }}
          <template v-if="store.updateCount > 0">
            · <span class="highlight-warn">{{ t('skills.pillUpdatesAvailable', { count: store.updateCount }) }}</span>
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
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { useMessage } from 'naive-ui'
import { Icon } from '@iconify/vue'
import { useI18n } from 'vue-i18n'
import { useSkillsStore } from '@/stores/skills'
import { useSkillExploreStore } from '@/stores/skill-explore'
import { inferSkillStage } from '@/services/skills/skill-catalog-service'
import type { AgentSkill, SdlcStage } from '@/types/skills'
import SkillStageBadge from '../common/SkillStageBadge.vue'

const { t } = useI18n()
const message = useMessage()
const store = useSkillsStore()
const exploreStore = useSkillExploreStore()

const searchInputRef = ref<HTMLInputElement | null>(null)
const localSearch = ref('')
let searchTimer: ReturnType<typeof setTimeout> | null = null

const categoryDimension = ref<'agent' | 'sdlc'>('agent')
const selectedFilter = ref<string>('all')

const isMac = computed(() => {
  if (typeof navigator === 'undefined') return false
  return /Mac|iPod|iPhone|iPad/.test(navigator.platform)
})

const availableStages: Array<{ key: SdlcStage; label: string }> = [
  { key: 'requirement', label: '需求与规格' },
  { key: 'design', label: '体验与架构' },
  { key: 'coding', label: '工程与编码' },
  { key: 'testing', label: '质量与测试' },
  { key: 'delivery', label: '交付与沉淀' },
]

function getSkillSdlcStage(skill: AgentSkill): SdlcStage {
  const match = exploreStore.catalogSkills.find(
    s => s.id.toLowerCase() === skill.id.toLowerCase() || s.name.toLowerCase() === skill.name.toLowerCase()
  )
  if (match) return match.stage
  return inferSkillStage(skill.name, skill.description)
}

function getCustomTags(skillId: string): string[] {
  return exploreStore.userCustomCategories[skillId] || []
}

function promptAddTag(skill: AgentSkill) {
  const input = window.prompt(`为技能 [${skill.name}] 添加自定义分类标签 (例如: 前端规范、林洋项目):`)
  if (input && input.trim()) {
    exploreStore.addSkillCustomCategory(skill.id, input.trim())
    message.success(`已为 [${skill.name}] 添加分类: #${input.trim()}`)
  }
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
  if (selectedFilter.value === 'updates') {
    list = list.filter(s => s.hasUpdate)
  } else if (selectedFilter.value !== 'all') {
    if (categoryDimension.value === 'agent') {
      list = list.filter(
        s =>
          s.agentType === selectedFilter.value ||
          (s.associatedAgents && s.associatedAgents.includes(selectedFilter.value))
      )
    } else if (categoryDimension.value === 'sdlc') {
      list = list.filter(s => getSkillSdlcStage(s) === selectedFilter.value)
    }

    // 检查是否选中的是自定义分类
    if (exploreStore.allCustomCategories.includes(selectedFilter.value)) {
      list = list.filter(s => getCustomTags(s.id).includes(selectedFilter.value))
    }
  }

  // 2. 搜索查询
  const q = localSearch.value.trim().toLowerCase()
  if (q) {
    list = list.filter(s => {
      const tags = getCustomTags(s.id).join(' ').toLowerCase()
      return (
        s.name.toLowerCase().includes(q) ||
        s.description.toLowerCase().includes(q) ||
        s.agentLabel.toLowerCase().includes(q) ||
        tags.includes(q)
      )
    })
  }

  return list
})

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
  if ((e.metaKey || e.ctrlKey) && (e.key === 'f' || e.key === 'F')) {
    e.preventDefault()
    searchInputRef.value?.focus()
    searchInputRef.value?.select()
  }
}

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
  if (skill.isDirty) {
    message.warning(t('skills.dirtyWarning'))
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

.macos-window-toolbar {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-surface-base) 80%, transparent);
  border-bottom: 0.5px solid var(--lumina-separator);
  display: flex;
  flex-shrink: 0;
  gap: 12px;
  padding: 8px 18px;
}

.macos-search-box {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-fill-quaternary) 75%, transparent);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-md, 7px);
  cursor: text;
  display: flex;
  gap: 6px;
  height: 28px;
  padding: 0 8px;
  transition: all 120ms ease;
  width: 220px;
  flex-shrink: 0;

  &:focus-within {
    background: var(--lumina-surface-elevated);
    border-color: var(--lumina-primary);
    box-shadow: 0 0 0 2px var(--lumina-primary-soft);
  }

  .search-glyph {
    color: var(--lumina-text-tertiary);
    font-size: 13px;
    flex-shrink: 0;
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

.category-dimension-bar {
  display: flex;
  background: var(--lumina-control-bg);
  border: 0.5px solid var(--lumina-separator);
  border-radius: 6px;
  padding: 2px;
  gap: 2px;
  flex-shrink: 0;
}

.dimension-toggle-btn {
  background: transparent;
  border: none;
  border-radius: 4px;
  color: var(--lumina-text-secondary);
  cursor: pointer;
  font-size: 11px;
  font-weight: 500;
  padding: 2px 7px;
  transition: all 120ms ease;

  &.active {
    background: var(--lumina-surface-elevated);
    color: var(--lumina-primary);
    font-weight: 600;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.08);
  }
}

.macos-segmented-bar {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-fill-quaternary) 65%, transparent);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-md, 7px);
  display: flex;
  gap: 2px;
  flex: 1;
  overflow-x: auto;
  padding: 2px;

  &::-webkit-scrollbar {
    display: none;
  }

  .segmented-tab {
    align-items: center;
    background: transparent;
    border: none;
    border-radius: 5px;
    color: var(--lumina-text-secondary);
    cursor: pointer;
    display: inline-flex;
    font-size: 11px;
    font-weight: 500;
    gap: 4px;
    height: 24px;
    padding: 0 7px;
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

.macos-table-container {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
}

.macos-table-header {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-surface-elevated) 60%, transparent);
  border-bottom: 0.5px solid var(--lumina-separator);
  color: var(--lumina-text-tertiary);
  display: flex;
  flex-shrink: 0;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.02em;
  padding: 7px 18px;
  user-select: none;
}

.col {
  padding: 0 6px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;

  &-agent {
    flex-shrink: 0;
    width: 120px;
  }

  &-stage {
    flex-shrink: 0;
    width: 150px;
  }

  &-name {
    flex-shrink: 0;
    width: 160px;
  }

  &-status {
    flex-shrink: 0;
    width: 100px;
  }

  &-type {
    flex-shrink: 0;
    width: 120px;
  }

  &-desc {
    flex: 1;
    min-width: 0;
  }

  &-actions {
    display: flex;
    flex-shrink: 0;
    justify-content: flex-end;
    width: 100px;
  }
}

.stage-tag-cell {
  display: flex;
  align-items: center;
  gap: 4px;
}

.custom-tag-micro {
  background: var(--lumina-primary-soft);
  color: var(--lumina-primary);
  font-size: 11.5px;
  font-weight: 600;
  padding: 1px 5px;
  border-radius: 3px;
}

.add-tag-micro-btn {
  background: transparent;
  border: 0.5px dashed var(--lumina-separator);
  color: var(--lumina-text-tertiary);
  cursor: pointer;
  border-radius: 3px;
  font-size: 11.5px;
  line-height: 1;
  padding: 1px 4px;

  &:hover {
    border-color: var(--lumina-primary);
    color: var(--lumina-primary);
  }
}

.macos-table-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
}

.macos-list-row {
  align-items: center;
  border-bottom: 0.5px solid color-mix(in srgb, var(--lumina-separator) 50%, transparent);
  display: flex;
  font-size: 12px;
  height: 40px;
  padding: 0 18px;
  transition: background-color 80ms ease;

  &:hover {
    background: color-mix(in srgb, var(--lumina-fill-quaternary) 50%, transparent);
  }

  &.row-has-update {
    background: color-mix(in srgb, var(--lumina-warning, #b76500) 3%, transparent);
  }
}

.macos-agent-badge {
  align-items: center;
  background: var(--lumina-surface-secondary);
  border: 0.5px solid var(--lumina-separator);
  border-radius: 4px;
  color: var(--lumina-text-secondary);
  display: inline-flex;
  font-size: 11.5px;
  font-weight: 500;
  max-width: 100%;
  overflow: hidden;
  padding: 1.5px 6px;
  text-overflow: ellipsis;
  white-space: nowrap;

  &[data-agent='antigravity'] {
    border-color: color-mix(in srgb, var(--lumina-primary) 30%, transparent);
    color: var(--lumina-primary);
  }
}

.skill-title {
  font-weight: 600;
  color: var(--lumina-text);
}

.skill-ver-pill {
  background: color-mix(in srgb, var(--lumina-fill-tertiary) 70%, transparent);
  border-radius: 3px;
  color: var(--lumina-text-secondary);
  font-size: 11px;
  font-weight: 500;
  margin-left: 6px;
  padding: 1px 5px;
}

.apple-capsule {
  align-items: center;
  border-radius: 9999px;
  display: inline-flex;
  font-size: 11.5px;
  font-weight: 500;
  gap: 4px;
  padding: 1.5px 6px;

  .capsule-dot {
    border-radius: 50%;
    height: 5px;
    width: 5px;
  }

  &--ok {
    background: color-mix(in srgb, var(--lumina-success, #27853f) 12%, transparent);
    color: var(--lumina-success, #27853f);

    .capsule-dot {
      background: var(--lumina-success, #27853f);
    }
  }

  &--warn {
    background: color-mix(in srgb, var(--lumina-warning, #b76500) 12%, transparent);
    color: var(--lumina-warning, #b76500);

    .capsule-dot {
      background: var(--lumina-warning, #b76500);
    }
  }

  &--dirty {
    background: color-mix(in srgb, #8b5cf6 12%, transparent);
    color: #8b5cf6;

    .capsule-dot {
      background: #8b5cf6;
    }
  }
}

.source-tag {
  align-items: center;
  color: var(--lumina-text-secondary);
  display: inline-flex;
  font-size: 11px;
  gap: 4px;

  &--git {
    color: var(--lumina-text);
  }

  .git-branch-text {
    font-weight: 500;
  }

  .git-hash-text {
    color: var(--lumina-text-tertiary);
    font-family: var(--lumina-font-mono, monospace);
    font-size: 10px;
  }
}

.desc-update-text {
  align-items: center;
  color: var(--lumina-warning, #b76500);
  display: inline-flex;
  gap: 4px;
  font-size: 11.5px;
}

.desc-normal-text {
  color: var(--lumina-text-secondary);
  font-size: 11.5px;
}

.desc-empty-text {
  color: var(--lumina-text-tertiary);
}

.macos-row-action-btn {
  align-items: center;
  border-radius: 5px;
  cursor: pointer;
  display: inline-flex;
  font-size: 11px;
  font-weight: 500;
  gap: 4px;
  height: 24px;
  padding: 0 8px;
  transition: all 120ms ease;

  &--ghost {
    background: transparent;
    border: 0.5px solid var(--lumina-separator);
    color: var(--lumina-text-secondary);

    &:hover {
      background: var(--lumina-control-hover);
      color: var(--lumina-text);
    }
  }

  &--primary {
    background: var(--lumina-primary);
    border: none;
    color: #ffffff;

    &:hover:not(:disabled) {
      filter: brightness(1.08);
    }
  }

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
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
