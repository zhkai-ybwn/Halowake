<template>
  <div class="install-history-view">
    <div v-if="history.length > 0" class="history-toolbar">
      <div>
        <h3>{{ t('skills.review.history') }} <span>{{ history.length }}</span></h3>
        <p>{{ t('skills.review.historySubtitle') }}</p>
      </div>
      <div class="history-filters">
        <label class="history-search">
          <Icon icon="solar:magnifer-linear" />
          <input v-model="query" :aria-label="t('skills.review.historySearchAria')" :placeholder="t('skills.review.historySearch')" />
        </label>
        <select v-model="statusFilter" :aria-label="t('skills.review.historyFilterAria')">
          <option value="all">{{ t('skills.review.allResults') }}</option>
          <option value="success">{{ t('skills.review.statusSuccess') }}</option>
          <option value="skipped">{{ t('skills.review.statusSkipped') }}</option>
          <option value="partial">{{ t('skills.review.statusPartial') }}</option>
          <option value="failed">{{ t('skills.review.statusFailed') }}</option>
        </select>
      </div>
    </div>

    <div class="history-scroll">
      <div v-if="loading" class="history-state">
        <span class="history-state__icon"><Icon icon="solar:refresh-linear" class="spin-glyph" /></span>
        <strong>{{ t('skills.review.historyLoading') }}</strong>
        <p>{{ t('skills.review.pleaseWait') }}</p>
      </div>
      <div v-else-if="error" class="history-state history-state--error" role="alert">
        <span class="history-state__icon"><Icon icon="solar:danger-circle-linear" /></span>
        <strong>{{ t('skills.review.historyError') }}</strong>
        <p>{{ t('skills.review.historyErrorHint') }}</p>
        <button type="button" class="history-retry" @click="loadHistory">
          <Icon icon="solar:refresh-linear" />
          {{ t('skills.review.retry') }}
        </button>
        <details class="history-error-detail">
          <summary>{{ t('skills.review.errorDetails') }}</summary>
          <pre>{{ error }}</pre>
        </details>
      </div>
      <div v-else-if="history.length === 0" class="history-state history-state--empty">
        <span class="history-state__icon"><Icon icon="solar:box-minimalistic-linear" /></span>
        <strong>{{ t('skills.review.historyEmpty') }}</strong>
        <p>{{ t('skills.review.historyEmptyHint') }}</p>
        <div class="empty-action-group">
          <button
            type="button"
            class="empty-cta-btn empty-cta-btn--primary"
            @click="exploreStore.activeMainTab = 'explore'"
          >
            <Icon icon="solar:planet-2-linear" />
            <span>{{ t('skills.review.goToExplore') }}</span>
          </button>
          <button
            v-if="skillsStore.skills.length > 0"
            type="button"
            class="empty-cta-btn empty-cta-btn--secondary"
            @click="exploreStore.activeMainTab = 'installed'"
          >
            <Icon icon="solar:folder-with-files-linear" />
            <span>{{ t('skills.review.goToInstalledBtn', { count: skillsStore.skills.length }) }}</span>
          </button>
        </div>
      </div>
      <div v-else-if="filteredHistory.length === 0" class="history-state">
        <span class="history-state__icon"><Icon icon="solar:magnifer-linear" /></span>
        <strong>{{ t('skills.review.historyNoMatch') }}</strong>
        <p>{{ t('skills.review.historyNoMatchHint') }}</p>
      </div>
      <div v-else class="history-list">
        <article v-for="entry in filteredHistory" :key="entry.id" class="history-card">
          <div class="history-main">
            <div class="history-name-row">
              <strong>{{ entry.displayName }}</strong>
              <span class="history-status" :class="`history-status--${entry.status}`">{{ statusLabel(entry.status) }}</span>
              <time :datetime="new Date(entry.createdAt).toISOString()">{{ formatTime(entry.createdAt) }}</time>
              <button v-if="entry.existingPaths.length" type="button" class="history-manage" @click="goToInstalled(entry)">{{ t('skills.review.viewInstalled') }}</button>
              <button
                v-if="entry.status === 'failed' || entry.status === 'partial'"
                type="button"
                class="history-manage"
                @click="reinstallEntry(entry)"
              >
                {{ t('skills.review.retry', '重新安装') }}
              </button>
            </div>
            <p class="history-target">{{ targetLabel(entry) }}</p>
            <p class="history-source" :title="sourceLabel(entry)">{{ t('skills.review.source', { source: sourceLabel(entry) }) }}</p>
            <p v-if="entry.status !== 'success'" class="history-message" :class="{ 'history-message--error': entry.status === 'failed' || entry.status === 'partial' }">{{ entry.message }}</p>
          </div>
          <div v-if="entry.installedPaths.length" class="history-paths">
            <div v-for="path in entry.installedPaths" :key="path" class="history-path">
              <Icon icon="solar:folder-linear" />
              <span :title="path">{{ path }}</span>
              <small v-if="!entry.existingPaths.includes(path)">{{ t('skills.review.folderRemoved') }}</small>
              <button v-else type="button" :aria-label="t('skills.review.openPath', { path })" @click="openPath(path)">{{ t('skills.review.open') }}</button>
            </div>
          </div>
        </article>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { Icon } from '@iconify/vue'
import { useMessage } from 'naive-ui'
import { useI18n } from 'vue-i18n'
import { useSkillExploreStore } from '@/stores/skill-explore'
import { useSkillsStore } from '@/stores/skills'
import { listSkillInstallHistory, openSkillFolder } from '@/services/skills/skills-service'
import type { SkillInstallHistoryEntry } from '@/types/skills'

const exploreStore = useSkillExploreStore()
const { locale, t } = useI18n()
const skillsStore = useSkillsStore()
const message = useMessage()
const history = ref<SkillInstallHistoryEntry[]>([])
const loading = ref(false)
const error = ref('')
const query = ref('')
const statusFilter = ref<'all' | SkillInstallHistoryEntry['status']>('all')

const filteredHistory = computed(() => {
  const term = query.value.trim().toLowerCase()
  return history.value.filter(entry => {
    if (statusFilter.value !== 'all' && entry.status !== statusFilter.value) return false
    if (!term) return true
    return [entry.skillId, entry.displayName, entry.subSkillId, entry.gitUrl, entry.subPath,
      entry.projectCwd, ...entry.targetAgents, ...entry.installedPaths]
      .some(value => value?.toLowerCase().includes(term))
  })
})

async function loadHistory() {
  loading.value = true
  error.value = ''
  try {
    history.value = await listSkillInstallHistory()
  } catch (cause) {
    error.value = t('skills.review.historyReadFailed', { error: cause instanceof Error ? cause.message : String(cause) })
  } finally {
    loading.value = false
  }
}

watch(() => exploreStore.activeMainTab, tab => {
  if (tab === 'history') void loadHistory()
}, { immediate: true })

watch(() => exploreStore.installModalVisible, (visible, prevVisible) => {
  if (!visible && prevVisible && exploreStore.activeMainTab === 'history') {
    void loadHistory()
  }
})

function statusLabel(status: SkillInstallHistoryEntry['status']) {
  return t(`skills.review.${status === 'success' ? 'statusSuccess' : status === 'skipped' ? 'statusSkipped' : status === 'partial' ? 'statusPartial' : 'statusFailed'}`)
}

function targetLabel(entry: SkillInstallHistoryEntry) {
  if (entry.targetMode === 'project') return t('skills.review.targetProject', { path: entry.projectCwd || t('skills.review.unknownPath') })
  if (entry.targetMode === 'global') return t('skills.review.targetGlobal')
  const labels: Record<string, string> = { codex: 'Codex', antigravity: 'Antigravity', claude: 'Claude Code', cursor: 'Cursor', trae: 'Trae' }
  return entry.targetAgents.map(agent => labels[agent] || agent).join(', ') || t('skills.review.targetAgents')
}

function sourceLabel(entry: SkillInstallHistoryEntry) {
  return `${entry.gitUrl}${entry.subPath ? `/${entry.subPath}` : ''}`
}

function formatTime(timestamp: number) {
  return new Date(timestamp).toLocaleString(locale.value, { year: 'numeric', month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' })
}

async function openPath(path: string) {
  try {
    await openSkillFolder(path)
  } catch (cause) {
    message.error(t('skills.review.openFailed', { error: cause instanceof Error ? cause.message : String(cause) }))
  }
}

function goToInstalled(entry: SkillInstallHistoryEntry) {
  skillsStore.revealInstalledSkills(entry.displayName)
  exploreStore.activeMainTab = 'installed'
}

function reinstallEntry(entry: SkillInstallHistoryEntry) {
  exploreStore.openReinstallFromHistory(entry)
}
</script>

<style scoped lang="scss">
.install-history-view { display: flex; flex: 1; flex-direction: column; min-height: 0; }
.history-toolbar {
  align-items: center;
  border-bottom: 0.5px solid var(--lumina-separator);
  display: flex;
  flex-shrink: 0;
  flex-wrap: wrap;
  gap: 12px;
  justify-content: space-between;
  padding: 14px 18px;

  h3 { color: var(--lumina-text); font-size: 13px; margin: 0; }
  h3 span { color: var(--lumina-text-tertiary); font-size: 11px; font-weight: 500; }
  p { color: var(--lumina-text-secondary); font-size: 11px; margin: 3px 0 0; }
}
.history-filters { align-items: center; display: flex; flex-wrap: wrap; gap: 8px; }
.history-search {
  align-items: center;
  background: var(--lumina-surface-elevated);
  border: 0.5px solid var(--lumina-separator);
  border-radius: 7px;
  color: var(--lumina-text-tertiary);
  display: flex;
  gap: 6px;
  padding: 0 8px;

  input { background: transparent; border: 0; color: var(--lumina-text); font-size: 11px; height: 30px; outline: none; width: 190px; }
}
.history-filters select {
  background: var(--lumina-surface-elevated);
  border: 0.5px solid var(--lumina-separator);
  border-radius: 7px;
  color: var(--lumina-text);
  font-size: 11px;
  height: 31px;
  padding: 0 8px;
}
.history-scroll { flex: 1; min-height: 0; overflow-y: auto; padding: 16px 18px; }
.history-list { display: grid; gap: 10px; }
.history-card { background: var(--lumina-surface-elevated); border: 0.5px solid var(--lumina-separator); border-radius: 9px; padding: 12px 14px; }
.history-name-row { align-items: center; display: flex; flex-wrap: wrap; gap: 8px; }
.history-name-row strong { color: var(--lumina-text); font-size: 12px; }
.history-name-row time { color: var(--lumina-text-tertiary); font-size: 11px; margin-left: auto; }
.history-manage { background: transparent; border: 0; color: var(--lumina-primary); cursor: pointer; font-size: 11px; padding: 2px 4px; }
.history-status { border-radius: 4px; font-size: 10px; padding: 2px 6px; }
.history-status--success { background: var(--lumina-primary-soft); color: var(--lumina-primary); }
.history-status--skipped { background: var(--lumina-fill-quaternary); color: var(--lumina-text-secondary); }
.history-status--partial { background: color-mix(in srgb, var(--lumina-warning) 14%, transparent); color: var(--lumina-warning); }
.history-status--failed { background: color-mix(in srgb, var(--lumina-danger) 12%, transparent); color: var(--lumina-danger); }
.history-target, .history-source, .history-message { color: var(--lumina-text-secondary); font-size: 11px; margin: 6px 0 0; overflow-wrap: anywhere; }
.history-source { color: var(--lumina-text-tertiary); }
.history-message--error { color: var(--lumina-danger); }
.history-paths { border-top: 0.5px solid var(--lumina-separator); display: grid; gap: 6px; margin-top: 10px; padding-top: 10px; }
.history-path { align-items: center; color: var(--lumina-text-secondary); display: flex; font-size: 11px; gap: 7px; min-width: 0; }
.history-path span { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.history-path small { color: var(--lumina-text-tertiary); flex-shrink: 0; }
.history-path button { background: transparent; border: 0; color: var(--lumina-primary); cursor: pointer; font-size: 11px; padding: 2px 4px; }
.history-state {
  align-items: center;
  display: flex;
  flex-direction: column;
  margin: min(12vh, 88px) auto 0;
  max-width: min(100%, 480px);
  padding: 24px;
  text-align: center;
  width: 100%;

  &__icon {
    align-items: center;
    background: var(--lumina-fill-quaternary);
    border-radius: 12px;
    color: var(--lumina-text-secondary);
    display: inline-flex;
    font-size: 23px;
    height: 48px;
    justify-content: center;
    margin-bottom: 15px;
    width: 48px;
  }

  strong { color: var(--lumina-text); font-size: 14px; font-weight: 600; }
  p { color: var(--lumina-text-secondary); font-size: 12px; line-height: 1.6; margin: 7px 0 0; }

  &--error .history-state__icon {
    background: color-mix(in srgb, var(--lumina-danger) 9%, var(--lumina-surface-elevated));
    color: var(--lumina-danger);
  }

  &--empty .history-state__icon {
    background: color-mix(in srgb, var(--lumina-primary) 12%, transparent);
    color: var(--lumina-primary);
  }
}

.empty-action-group {
  align-items: center;
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  justify-content: center;
  margin-top: 20px;
}

.empty-cta-btn {
  align-items: center;
  border-radius: 6px;
  cursor: pointer;
  display: inline-flex;
  font-size: 11.5px;
  font-weight: 550;
  gap: 6px;
  height: 30px;
  padding: 0 14px;
  transition: all 140ms ease;

  &--primary {
    background: var(--lumina-primary);
    border: none;
    color: #fff;

    &:hover {
      filter: brightness(1.08);
      transform: translateY(-1px);
    }
  }

  &--secondary {
    background: var(--lumina-control-bg);
    border: 0.5px solid var(--lumina-separator);
    color: var(--lumina-text);

    &:hover {
      background: var(--lumina-control-hover);
      color: var(--lumina-primary);
    }
  }
}
.history-retry {
  align-items: center;
  background: var(--lumina-primary);
  border: 0;
  border-radius: 7px;
  color: #fff;
  cursor: pointer;
  display: inline-flex;
  font-size: 12px;
  font-weight: 600;
  gap: 6px;
  height: 30px;
  margin-top: 17px;
  padding: 0 13px;

  &:hover { filter: brightness(1.07); }
}
.history-error-detail {
  color: var(--lumina-text-tertiary);
  font-size: 11px;
  margin-top: 18px;
  max-width: 100%;
  text-align: left;
  width: 100%;

  summary { cursor: pointer; text-align: center; }
  pre {
    background: var(--lumina-surface-elevated);
    border: 0.5px solid var(--lumina-separator);
    border-radius: 7px;
    color: var(--lumina-text-secondary);
    font-family: var(--lumina-font-mono, monospace);
    font-size: 10px;
    line-height: 1.5;
    margin: 9px 0 0;
    max-height: 120px;
    overflow: auto;
    padding: 10px;
    white-space: pre-wrap;
    word-break: break-word;
  }
}
</style>
