<template>
  <WorkbenchTopbar>
    <WorkbenchIdentity :label="t('gitAssistant.repo.currentRepoShort')" :value="currentRecentLabel" :title="repoPath || t('gitAssistant.repo.emptyPath')">
      <button class="repo-switcher-manage" type="button" :title="t('gitAssistant.repo.recentRepoManage')" @click="$emit('manage-repos')">
        <Icon icon="solar:settings-linear" />
      </button>
    </WorkbenchIdentity>

    <WorkbenchTag :label="t('gitAssistant.repo.branchShort')" :value="branch || '--'" />
    <WorkbenchTag :label="t('gitAssistant.repo.summaryTotal')" :value="summary.total" />
    <WorkbenchTag :label="t('gitAssistant.repo.summaryStaged')" :value="summary.staged" />
    <WorkbenchTag :label="t('gitAssistant.repo.summaryUnstaged')" :value="summary.unstaged" />
    <WorkbenchTag :label="t('gitAssistant.repo.summaryUntracked')" :value="summary.untracked" />
    <WorkbenchTag v-if="summary.conflicted" :label="t('gitAssistant.repo.summaryConflicted')" :value="summary.conflicted" tone="danger" />
    <WorkbenchTag :label="t('gitAssistant.repo.summaryRecommended')" :value="recommendedCount" tone="primary" />

    <template #actions>
      <NDropdown trigger="click" :options="viewOptions" @select="handleViewAction">
        <WorkbenchButton>
          <Icon icon="solar:sidebar-minimalistic-linear" />
          {{ t('gitAssistant.layout.view') }}
        </WorkbenchButton>
      </NDropdown>
      <span class="sync-pill" :class="syncTone">
        <span class="sync-dot"></span>
        {{ syncLabel }}
      </span>
      <div v-if="hasSnapshot" class="action-group">
        <NDropdown trigger="click" :options="syncOptions" @select="action => $emit('sync-action', String(action))">
          <WorkbenchButton :disabled="syncDisabled">{{ t('gitAssistant.repo.syncActions') }}</WorkbenchButton>
        </NDropdown>
      </div>
      <div class="action-group action-group-project">
        <NDropdown trigger="click" :options="projectOptions" @select="handleProjectAction">
          <WorkbenchButton>
            <Icon icon="solar:folder-with-files-linear" />
            {{ t('gitAssistant.repo.currentRepoShort') }}
          </WorkbenchButton>
        </NDropdown>
        <WorkbenchButton v-if="hasSnapshot" variant="primary" :disabled="loading || !repoPath" @click="$emit('refresh')">
          {{ t('gitAssistant.repo.refreshRepo') }}
        </WorkbenchButton>
      </div>
    </template>
  </WorkbenchTopbar>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted } from 'vue'
import { NDropdown, type DropdownOption } from 'naive-ui'
import { Icon } from '@iconify/vue'
import { useLocale } from '@/hooks/useLocale'
import WorkbenchButton from '@/components/workbench/WorkbenchButton.vue'
import WorkbenchIdentity from '@/components/workbench/WorkbenchIdentity.vue'
import WorkbenchTag from '@/components/workbench/WorkbenchTag.vue'
import WorkbenchTopbar from '@/components/workbench/WorkbenchTopbar.vue'
import type { GitRepositoryState } from '@/services/git/git-service'
import type { GitAssistantSummary } from '../git-assistant.types'
import { hasPrimaryModifier } from '@/utils/platform-shortcuts'

const props = defineProps<{
  repoPath: string
  branch: string
  loading: boolean
  fetching: boolean
  pushing: boolean
  pulling: boolean
  summary: GitAssistantSummary
  recommendedCount: number
  repositoryState: GitRepositoryState | null
  recentRepos: RecentGitRepo[]
  hasSnapshot: boolean
  panelVisibility: {
    changes: boolean
    diff: boolean
    commit: boolean
  }
}>()

const emit = defineEmits<{
  (e: 'pick-directory'): void
  (e: 'refresh'): void
  (e: 'sync-action', value: string): void
  (e: 'manage-repos'): void
  (e: 'open-branch-selector'): void
  (e: 'open-merge'): void
  (e: 'clone-repository'): void
  (e: 'init-repository'): void
  (e: 'open-repository-rules'): void
  (e: 'toggle-panel', panel: 'changes' | 'diff' | 'commit'): void
  (e: 'reset-layout'): void
}>()

const { t } = useLocale()

const repoName = computed(() => {
  const normalized = props.repoPath.replace(/\\/g, '/')
  const parts = normalized.split('/').filter(Boolean)
  return parts[parts.length - 1] ?? ''
})

const currentRecentLabel = computed(() => {
  const current = props.recentRepos.find(repo => normalizePath(repo.path) === normalizePath(props.repoPath))
  return current?.name || repoName.value || t('gitAssistant.repo.recentRepoPlaceholder')
})

const syncLabel = computed(() => {
  const state = props.repositoryState
  if (!state?.remoteName) return t('gitAssistant.sync.remoteMissing')
  if (!state.hasCommits) return t('gitAssistant.sync.firstCommit')
  if (state.upstreamGone) return t('gitAssistant.sync.upstreamGone')
  if (!state.upstream) return t('gitAssistant.sync.notSet')
  if (state.ahead > 0 && state.behind > 0) return t('gitAssistant.sync.diverged', { ahead: state.ahead, behind: state.behind })
  if (state.ahead > 0) return t('gitAssistant.sync.ahead', { count: state.ahead })
  if (state.behind > 0) return t('gitAssistant.sync.behind', { count: state.behind })
  return t('gitAssistant.sync.syncedShort')
})

const syncTone = computed(() => {
  const state = props.repositoryState
  if (!state?.remoteName || !state.hasCommits || state.upstreamGone || !state.upstream) return 'warning'
  if (state.behind > 0) return 'danger'
  if (state.ahead > 0) return 'accent'
  return 'ready'
})

const pushDisabled = computed(() =>
  props.pushing || props.pulling || props.fetching || props.loading || !props.repositoryState?.hasCommits || !props.repositoryState.remoteName,
)
const pullDisabled = computed(() =>
  props.pulling || props.pushing || props.fetching || props.loading || !props.repositoryState?.upstream || props.repositoryState.upstreamGone,
)
const fetchDisabled = computed(() =>
  props.fetching || props.pulling || props.pushing || props.loading || !props.repositoryState?.remoteName,
)
const syncDisabled = computed(() => props.fetching || props.pulling || props.pushing || props.loading || !props.repositoryState?.remoteName)
const syncOptions = computed(() => [
  { label: props.pulling ? t('gitAssistant.ai.pulling') : t('gitAssistant.ai.pull'), key: 'pull', disabled: pullDisabled.value },
  { label: props.fetching ? t('gitAssistant.ai.fetching') : t('gitAssistant.ai.fetch'), key: 'fetch', disabled: fetchDisabled.value },
  { label: props.pushing ? t('gitAssistant.ai.pushing') : t('gitAssistant.ai.push'), key: 'push', disabled: pushDisabled.value },
])

const visiblePanelCount = computed(() => Object.values(props.panelVisibility).filter(Boolean).length)
const viewOptions = computed<DropdownOption[]>(() => [
  {
    label: `${props.panelVisibility.changes ? '✓  ' : ''}${t('gitAssistant.layout.changes')}`,
    key: 'changes',
    disabled: props.panelVisibility.changes && visiblePanelCount.value === 1,
  },
  {
    label: `${props.panelVisibility.diff ? '✓  ' : ''}${t('gitAssistant.layout.diff')}`,
    key: 'diff',
    disabled: props.panelVisibility.diff && visiblePanelCount.value === 1,
  },
  {
    label: `${props.panelVisibility.commit ? '✓  ' : ''}${t('gitAssistant.layout.commit')}`,
    key: 'commit',
    disabled: props.panelVisibility.commit && visiblePanelCount.value === 1,
  },
  { type: 'divider', key: 'layout-divider' },
  { label: t('gitAssistant.layout.reset'), key: 'reset' },
])
const projectOptions = computed<DropdownOption[]>(() => props.hasSnapshot ? [
  { label: t('gitAssistant.repo.manageBranches'), key: 'branches' },
  { label: t('gitAssistant.repo.mergeBranch'), key: 'merge' },
  { label: t('gitAssistant.repositoryRules.title'), key: 'repository-rules' },
  { type: 'divider', key: 'repo-divider' },
  { label: t('gitAssistant.repo.chooseDirectory'), key: 'choose' },
] : [
  { label: t('gitAssistant.repo.cloneRepository'), key: 'clone' },
  { label: t('gitAssistant.repo.initRepository'), key: 'init' },
  { type: 'divider', key: 'repo-divider' },
  { label: t('gitAssistant.repo.chooseDirectory'), key: 'choose' },
])

function handleViewAction(key: string | number) {
  if (key === 'reset') {
    emit('reset-layout')
    return
  }
  if (key === 'changes' || key === 'diff' || key === 'commit') {
    emit('toggle-panel', key)
  }
}

function handleProjectAction(key: string | number) {
  if (key === 'branches') emit('open-branch-selector')
  else if (key === 'merge') emit('open-merge')
  else if (key === 'repository-rules') emit('open-repository-rules')
  else if (key === 'clone') emit('clone-repository')
  else if (key === 'init') emit('init-repository')
  else if (key === 'choose') emit('pick-directory')
}

function handleRefreshShortcut(event: KeyboardEvent) {
  if (!hasPrimaryModifier(event) || event.key.toLowerCase() !== 'r') return
  if (!props.hasSnapshot || props.loading || !props.repoPath) return
  event.preventDefault()
  emit('refresh')
}

onMounted(() => window.addEventListener('keydown', handleRefreshShortcut))
onUnmounted(() => window.removeEventListener('keydown', handleRefreshShortcut))

export interface RecentGitRepo {
  path: string
  name: string
}

function normalizePath(path: string) {
  return path.replace(/\\/g, '/').toLowerCase()
}

</script>

<style scoped lang="scss">
.action-group {
  align-items: center;
  display: inline-flex;
  gap: 6px;
}

.action-group-project {
  border-left: 0.5px solid var(--lumina-separator);
  margin-left: 2px;
  padding-left: 8px;
}

.sync-pill {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-surface-3) 82%, transparent);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-sm);
  color: var(--lumina-text-secondary);
  display: inline-flex;
  font-size: 11px;
  gap: 7px;
  height: 28px;
  padding: 0 10px;
  white-space: nowrap;

  &.ready {
    color: var(--lumina-primary);
  }

  &.accent {
    background: color-mix(in srgb, var(--lumina-primary-soft) 60%, var(--lumina-surface-2));
    color: var(--lumina-primary);
  }

  &.warning {
    color: var(--lumina-warning);
  }

  &.danger {
    color: var(--lumina-danger);
  }
}

.sync-dot {
  background: currentcolor;
  border-radius: 999px;
  height: 6px;
  width: 6px;
}

.repo-switcher-manage {
  align-items: center;
  background: transparent;
  border: 0;
  border-radius: 6px;
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: inline-flex;
  flex: 0 0 auto;
  height: 22px;
  justify-content: center;
  padding: 0;
  transition:
    background 0.18s ease,
    color 0.18s ease;
  width: 22px;

  svg {
    height: 15px;
    width: 15px;
  }

  &:hover {
    background: color-mix(in srgb, var(--lumina-button-secondary-hover) 82%, transparent);
    color: var(--lumina-text);
  }
}
</style>
