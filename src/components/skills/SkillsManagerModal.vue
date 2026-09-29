<template>
  <NModal
    v-model:show="store.modalVisible"
    :auto-focus="false"
    :trap-focus="false"
    :mask-closable="true"
    :close-on-esc="true"
    transform-origin="center"
  >
    <section
      class="skills-panel"
      :class="{ maximized }"
      role="dialog"
      aria-modal="true"
      aria-labelledby="skills-manager-title"
    >
      <!-- 1. Window-Level Chrome Header: Strictly Window Identity & Window Actions -->
      <header class="skills-titlebar" @dblclick="handleTitlebarDoubleClick">
        <div class="title-identity">
          <span class="app-icon">
            <Icon icon="solar:magic-stick-3-linear" />
          </span>
          <div>
            <h2 id="skills-manager-title">{{ t('skills.title') }}</h2>
            <p>
              <template v-if="exploreStore.activeMainTab === 'installed'">
                {{ t('skills.review.managerInstalledSubtitle') }}
                <template v-if="store.updateCount > 0">
                  · <span class="highlight-warn">{{ t('skills.pillUpdatesAvailable', { count: store.updateCount }) }}</span>
                </template>
              </template>
              <template v-else-if="exploreStore.activeMainTab === 'explore'">
                {{ t('skills.review.managerExploreSubtitle') }}
              </template>
              <template v-else>{{ t('skills.review.managerHistorySubtitle') }}</template>
            </p>
          </div>
        </div>

        <div class="window-actions">
          <button
            class="maximize-button"
            type="button"
            :title="maximized ? t('workbench.restore', '还原窗口') : t('workbench.maximize', '最大化窗口')"
            :aria-label="maximized ? t('workbench.restore', '还原窗口') : t('workbench.maximize', '最大化窗口')"
            @click.stop="toggleMaximized"
          >
            <Icon :icon="maximized ? 'solar:minimize-square-3-linear' : 'solar:maximize-square-3-linear'" />
          </button>
          <WorkbenchCloseButton @close="store.closeModal" />
        </div>
      </header>

      <!-- 2. Inner Workspace Container -->
      <div class="skills-workspace">
        <!-- Inner Sub-Navbar: Navigation Tabs (Left) & Page Actions (Right) -->
        <nav class="skills-subnav" :aria-label="t('skills.review.managerNavigation')">
          <!-- Left: Segmented Main Tabs Switcher -->
          <div class="workspace-tabs" role="tablist">
            <button
              type="button"
              role="tab"
              :aria-selected="exploreStore.activeMainTab === 'installed'"
              class="workspace-tab"
              :class="{ active: exploreStore.activeMainTab === 'installed' }"
              @click="exploreStore.activeMainTab = 'installed'"
            >
              <Icon icon="solar:folder-with-files-linear" />
              <span>{{ t('skills.tabInstalled') }}</span>
              <span class="tab-counter">{{ store.skills.length }}</span>
              <span v-if="store.updateCount > 0" class="tab-warn-dot"></span>
            </button>

            <button
              type="button"
              role="tab"
              :aria-selected="exploreStore.activeMainTab === 'explore'"
              class="workspace-tab"
              :class="{ active: exploreStore.activeMainTab === 'explore' }"
              @click="exploreStore.activeMainTab = 'explore'"
            >
              <Icon icon="solar:planet-2-linear" />
              <span>{{ t('skills.tabExplore') }}</span>
              <span class="tab-sparkle-pill">{{ exploreStore.catalogSkills.length }}</span>
            </button>

            <button
              type="button"
              role="tab"
              :aria-selected="exploreStore.activeMainTab === 'history'"
              class="workspace-tab"
              :class="{ active: exploreStore.activeMainTab === 'history' }"
              @click="exploreStore.activeMainTab = 'history'"
            >
              <Icon icon="solar:history-linear" />
              <span>{{ t('skills.review.history') }}</span>
            </button>
          </div>

          <!-- Right: Page Operations -->
          <div class="workspace-actions">
            <!-- 重新扫描本地磁盘 (两边都可以用) -->
            <button
              v-if="exploreStore.activeMainTab !== 'history'"
              type="button"
              class="page-action-btn"
              :disabled="store.loading || store.checkingUpdates"
              :title="t('skills.rescan')"
              @click="handleRescan"
            >
              <Icon icon="solar:refresh-linear" :class="{ 'spin-glyph': store.loading }" />
              <span>{{ store.loading ? t('skills.rescanning') : t('skills.rescan') }}</span>
            </button>

            <!-- 已安装视图下的 Git 检查与批量更新操作 -->
            <template v-if="exploreStore.activeMainTab === 'installed'">
              <button
                type="button"
                class="page-action-btn"
                :disabled="store.checkingUpdates || store.loading"
                :title="t('skills.checkUpdates')"
                @click="handleCheckUpdates"
              >
                <Icon icon="solar:cloud-download-linear" :class="{ 'spin-glyph': store.checkingUpdates }" />
                <span>{{ store.checkingUpdates ? t('skills.checking') : t('skills.checkUpdates') }}</span>
              </button>

              <button
                v-if="store.hasUpdates"
                type="button"
                class="page-action-btn page-action-btn--primary"
                :disabled="store.updatingPaths.length > 0"
                @click="handleUpdateAll"
              >
                <Icon icon="solar:cloud-check-linear" />
                <span>{{ t('skills.updateAll') }} ({{ store.updateCount }})</span>
              </button>
            </template>

            <!-- 探索视图下的目录同步 -->
            <template v-else-if="exploreStore.activeMainTab === 'explore'">
              <button
                type="button"
                class="page-action-btn"
                :disabled="exploreStore.isSyncing"
                :title="t('skills.syncRegistryTip', '从远端同步最新推荐技能与流水线')"
                @click="handleSyncRegistry"
              >
                <Icon icon="solar:restart-linear" :class="{ 'spin-glyph': exploreStore.isSyncing }" />
                <span>{{ exploreStore.isSyncing ? t('skills.syncingRegistry', '同步中…') : t('skills.syncRegistry', '同步开源库') }}</span>
              </button>
            </template>
          </div>
        </nav>

        <!-- 3. Views Container (Keep-Alive via v-show) -->
        <div class="skills-body-container">
          <InstalledSkillsView v-show="exploreStore.activeMainTab === 'installed'" />
          <SkillExploreView v-show="exploreStore.activeMainTab === 'explore'" />
          <SkillInstallHistoryView v-show="exploreStore.activeMainTab === 'history'" />
        </div>
      </div>

      <!-- Root Modals & Drawers -->
      <SkillInstallModal />
      <SkillDetailDrawer />
    </section>
  </NModal>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { NModal, useMessage } from 'naive-ui'
import { Icon } from '@iconify/vue'
import { useI18n } from 'vue-i18n'
import { useSkillsStore } from '@/stores/skills'
import { useSkillExploreStore } from '@/stores/skill-explore'
import WorkbenchCloseButton from '@/components/workbench/WorkbenchCloseButton.vue'
import InstalledSkillsView from './installed/InstalledSkillsView.vue'
import SkillExploreView from './explore/SkillExploreView.vue'
import SkillInstallHistoryView from './installed/SkillInstallHistoryView.vue'
import SkillInstallModal from './explore/SkillInstallModal.vue'
import SkillDetailDrawer from './explore/SkillDetailDrawer.vue'
import { GIT_REPO_STORAGE_KEY } from '@/constants/git'

const { t } = useI18n()
const store = useSkillsStore()
const exploreStore = useSkillExploreStore()
const message = useMessage()

const maximized = ref(false)

function toggleMaximized() {
  maximized.value = !maximized.value
}

function handleTitlebarDoubleClick(event: MouseEvent) {
  if (!(event.target instanceof Element) || !event.target.closest('button')) {
    toggleMaximized()
  }
}

watch(
  () => store.modalVisible,
  visible => {
    if (visible) {
      const currentCwd = localStorage.getItem(GIT_REPO_STORAGE_KEY) || undefined
      void store.scan(currentCwd)
    }
  }
)

async function handleRescan() {
  const currentCwd = localStorage.getItem(GIT_REPO_STORAGE_KEY) || undefined
  const res = await store.scan(currentCwd)
  if (store.errorMessage) {
    message.error(t('skills.review.localScanFailed', { error: store.errorMessage }))
    return
  }
  store.revealInstalledSkills(res.newCount === 1 ? res.newSkills[0].name : '')
  exploreStore.activeMainTab = 'installed'
  if (res.newCount > 0) {
    message.success(t('skills.rescanWithNew', { count: res.newCount, total: res.total }))
  } else {
    message.info(t('skills.rescanUpToDate', { total: res.total }))
  }
}

async function handleCheckUpdates() {
  const currentCwd = localStorage.getItem(GIT_REPO_STORAGE_KEY) || undefined
  const res = await store.checkUpdates(currentCwd)
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

async function handleUpdateAll() {
  const results = await store.updateAll()
  const successCount = results.filter(r => r.success).length
  if (successCount > 0) {
    message.success(`${t('skills.updateSuccess')}: ${successCount}`)
  }
}

async function handleSyncRegistry() {
  try {
    const res = await exploreStore.syncRegistry()
    message.success(res.message || t('skills.review.syncFallback', { count: res.totalCount }))
  } catch (err: unknown) {
    message.error((err as Error)?.message || t('skills.review.syncError'))
  }
}
</script>

<style scoped lang="scss">
.skills-panel {
  background: color-mix(in srgb, var(--lumina-surface-elevated) 96%, transparent);
  border: 0.5px solid var(--lumina-separator-strong);
  border-radius: 12px;
  box-shadow: 0 0 0 0.5px color-mix(in srgb, var(--lumina-text) 8%, transparent),
    0 2px 8px rgb(0 0 0 / 10%),
    0 20px 60px rgb(0 0 0 / 20%);
  color: var(--lumina-text);
  display: grid;
  grid-template-rows: 54px minmax(0, 1fr);
  height: min(840px, calc(100vh - 40px));
  overflow: hidden;
  transition: border-radius var(--lumina-duration-normal) var(--lumina-ease-out),
    height var(--lumina-duration-normal) var(--lumina-ease-out),
    width var(--lumina-duration-normal) var(--lumina-ease-out);
  width: min(1120px, calc(100vw - 40px));

  &.maximized {
    border-radius: 8px;
    height: calc(100vh - 12px);
    width: calc(100vw - 12px);
  }
}

.skills-titlebar {
  align-items: center;
  backdrop-filter: saturate(180%) blur(24px);
  background: var(--lumina-toolbar-bg);
  border-bottom: 0.5px solid var(--lumina-separator);
  display: flex;
  justify-content: space-between;
  padding: 7px 12px 7px 14px;
  user-select: none;
}

.title-identity {
  align-items: center;
  display: flex;
  gap: 10px;

  h2 {
    font-size: 14px;
    font-weight: 600;
    margin: 0;
  }

  p {
    color: var(--lumina-text-secondary);
    font-size: 11.5px;
    margin: 1px 0 0;
  }
}

.app-icon {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-primary) 12%, var(--lumina-surface-2));
  border-radius: 8px;
  color: var(--lumina-primary);
  display: flex;
  height: 32px;
  justify-content: center;
  width: 32px;

  svg {
    height: 17px;
    width: 17px;
  }
}

.window-actions {
  align-items: center;
  display: flex;
  gap: 2px;
}

.maximize-button {
  align-items: center;
  background: transparent;
  border: 0;
  border-radius: 6px;
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: flex;
  height: 28px;
  justify-content: center;
  padding: 0;
  width: 28px;

  svg {
    height: 15px;
    width: 15px;
  }

  &:hover {
    background: var(--lumina-control-hover);
    color: var(--lumina-text);
  }

  &:focus-visible {
    box-shadow: 0 0 0 3px var(--lumina-accent-ring);
    outline: none;
  }
}

.skills-workspace {
  background: var(--lumina-content-bg);
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
}

.skills-subnav {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-surface-elevated) 88%, transparent);
  border-bottom: 0.5px solid var(--lumina-separator);
  display: flex;
  flex-shrink: 0;
  height: 48px;
  justify-content: space-between;
  padding: 0 16px;
  user-select: none;
}

.workspace-tabs {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-fill-quaternary) 70%, transparent);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-md, 8px);
  display: flex;
  gap: 3px;
  padding: 2.5px;
}

.workspace-tab {
  align-items: center;
  background: transparent;
  border: none;
  border-radius: 6px;
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: inline-flex;
  font-size: 12px;
  font-weight: 500;
  gap: 6px;
  height: 28px;
  padding: 0 12px;
  position: relative;
  transition: all 0.15s ease;
  user-select: none;

  &:hover {
    color: var(--lumina-text);
  }

  &.active {
    background: var(--lumina-surface-elevated);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.12);
    color: var(--lumina-primary);
    font-weight: 600;
  }
}

.tab-counter {
  background: color-mix(in srgb, var(--lumina-fill-tertiary) 70%, transparent);
  border-radius: 9999px;
  font-size: 11.5px;
  font-weight: 500;
  padding: 0.5px 6px;
}

.tab-warn-dot {
  background: var(--lumina-warning, #b76500);
  border-radius: 50%;
  height: 6px;
  width: 6px;
}

.tab-sparkle-pill {
  background: var(--lumina-primary-soft);
  border-radius: 4px;
  color: var(--lumina-primary);
  font-size: 11px;
  font-weight: 700;
  padding: 0.5px 6px;
}

.workspace-actions {
  align-items: center;
  display: flex;
  gap: 8px;
}

.page-action-btn {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-fill-quaternary) 80%, transparent);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-sm, 6px);
  color: var(--lumina-text);
  cursor: pointer;
  display: inline-flex;
  font-size: 11.5px;
  font-weight: 500;
  gap: 5px;
  height: 28px;
  padding: 0 10px;
  transition: all 120ms ease;

  &:hover:not(:disabled) {
    background: color-mix(in srgb, var(--lumina-fill-secondary) 85%, transparent);
  }

  &:disabled {
    cursor: not-allowed;
    opacity: 0.45;
  }

  &--primary {
    background: var(--lumina-primary);
    border-color: transparent;
    color: #ffffff;
    font-weight: 600;

    &:hover:not(:disabled) {
      filter: brightness(1.08);
    }
  }
}

.skills-body-container {
  display: flex;
  flex: 1;
  min-height: 0;
  overflow: hidden;
  position: relative;
}

.highlight-warn {
  color: var(--lumina-warning, #f59e0b);
  font-weight: 600;
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
