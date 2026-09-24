<template>
  <NModal
    v-model:show="store.modalVisible"
    :auto-focus="false"
    :trap-focus="false"
    :mask-closable="true"
    :close-on-esc="true"
    transform-origin="center"
  >
    <section class="macos-skills-window" role="dialog" :aria-label="t('skills.title')">
      <!-- macOS Style Unified Header Bar -->
      <header class="macos-window-header">
        <div class="header-leading">
          <Icon icon="solar:magic-stick-3-linear" class="header-icon" />
          <div class="header-titles">
            <h2 class="window-title">{{ t('skills.title') }}</h2>
            <span class="window-subtitle">
              {{ t('skills.categoryAll') }} {{ store.skills.length }}
              <template v-if="store.updateCount > 0">
                · <span class="highlight-warn">{{ t('skills.pillUpdatesAvailable', { count: store.updateCount }) }}</span>
              </template>
            </span>
          </div>
        </div>

        <div class="header-trailing">
          <!-- 1. 重新扫描本地磁盘 (快速发现新安装技能，零网络延迟) -->
          <button
            type="button"
            class="macos-push-btn"
            :disabled="store.loading || store.checkingUpdates"
            :title="t('skills.rescan')"
            @click="handleRescan"
          >
            <Icon icon="solar:refresh-linear" :class="{ 'spin-glyph': store.loading }" />
            <span>{{ store.loading ? t('skills.rescanning') : t('skills.rescan') }}</span>
          </button>

          <!-- 2. 检查远端 Git 更新 -->
          <button
            type="button"
            class="macos-push-btn"
            :disabled="store.checkingUpdates || store.loading"
            :title="t('skills.checkUpdates')"
            @click="handleCheckUpdates"
          >
            <Icon icon="solar:cloud-download-linear" :class="{ 'spin-glyph': store.checkingUpdates }" />
            <span>{{ store.checkingUpdates ? t('skills.checking') : t('skills.checkUpdates') }}</span>
          </button>

          <!-- 3. 一键全部更新 (仅在有可用更新时展示) -->
          <button
            v-if="store.hasUpdates"
            type="button"
            class="macos-push-btn macos-push-btn--primary"
            :disabled="store.updatingPaths.length > 0"
            @click="handleUpdateAll"
          >
            <Icon icon="solar:cloud-check-linear" />
            <span>{{ t('skills.updateAll') }} ({{ store.updateCount }})</span>
          </button>

          <!-- 4. 关闭按钮 -->
          <WorkbenchCloseButton
            @close="store.closeModal"
          />
        </div>
      </header>

      <!-- macOS Toolbar: Search & Segmented Control -->
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

        <!-- macOS Segmented Control -->
        <div class="macos-segmented-bar" role="tablist">
          <button
            type="button"
            role="tab"
            class="segmented-tab"
            :class="{ active: store.selectedCategory === 'all' }"
            @click="store.selectedCategory = 'all'"
          >
            {{ t('skills.categoryAll') }}
            <span class="tab-badge">{{ store.skills.length }}</span>
          </button>
          <button
            v-if="store.updateCount > 0"
            type="button"
            role="tab"
            class="segmented-tab segmented-tab--warning"
            :class="{ active: store.selectedCategory === 'updates' }"
            @click="store.selectedCategory = 'updates'"
          >
            <span class="dot-indicator"></span>
            {{ t('skills.categoryUpdates') }}
            <span class="tab-badge tab-badge--warn">{{ store.updateCount }}</span>
          </button>
          <button
            v-for="cat in store.availableCategories"
            :key="cat"
            type="button"
            role="tab"
            class="segmented-tab"
            :class="{ active: store.selectedCategory === cat }"
            @click="store.selectedCategory = cat"
          >
            {{ getCategoryLabel(cat) }}
            <span class="tab-badge">{{ store.categoryCounts[cat] ?? 0 }}</span>
          </button>
        </div>
      </div>

      <!-- Content Area: Structured Inset Table with Strict Vertical Column Alignment -->
      <main class="macos-content-viewport">
        <div v-if="store.loading" class="macos-empty-state">
          <Icon icon="solar:refresh-linear" class="spin-glyph" />
          <p>{{ t('common.loading') }}</p>
        </div>

        <div v-else-if="store.filteredSkills.length === 0" class="macos-empty-state">
          <Icon icon="solar:box-minimalistic-linear" class="empty-glyph" />
          <p v-if="store.skills.length === 0">{{ t('skills.noSkillsFound') }}</p>
          <p v-else>{{ t('skills.noMatchingSkills') }}</p>
        </div>

        <div v-else class="macos-table-container">
          <!-- Column Header Row -->
          <div class="macos-table-header">
            <div class="col col-agent">{{ t('skills.colAgent') }}</div>
            <div class="col col-name">{{ t('skills.colName') }}</div>
            <div class="col col-status">{{ t('skills.colStatus') }}</div>
            <div class="col col-type">{{ t('skills.colType') }}</div>
            <div class="col col-desc">{{ t('skills.colDesc') }}</div>
            <div class="col col-actions">{{ t('skills.colActions') }}</div>
          </div>

          <!-- Data Rows -->
          <div class="macos-table-body">
            <div
              v-for="skill in store.filteredSkills"
              :key="skill.id"
              v-memo="[skill.id, skill.hasUpdate, skill.isDirty, skill.commitsBehind, store.updatingPaths.includes(skill.path)]"
              class="macos-list-row"
              :class="{ 'row-has-update': skill.hasUpdate, 'row-dirty': skill.isDirty }"
            >
              <!-- 1. Agent 归属 (固定宽度 140px) -->
              <div class="col col-agent">
                <span class="macos-agent-badge" :data-agent="skill.agentType" :title="skill.displayAgentTag || getPrimaryAgentTag(skill)">
                  {{ skill.displayAgentTag || getPrimaryAgentTag(skill) }}
                </span>
              </div>

              <!-- 2. 技能名称与版本 (固定宽度 180px) -->
              <div class="col col-name" :title="skill.name">
                <span class="skill-title">{{ skill.name }}</span>
                <span v-if="skill.version" class="skill-ver-pill">v{{ skill.version }}</span>
              </div>

              <!-- 3. 版本检查与更新状态 (固定宽度 105px 严格对齐) -->
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

              <!-- 4. 本地技能与 Git 来源标识 (固定宽度 130px 严格对齐) -->
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

              <!-- 5. 功能说明与更新摘要 (自适应宽度 1fr) -->
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

              <!-- 6. 操作按钮 (固定靠右对齐 105px) -->
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

      <!-- macOS Style Status Footer -->
      <footer class="macos-window-footer">
        <div class="footer-status-text">
          <span>{{ t('skills.categoryAll') }}: <strong>{{ store.skills.length }}</strong></span>
          <span v-if="store.updateCount > 0" class="warn-text">
            {{ t('skills.categoryUpdates') }}: <strong>{{ store.updateCount }}</strong>
          </span>
          <span v-if="formattedLastChecked" class="time-text">
            {{ t('skills.lastChecked') }} {{ formattedLastChecked }}
          </span>
        </div>
        <button type="button" class="macos-done-btn" @click="store.closeModal">
          {{ t('common.close', '完成') }}
        </button>
      </footer>
    </section>
  </NModal>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { NModal, useMessage } from 'naive-ui'
import { Icon } from '@iconify/vue'
import { useI18n } from 'vue-i18n'
import { useSkillsStore } from '@/stores/skills'
import WorkbenchCloseButton from '@/components/workbench/WorkbenchCloseButton.vue'
import { hasPrimaryModifier, isMacPlatform } from '@/utils/platform-shortcuts'
import type { AgentSkill } from '@/services/skills/skills-service'

const { t } = useI18n()
const store = useSkillsStore()
const message = useMessage()
const isMac = isMacPlatform
const searchInputRef = ref<HTMLInputElement | null>(null)
const localSearch = ref(store.searchQuery)
let searchTimer: ReturnType<typeof setTimeout> | undefined

watch(
  () => store.searchQuery,
  val => {
    if (localSearch.value !== val) {
      localSearch.value = val
    }
  }
)

function handleSearchInput(event: Event) {
  const target = event.target as HTMLInputElement
  localSearch.value = target.value
  if (searchTimer) clearTimeout(searchTimer)
  searchTimer = setTimeout(() => {
    store.searchQuery = localSearch.value
  }, 120)
}

function focusSearch() {
  searchInputRef.value?.focus()
}

function clearSearch() {
  if (searchTimer) clearTimeout(searchTimer)
  localSearch.value = ''
  store.searchQuery = ''
  focusSearch()
}

function handleInputKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    if (localSearch.value || store.searchQuery) {
      clearSearch()
    } else {
      store.closeModal()
    }
    event.stopPropagation()
    return
  }
  event.stopPropagation()
}

function handleFindShortcut(event: KeyboardEvent) {
  if (!store.modalVisible) return
  if (!hasPrimaryModifier(event) || event.key.toLowerCase() !== 'f') return
  event.preventDefault()
  event.stopPropagation()
  searchInputRef.value?.focus()
  searchInputRef.value?.select()
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
      void store.scan()
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

async function handleRescan() {
  const res = await store.scan()
  if (res.newCount > 0) {
    message.success(t('skills.rescanWithNew', { count: res.newCount, total: res.total }))
  } else {
    message.info(t('skills.rescanUpToDate', { total: res.total }))
  }
}

async function handleCheckUpdates() {
  const res = await store.checkUpdates()
  if (res.updateCount > 0) {
    if (res.newCount > 0) {
      message.info(t('skills.checkUpdatesWithNew', { newCount: res.newCount, updateCount: res.updateCount }))
    } else {
      message.info(t('skills.pillUpdatesAvailable', { count: res.updateCount }))
    }
  } else {
    if (res.newCount > 0) {
      message.success(t('skills.rescanWithNew', { count: res.newCount, total: res.total }))
    } else {
      message.success(t('skills.checkUpdatesAllUpToDate', { total: res.total }))
    }
  }
}

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

async function handleUpdateAll() {
  const results = await store.updateAll()
  const successCount = results.filter(r => r.success).length
  if (successCount > 0) {
    message.success(`${t('skills.updateSuccess')}: ${successCount}`)
  }
}
</script>

<style scoped lang="scss">
.macos-skills-window {
  backdrop-filter: var(--lumina-vibrancy, blur(28px));
  background: color-mix(in srgb, var(--lumina-surface-elevated) 96%, transparent);
  border: 0.5px solid var(--lumina-separator-strong);
  border-radius: var(--lumina-radius-lg, 12px);
  box-shadow: var(--lumina-shadow-lg, 0 24px 64px rgba(0, 0, 0, 0.28));
  color: var(--lumina-text);
  display: flex;
  flex-direction: column;
  font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", "SF Pro Display", "PingFang SC", "Hiragino Sans GB", "Microsoft YaHei", sans-serif;
  height: 82vh;
  max-height: 740px;
  max-width: 1040px;
  overflow: hidden;
  user-select: none;
  width: 92vw;
}

/* macOS Window Header */
.macos-window-header {
  align-items: center;
  background: var(--lumina-toolbar-bg, var(--lumina-surface-elevated));
  border-bottom: 0.5px solid var(--lumina-separator);
  display: flex;
  flex-shrink: 0;
  height: 52px;
  justify-content: space-between;
  padding: 0 16px;

  .header-leading {
    align-items: center;
    display: flex;
    gap: 10px;

    .header-icon {
      color: var(--lumina-primary, #39786f);
      font-size: 20px;
    }

    .header-titles {
      display: flex;
      flex-direction: column;
      gap: 1px;

      .window-title {
        color: var(--lumina-text);
        font-size: 13.5px;
        font-weight: 600;
        letter-spacing: -0.1px;
        margin: 0;
      }

      .window-subtitle {
        color: var(--lumina-text-tertiary);
        font-size: 11px;

        .highlight-warn {
          color: var(--lumina-warning, #b76500);
          font-weight: 550;
        }
      }
    }
  }

  .header-trailing {
    align-items: center;
    display: flex;
    gap: 8px;
  }
}

.macos-push-btn {
  align-items: center;
  background: var(--lumina-control-bg);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-sm, 5px);
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.05);
  color: var(--lumina-text);
  cursor: pointer;
  display: inline-flex;
  font-size: 11.5px;
  font-weight: 500;
  gap: 5px;
  height: 26px;
  padding: 0 10px;
  transition: all var(--lumina-duration-fast) var(--lumina-ease-out);

  &:hover:not(:disabled) {
    background: var(--lumina-control-hover);
  }

  &:active:not(:disabled) {
    transform: scale(0.98);
  }

  &:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }

  &--primary {
    background: var(--lumina-primary, #39786f);
    border-color: var(--lumina-primary, #39786f);
    color: #fff;

    &:hover:not(:disabled) {
      filter: brightness(1.1);
    }
  }

  &.spinning svg {
    animation: spin 1s linear infinite;
  }
}

/* Toolbar: Search & Segmented Control */
.macos-window-toolbar {
  border-bottom: 0.5px solid var(--lumina-separator);
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  gap: 10px;
  padding: 10px 16px;
}

.macos-search-box {
  align-items: center;
  background: var(--lumina-control-bg);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-sm, 6px);
  cursor: text;
  display: flex;
  height: 28px;
  padding: 0 8px;
  user-select: text;

  .search-glyph {
    color: var(--lumina-text-tertiary);
    font-size: 14px;
  }

  .macos-search-field {
    background: transparent;
    border: 0;
    color: var(--lumina-text);
    font-size: 12px;
    height: 100%;
    outline: none;
    padding: 0 6px;
    user-select: text;
    width: 100%;

    &::placeholder {
      color: var(--lumina-text-tertiary);
    }
  }

  .search-kbd {
    align-items: center;
    background: color-mix(in srgb, var(--lumina-text) 6%, transparent);
    border: 0.5px solid var(--lumina-separator);
    border-radius: 3px;
    color: var(--lumina-text-tertiary);
    display: inline-flex;
    font-family: var(--lumina-font-mono, monospace);
    font-size: 9.5px;
    padding: 1px 4px;
  }

  .search-clear-glyph {
    background: transparent;
    border: 0;
    color: var(--lumina-text-tertiary);
    cursor: pointer;
    font-size: 15px;
    line-height: 1;
    padding: 2px;

    &:hover {
      color: var(--lumina-text);
    }
  }
}

.macos-segmented-bar {
  align-items: center;
  background: var(--lumina-control-bg);
  border: 0.5px solid var(--lumina-separator);
  border-radius: 7px;
  display: flex;
  gap: 2px;
  overflow-x: auto;
  padding: 2px;
  scrollbar-width: none;

  &::-webkit-scrollbar {
    display: none;
  }

  .segmented-tab {
    align-items: center;
    background: transparent;
    border: 0;
    border-radius: 5px;
    color: var(--lumina-text-secondary);
    cursor: pointer;
    display: inline-flex;
    font-size: 11.5px;
    gap: 5px;
    height: 24px;
    padding: 0 8px;
    transition: all var(--lumina-duration-fast) var(--lumina-ease-out);
    white-space: nowrap;

    .tab-badge {
      background: color-mix(in srgb, var(--lumina-text) 8%, transparent);
      border-radius: 999px;
      font-size: 9.5px;
      padding: 0 4px;
    }

    &:hover {
      color: var(--lumina-text);
    }

    &.active {
      background: var(--lumina-surface-elevated);
      box-shadow: 0 1px 3px rgba(0, 0, 0, 0.12);
      color: var(--lumina-text);
      font-weight: 550;

      .tab-badge {
        background: color-mix(in srgb, var(--lumina-text) 14%, transparent);
        color: var(--lumina-text);
      }
    }

    &--warning {
      color: var(--lumina-warning, #b76500);

      .dot-indicator {
        background: var(--lumina-warning, #b76500);
        border-radius: 50%;
        height: 5px;
        width: 5px;
      }

      .tab-badge--warn {
        background: color-mix(in srgb, var(--lumina-warning, #b76500) 15%, transparent);
        color: var(--lumina-warning, #b76500);
      }
    }
  }
}

/* Content Viewport */
.macos-content-viewport {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
  padding: 12px 16px;
}

.macos-empty-state {
  align-items: center;
  color: var(--lumina-text-tertiary);
  display: flex;
  flex-direction: column;
  gap: 10px;
  justify-content: center;
  padding: 60px 0;

  .empty-glyph {
    font-size: 36px;
    opacity: 0.4;
  }

  p {
    font-size: 12.5px;
    margin: 0;
  }
}

/* Inset Grouped Table with Structured Columns */
.macos-table-container {
  background: var(--lumina-surface-secondary);
  border: 0.5px solid var(--lumina-separator);
  border-radius: 8px;
  display: flex;
  flex: 1;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
}

/* Column Definitions across Header & Rows */
.col {
  align-items: center;
  display: flex;
  min-width: 0;
  padding: 0 6px;
}

.col-agent {
  flex: 0 0 140px;
  max-width: 140px;
}

.col-name {
  flex: 0 0 180px;
  gap: 5px;
  max-width: 180px;
}

.col-status {
  flex: 0 0 105px;
  max-width: 105px;
}

.col-type {
  flex: 0 0 130px;
  max-width: 130px;
}

.col-desc {
  flex: 1 1 140px;
  min-width: 120px;
}

.col-actions {
  flex: 0 0 105px;
  justify-content: flex-end;
  max-width: 105px;
}

/* Table Header */
.macos-table-header {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-control-bg) 60%, var(--lumina-surface-secondary));
  border-bottom: 0.5px solid var(--lumina-separator);
  color: var(--lumina-text-tertiary);
  display: flex;
  flex-shrink: 0;
  font-size: 10.5px;
  font-weight: 600;
  height: 28px;
  letter-spacing: 0.02em;
  padding: 0 8px;
}

/* Table Scrollable Body */
.macos-table-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
}

.macos-list-row {
  align-items: center;
  border-bottom: 0.5px solid var(--lumina-separator);
  content-visibility: auto;
  contain-intrinsic-size: 0 42px;
  display: flex;
  height: 42px;
  padding: 0 8px;
  transition: background var(--lumina-duration-fast) var(--lumina-ease-out);

  &:last-child {
    border-bottom: 0;
  }

  &:hover {
    background: var(--lumina-control-hover);
  }

  &.row-has-update {
    background: color-mix(in srgb, var(--lumina-warning, #b76500) 8%, var(--lumina-surface-secondary));
  }
}

/* Agent Badge */
.macos-agent-badge {
  border-radius: 4px;
  display: block;
  font-size: 10px;
  font-weight: 600;
  letter-spacing: 0.2px;
  max-width: 122px;
  overflow: hidden;
  padding: 2px 6px;
  text-align: center;
  text-overflow: ellipsis;
  text-transform: uppercase;
  white-space: nowrap;

  &[data-agent='agents'] {
    background: color-mix(in srgb, #8b5cf6 14%, transparent);
    border: 0.5px solid color-mix(in srgb, #8b5cf6 30%, transparent);
    color: #7c3aed;
  }

  &[data-agent='codex'] {
    background: color-mix(in srgb, #10b981 14%, transparent);
    border: 0.5px solid color-mix(in srgb, #10b981 30%, transparent);
    color: #059669;
  }

  &[data-agent='antigravity'] {
    background: color-mix(in srgb, #6366f1 14%, transparent);
    border: 0.5px solid color-mix(in srgb, #6366f1 30%, transparent);
    color: #4f46e5;
  }

  &[data-agent='cursor'] {
    background: color-mix(in srgb, #0ea5e9 14%, transparent);
    border: 0.5px solid color-mix(in srgb, #0ea5e9 30%, transparent);
    color: #0284c7;
  }

  &[data-agent='trae'] {
    background: color-mix(in srgb, #14b8a6 14%, transparent);
    border: 0.5px solid color-mix(in srgb, #14b8a6 30%, transparent);
    color: #0d9488;
  }

  &[data-agent='claude'] {
    background: color-mix(in srgb, #f97316 14%, transparent);
    border: 0.5px solid color-mix(in srgb, #f97316 30%, transparent);
    color: #ea580c;
  }

  &[data-agent='qoder'] {
    background: color-mix(in srgb, #ef4444 14%, transparent);
    border: 0.5px solid color-mix(in srgb, #ef4444 30%, transparent);
    color: #dc2626;
  }

  &[data-agent='opencode'] {
    background: color-mix(in srgb, #38bdf8 14%, transparent);
    border: 0.5px solid color-mix(in srgb, #38bdf8 30%, transparent);
    color: #0284c7;
  }

  &[data-agent='windsurf'] {
    background: color-mix(in srgb, #10b981 14%, transparent);
    border: 0.5px solid color-mix(in srgb, #10b981 30%, transparent);
    color: #059669;
  }

  &[data-agent='cline'] {
    background: color-mix(in srgb, #8b5cf6 14%, transparent);
    border: 0.5px solid color-mix(in srgb, #8b5cf6 30%, transparent);
    color: #7c3aed;
  }

  &[data-agent='project'] {
    background: color-mix(in srgb, #a855f7 14%, transparent);
    border: 0.5px solid color-mix(in srgb, #a855f7 30%, transparent);
    color: #9333ea;
  }
}

/* Skill Name Column */
.skill-title {
  color: var(--lumina-text);
  font-size: 12.5px;
  font-weight: 550;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.skill-ver-pill {
  background: var(--lumina-control-bg);
  border-radius: 3px;
  color: var(--lumina-text-tertiary);
  flex-shrink: 0;
  font-family: var(--lumina-font-mono, monospace);
  font-size: 9.5px;
  padding: 1px 4px;
}

/* Apple Status Capsule */
.apple-capsule {
  align-items: center;
  border-radius: 999px;
  display: inline-flex;
  font-size: 10px;
  gap: 4px;
  padding: 1px 6px;
  white-space: nowrap;

  .capsule-dot {
    border-radius: 50%;
    flex-shrink: 0;
    height: 5px;
    width: 5px;
  }

  &--warn {
    background: color-mix(in srgb, var(--lumina-warning, #b76500) 14%, transparent);
    color: var(--lumina-warning, #b76500);

    .capsule-dot {
      background: var(--lumina-warning, #b76500);
    }
  }

  &--dirty {
    background: color-mix(in srgb, #eab308 16%, transparent);
    color: #ca8a04;

    .capsule-dot {
      background: #ca8a04;
    }
  }

  &--ok {
    background: color-mix(in srgb, var(--lumina-success, #27853f) 14%, transparent);
    color: var(--lumina-success, #27853f);

    .capsule-dot {
      background: var(--lumina-success, #27853f);
    }
  }
}

/* Type / Source Column */
.source-tag {
  align-items: center;
  color: var(--lumina-text-tertiary);
  display: inline-flex;
  font-size: 11px;
  gap: 4px;
  max-width: 120px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;

  svg {
    flex-shrink: 0;
    font-size: 12px;
  }

  &--git {
    font-family: var(--lumina-font-mono, monospace);
    font-size: 10px;

    .git-branch-text {
      color: var(--lumina-text-secondary);
      max-width: 50px;
      overflow: hidden;
      text-overflow: ellipsis;
    }

    .git-hash-text {
      opacity: 0.8;
    }
  }
}

/* Description Column */
.desc-update-text {
  align-items: center;
  color: var(--lumina-warning, #b76500);
  display: inline-flex;
  font-size: 11.5px;
  font-weight: 500;
  gap: 3px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;

  svg {
    flex-shrink: 0;
  }
}

.desc-normal-text {
  color: var(--lumina-text-secondary);
  font-size: 11.5px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.desc-empty-text {
  color: var(--lumina-text-tertiary);
  font-size: 12px;
  opacity: 0.5;
}

/* Action Button */
.macos-row-action-btn {
  align-items: center;
  background: var(--lumina-control-bg);
  border: 0.5px solid var(--lumina-separator);
  border-radius: 5px;
  color: var(--lumina-text);
  cursor: pointer;
  display: inline-flex;
  flex-shrink: 0;
  font-size: 11px;
  font-weight: 500;
  gap: 4px;
  height: 24px;
  padding: 0 8px;
  transition: all var(--lumina-duration-fast) var(--lumina-ease-out);
  white-space: nowrap;

  &:hover:not(:disabled) {
    background: var(--lumina-control-hover);
  }

  &:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }

  &--primary {
    background: var(--lumina-primary, #39786f);
    border-color: var(--lumina-primary, #39786f);
    color: #fff;

    &:hover:not(:disabled) {
      filter: brightness(1.1);
    }
  }

  &--ghost {
    background: transparent;
    border-color: transparent;
    color: var(--lumina-text-secondary);

    &:hover {
      background: var(--lumina-control-hover);
      color: var(--lumina-text);
    }
  }
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

/* Status Footer */
.macos-window-footer {
  align-items: center;
  background: var(--lumina-toolbar-bg, var(--lumina-surface-elevated));
  border-top: 0.5px solid var(--lumina-separator);
  display: flex;
  flex-shrink: 0;
  height: 38px;
  justify-content: space-between;
  padding: 0 16px;

  .footer-status-text {
    align-items: center;
    color: var(--lumina-text-secondary);
    display: flex;
    font-size: 11.5px;
    gap: 12px;

    .warn-text {
      color: var(--lumina-warning, #b76500);
    }

    .time-text {
      color: var(--lumina-text-tertiary);
      font-size: 10.5px;
    }
  }

  .macos-done-btn {
    background: var(--lumina-control-bg);
    border: 0.5px solid var(--lumina-separator);
    border-radius: var(--lumina-radius-sm, 5px);
    color: var(--lumina-text);
    cursor: pointer;
    font-size: 11.5px;
    height: 24px;
    padding: 0 12px;
    transition: all var(--lumina-duration-fast) var(--lumina-ease-out);

    &:hover {
      background: var(--lumina-control-hover);
    }
  }
}
</style>
