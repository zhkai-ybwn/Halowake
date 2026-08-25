<template>
  <section class="change-explorer">
    <header class="change-header">
      <div>
        <span>{{ t('gitAssistant.files.title') }}</span>
        <small>{{ t('gitAssistant.files.refreshHint') }}</small>
      </div>
      <strong>{{ t('gitAssistant.files.selectedTotal', { selected: reviewSelectedRaws.length, total: totalCount }) }}</strong>
    </header>

    <div class="filter-toolbar">
      <label class="file-search">
        <Icon icon="solar:magnifer-linear" />
        <input
          ref="searchInput"
          :value="keyword"
          type="search"
          :placeholder="t('gitAssistant.files.searchPlaceholder')"
          @input="$emit('update:keyword', ($event.target as HTMLInputElement).value.trim())"
        />
      </label>
      <select
        :value="statusFilter"
        :aria-label="t('gitAssistant.files.tableStatus')"
        @change="$emit('update:status-filter', ($event.target as HTMLSelectElement).value)"
      >
        <option v-for="option in statusFilterOptions" :key="option.value" :value="option.value">
          {{ t(option.labelKey) }}
        </option>
      </select>
      <label class="recommended-filter">
        <input
          :checked="recommendedOnly"
          type="checkbox"
          @change="$emit('update:recommended-only', ($event.target as HTMLInputElement).checked)"
        />
        <span>{{ t('gitAssistant.files.recommendedOnly') }}</span>
      </label>
    </div>

    <div class="check-toolbar">
      <span>{{ t('gitAssistant.files.check') }}:</span>
      <button type="button" @click="$emit('set-review-selection', visibleRaws)">
        {{ t('gitAssistant.files.selectAll') }}
      </button>
      <button type="button" @click="$emit('set-review-selection', [])">
        {{ t('gitAssistant.files.selectNone') }}
      </button>
      <button type="button" @click="$emit('set-review-selection', unversionedRaws)">
        {{ t('gitAssistant.files.filters.untracked') }}
      </button>
      <button type="button" @click="$emit('set-review-selection', versionedRaws)">
        {{ t('gitAssistant.files.filters.versioned') }}
      </button>
      <button type="button" @click="$emit('set-review-selection', addedRaws)">
        {{ t('gitAssistant.files.filters.added') }}
      </button>
      <button type="button" @click="$emit('set-review-selection', deletedRaws)">
        {{ t('gitAssistant.files.filters.deleted') }}
      </button>
      <button type="button" @click="$emit('set-review-selection', modifiedRaws)">
        {{ t('gitAssistant.files.filters.modified') }}
      </button>
      <button type="button" @click="$emit('set-review-selection', conflictedRaws)">
        {{ t('gitAssistant.files.filters.conflicted') }}
      </button>
      <button type="button" @click="$emit('set-review-selection', visibleRaws)">
        {{ t('gitAssistant.files.filters.files') }}
      </button>
      <div class="review-score-action">
        <div
          class="review-score-progress"
          :class="{ 'is-hidden': !reviewScoring }"
          :aria-hidden="!reviewScoring"
          role="status"
        >
          <span>{{ reviewProgressText }}</span>
          <i><b :style="{ width: `${reviewProgressPercent}%` }"></b></i>
        </div>
        <span class="review-score-help" tabindex="0" :aria-label="t('gitAssistant.files.scoreHelpLabel')">
          ?
          <span class="review-score-help__popover" role="tooltip">{{ t('gitAssistant.files.scoreNotice') }}</span>
        </span>
        <button
          class="review-score-button"
          :class="{ 'is-running': reviewScoring }"
          type="button"
          :disabled="!hasSnapshot || loading || reviewScoring || !totalCount"
          @click="$emit('request-review-score')"
        >
          {{ reviewScoring
            ? t('gitAssistant.files.scoring')
            : hasReviewScores
              ? t('gitAssistant.files.rescore')
              : t('gitAssistant.files.scoreChanges') }}
        </button>
      </div>
    </div>

    <WorkbenchEmptyState v-if="!hasSnapshot && !loading" icon="solar:folder-open-linear" :title="t('gitAssistant.files.emptyNoRepoTitle')" :description="t('gitAssistant.files.emptyNoRepo')" />
    <WorkbenchEmptyState v-else-if="loading" icon="solar:refresh-circle-linear" :title="t('gitAssistant.files.emptyLoadingTitle')" :description="t('gitAssistant.files.emptyLoading')" />
    <WorkbenchEmptyState v-else-if="!groups.length" icon="solar:check-circle-linear" :title="totalCount ? t('gitAssistant.files.emptyNoMatchTitle') : t('gitAssistant.files.emptyCleanTitle')" :description="totalCount ? t('gitAssistant.files.emptyNoMatch') : t('gitAssistant.files.emptyClean')">
      <template #actions><WorkbenchButton @click="$emit('request-refresh')">{{ t('gitAssistant.repo.refreshRepo') }}</WorkbenchButton></template>
    </WorkbenchEmptyState>

    <div v-else class="file-table">
      <div class="table-header" :style="gridStyle">
        <span class="check-cell header-cell">
          <input
            ref="headerCheckbox"
            :checked="allVisibleSelected"
            type="checkbox"
            @change="$emit('set-review-selection', allVisibleSelected ? [] : visibleRaws)"
          />
        </span>
        <span
          v-for="column in resizableColumns"
          :key="column.key"
          class="header-cell"
          :class="{ 'numeric-header': column.key === 'added' || column.key === 'removed' || column.key === 'score' }"
        >
          {{ t(column.labelKey) }}
          <i class="column-resizer" @mousedown.prevent="startColumnResize(column.key, $event)"></i>
        </span>
      </div>

      <div
        v-for="file in visibleFiles"
        :key="file.raw"
        class="file-row"
        :class="{ active: activeFileRaw === file.raw }"
        :style="gridStyle"
        @contextmenu.prevent="handleFileContextMenu(file.raw, $event)"
        @dblclick="$emit('open-diff', file.raw)"
        @click="$emit('select-file', file.raw)"
      >
        <label class="commit-check" @click.stop>
          <input
            :checked="reviewSelectedRaws.includes(file.raw)"
            type="checkbox"
            @change="
              $emit('toggle-review-selection', {
                raw: file.raw,
                checked: ($event.target as HTMLInputElement).checked,
              })
            "
          />
        </label>

        <div class="path-cell mono" :title="file.path">
          <span v-if="file.recommended" class="attention-dot" :title="t('gitAssistant.files.recommended')"></span>
          {{ file.path }}
        </div>
        <div class="extension-cell mono">{{ file.extension || '-' }}</div>
        <div class="status-cell" :class="`tone-${statusMeta[file.type].tone}`">
          {{ t(statusMeta[file.type].labelKey) }}
        </div>
        <div class="line-cell added-lines">{{ formatLineCount(file.addedLines) }}</div>
        <div class="line-cell removed-lines">{{ formatLineCount(file.removedLines) }}</div>
        <div class="score-cell" :title="attentionBreakdownTitle(file)">{{ file.score ?? '-' }}</div>
        <div class="review-reason-cell">
          <span
            v-for="category in file.scoreCategories.slice(0, 2)"
            :key="category"
            class="review-category"
            :class="`tone-${reviewCategoryTone(category)}`"
          >
            {{ t(`gitAssistant.files.reviewCategories.${category}`) }}
          </span>
          <span v-if="!file.scoreCategories.length">-</span>
        </div>
      </div>
    </div>

    <NDropdown
      trigger="manual"
      placement="bottom-start"
      :show="contextMenu.show"
      :x="contextMenu.x"
      :y="contextMenu.y"
      :options="contextMenuOptions"
      @clickoutside="contextMenu.show = false"
      @select="handleContextMenuSelect"
    />
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref, watchEffect } from 'vue'
import { NDropdown } from 'naive-ui'
import { Icon } from '@iconify/vue'
import { useLocale } from '@/hooks/useLocale'
import { STATUS_FILTER_OPTIONS, STATUS_META } from '../git-assistant.config'
import WorkbenchEmptyState from '@/components/workbench/WorkbenchEmptyState.vue'
import WorkbenchButton from '@/components/workbench/WorkbenchButton.vue'
import { hasPrimaryModifier } from '@/utils/platform-shortcuts'
import type {
  GitAssistantFileGroup,
  GitAssistantFileView,
  GitAssistantStatusFilter,
  GitAssistantSummary,
} from '../git-assistant.types'

const props = defineProps<{
  hasSnapshot: boolean
  loading: boolean
  keyword: string
  statusFilter: GitAssistantStatusFilter
  recommendedOnly: boolean
  summary: GitAssistantSummary
  groups: GitAssistantFileGroup[]
  filteredCount: number
  totalCount: number
  activeFileRaw: string | null
  reviewSelectedRaws: string[]
  reviewScoring: boolean
  hasReviewScores: boolean
  reviewScoreProgress: {
    completed: number
    total: number
    phase: string
    filePath: string
  }
}>()

const emit = defineEmits<{
  (e: 'update:keyword', value: string): void
  (e: 'update:status-filter', value: string): void
  (e: 'update:recommended-only', value: boolean): void
  (e: 'select-file', raw: string): void
  (e: 'open-diff', raw: string): void
  (e: 'request-refresh'): void
  (e: 'file-action', payload: { action: 'open-diff' | 'diff-previous' | 'file-history' | 'open-external' | 'mark-resolved' | 'revert' | 'stage' | 'unstage'; raw: string }): void
  (e: 'toggle-review-selection', payload: { raw: string; checked: boolean }): void
  (e: 'set-review-selection', raws: string[]): void
  (e: 'request-review-score'): void
}>()

const statusMeta = STATUS_META
const statusFilterOptions = STATUS_FILTER_OPTIONS
const { t } = useLocale()
const searchInput = ref<HTMLInputElement | null>(null)
const reviewCategoryTones: Record<string, string> = {
  security: 'danger',
  data: 'warning',
  api: 'info',
  logic: 'warning',
  types: 'info',
  config: 'muted',
  markup: 'info',
  style: 'muted',
  test: 'success',
  i18n: 'muted',
  resource: 'muted',
  dependency: 'muted',
  generated: 'muted',
  docs: 'muted',
}

function reviewCategoryTone(category: string) {
  return reviewCategoryTones[category] ?? 'muted'
}

const visibleFiles = computed(() => props.groups.flatMap(group => group.files))
const visibleRaws = computed(() => visibleFiles.value.map(file => file.raw))
const selectedRawSet = computed(() => new Set(props.reviewSelectedRaws))
const selectedVisibleCount = computed(() => visibleRaws.value.filter(raw => selectedRawSet.value.has(raw)).length)
const allVisibleSelected = computed(() => visibleRaws.value.length > 0 && selectedVisibleCount.value === visibleRaws.value.length)
const reviewProgressPercent = computed(() => {
  if (!props.reviewScoreProgress.total) return 4
  return Math.max(4, Math.round((props.reviewScoreProgress.completed / props.reviewScoreProgress.total) * 100))
})
const reviewProgressText = computed(() => {
  const progress = props.reviewScoreProgress
  if (progress.phase === 'profile') return t('gitAssistant.files.scoreProgressProfile')
  if (progress.phase === 'complete') return t('gitAssistant.files.scoreProgressComplete')
  if (progress.filePath) {
    return t('gitAssistant.files.scoreProgressFile', {
      completed: progress.completed,
      total: progress.total,
      file: progress.filePath.split(/[\\/]/).pop() || progress.filePath,
    })
  }
  return t('gitAssistant.files.scoreProgressPreparing')
})
const partiallyVisibleSelected = computed(() => selectedVisibleCount.value > 0 && !allVisibleSelected.value)
const unversionedRaws = computed(() => visibleFiles.value.filter(file => file.type === 'untracked').map(file => file.raw))
const versionedRaws = computed(() => visibleFiles.value.filter(file => file.type !== 'untracked').map(file => file.raw))
const addedRaws = computed(() => visibleFiles.value.filter(file => file.type === 'added').map(file => file.raw))
const deletedRaws = computed(() => visibleFiles.value.filter(file => file.type === 'deleted').map(file => file.raw))
const modifiedRaws = computed(() => visibleFiles.value.filter(file => file.type === 'modified').map(file => file.raw))
const conflictedRaws = computed(() => visibleFiles.value.filter(file => file.type === 'updated-but-unmerged').map(file => file.raw))
const headerCheckbox = ref<HTMLInputElement | null>(null)
const contextFileRaw = ref('')
const contextMenu = reactive({
  show: false,
  x: 0,
  y: 0,
})
const columnWidths = reactive({
  path: 620,
  extension: 104,
  status: 110,
  added: 112,
  removed: 118,
  score: 70,
  reason: 340,
})

const resizableColumns = [
  { key: 'path', labelKey: 'gitAssistant.files.tablePath' },
  { key: 'extension', labelKey: 'gitAssistant.files.tableExtension' },
  { key: 'status', labelKey: 'gitAssistant.files.tableStatus' },
  { key: 'added', labelKey: 'gitAssistant.files.tableAdded' },
  { key: 'removed', labelKey: 'gitAssistant.files.tableRemoved' },
  { key: 'score', labelKey: 'gitAssistant.files.tableScore' },
  { key: 'reason', labelKey: 'gitAssistant.files.tableReviewReason' },
] as const

type ResizableColumnKey = keyof typeof columnWidths

const gridStyle = computed(() => ({
  gridTemplateColumns: `34px ${columnWidths.path}px ${columnWidths.extension}px ${columnWidths.status}px ${columnWidths.added}px ${columnWidths.removed}px ${columnWidths.score}px ${columnWidths.reason}px`,
}))
const contextFile = computed(() => visibleFiles.value.find(file => file.raw === contextFileRaw.value) ?? null)
const contextMenuOptions = computed(() => [
  {
    label: t('gitAssistant.files.menu.stage'),
    key: 'stage',
    disabled: !contextFile.value?.unstaged,
  },
  {
    label: t('gitAssistant.files.menu.unstage'),
    key: 'unstage',
    disabled: !contextFile.value?.staged,
  },
  {
    type: 'divider',
    key: 'divider-stage',
  },
  {
    label: t('gitAssistant.files.menu.openDiff'),
    key: 'open-diff',
  },
  {
    label: t('gitAssistant.files.menu.diffPrevious'),
    key: 'diff-previous',
    disabled: contextFile.value?.type === 'untracked',
  },
  {
    label: t('gitAssistant.files.menu.fileHistory'),
    key: 'file-history',
  },
  {
    label: t('gitAssistant.files.menu.openExternal'),
    key: 'open-external',
    disabled: contextFile.value?.type === 'deleted',
  },
  {
    type: 'divider',
    key: 'divider-danger',
  },
  {
    label: t('gitAssistant.files.menu.revert'),
    key: 'revert',
  },
  {
    type: 'divider',
    key: 'divider-conflict',
  },
  {
    label: t('gitAssistant.files.menu.markResolved'),
    key: 'mark-resolved',
    disabled: contextFile.value?.type !== 'updated-but-unmerged',
  },
  {
    type: 'divider',
    key: 'divider-path',
  },
  {
    label: t('gitAssistant.files.menu.copyPath'),
    key: 'copy-path',
  },
])

watchEffect(() => {
  if (headerCheckbox.value) {
    headerCheckbox.value.indeterminate = partiallyVisibleSelected.value
  }
})

let resizingColumn: ResizableColumnKey | null = null
let resizeStartX = 0
let resizeStartWidth = 0

function startColumnResize(column: ResizableColumnKey, event: MouseEvent) {
  resizingColumn = column
  resizeStartX = event.clientX
  resizeStartWidth = columnWidths[column]
  window.addEventListener('mousemove', handleColumnResize)
  window.addEventListener('mouseup', stopColumnResize)
}

function handleColumnResize(event: MouseEvent) {
  if (!resizingColumn) return
  const nextWidth = resizeStartWidth + event.clientX - resizeStartX
  columnWidths[resizingColumn] = Math.max(0, nextWidth)
}

function stopColumnResize() {
  resizingColumn = null
  window.removeEventListener('mousemove', handleColumnResize)
  window.removeEventListener('mouseup', stopColumnResize)
}

function formatLineCount(value: number | null) {
  return value === null ? '-' : String(value)
}

function attentionBreakdownTitle(file: GitAssistantFileView) {
  if (file.score === null) return ''
  const details = file.scoreBreakdown
    .filter(item => item.delta !== 0)
    .map(item => `${item.factor}: ${item.delta > 0 ? '+' : ''}${item.delta} (${item.evidence})`)
  return [`关注度 ${file.score}`, ...details].join('\n')
}

function handleFileContextMenu(raw: string, event: MouseEvent) {
  contextFileRaw.value = raw
  contextMenu.x = event.clientX
  contextMenu.y = event.clientY
  contextMenu.show = true
  emit('select-file', raw)
}

function handleContextMenuSelect(key: string | number) {
  const file = contextFile.value
  contextMenu.show = false
  if (!file) return

  if (key === 'copy-path') {
    void navigator.clipboard?.writeText(file.path)
    return
  }

  if (key === 'open-diff' || key === 'diff-previous' || key === 'file-history' || key === 'open-external' || key === 'mark-resolved' || key === 'revert' || key === 'stage' || key === 'unstage') {
    emit('file-action', { action: key, raw: file.raw })
  }
}

function handleFindShortcut(event: KeyboardEvent) {
  if (!hasPrimaryModifier(event) || event.key.toLowerCase() !== 'f') return
  if (!searchInput.value?.offsetParent) return
  event.preventDefault()
  searchInput.value.focus()
  searchInput.value.select()
}

onMounted(() => window.addEventListener('keydown', handleFindShortcut))
onUnmounted(() => {
  stopColumnResize()
  window.removeEventListener('keydown', handleFindShortcut)
})
</script>

<style scoped lang="scss">
.change-explorer {
  background: var(--lumina-surface-1);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-lg);
  box-shadow: 0 0 0 0.5px color-mix(in srgb, var(--lumina-text) 4%, transparent), 0 1px 2px rgb(0 0 0 / 5%);
  display: grid;
  grid-template-rows: auto auto auto minmax(0, 1fr);
  min-height: 0;
}

.change-header {
  align-items: center;
  border-bottom: 0.5px solid var(--lumina-separator);
  display: flex;
  justify-content: space-between;
  min-height: 38px;
  padding: 0 12px;

  div {
    align-items: center;
    display: flex;
    gap: 8px;
    min-width: 0;
  }

  span {
    font-size: 13px;
    font-weight: 650;
  }

  small,
  strong {
    color: var(--lumina-text-secondary);
    font-size: 11px;
    font-weight: 500;
  }
}

.filter-toolbar {
  align-items: center;
  border-bottom: 0.5px solid var(--lumina-separator);
  display: grid;
  gap: 6px;
  grid-template-columns: minmax(120px, 1fr) 104px auto;
  min-height: 38px;
  padding: 5px 8px;

  select {
    background: var(--lumina-input-bg);
    border: 0.5px solid var(--lumina-input-border);
    border-radius: var(--lumina-radius-sm);
    color: var(--lumina-text);
    height: 28px;
    min-width: 0;
    padding: 0 6px;
  }
}

.file-search {
  align-items: center;
  background: var(--lumina-input-bg);
  border: 0.5px solid var(--lumina-input-border);
  border-radius: var(--lumina-radius-sm);
  display: flex;
  gap: 6px;
  height: 28px;
  min-width: 0;
  padding: 0 7px;

  &:focus-within {
    border-color: var(--lumina-primary);
    box-shadow: 0 0 0 2px var(--lumina-accent-ring);
  }

  svg {
    color: var(--lumina-text-tertiary);
    flex: 0 0 auto;
    height: 14px;
    width: 14px;
  }

  input {
    background: transparent;
    border: 0;
    color: var(--lumina-text);
    font: inherit;
    min-width: 0;
    outline: 0;
    width: 100%;
  }
}

.recommended-filter {
  align-items: center;
  color: var(--lumina-text-secondary);
  display: flex;
  font-size: 11px;
  gap: 5px;
  white-space: nowrap;
}

.check-toolbar {
  align-items: center;
  background: var(--lumina-surface-2);
  border-bottom: 0.5px solid var(--lumina-separator);
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  min-height: 36px;
  padding: 5px 10px;

  span {
    color: var(--lumina-text-secondary);
    font-size: 12px;
  }

  button {
    background: transparent;
    border: 1px solid transparent;
    border-radius: 6px;
    color: var(--lumina-text);
    cursor: pointer;
    font-size: 12px;
    font-weight: 600;
    min-height: 24px;
    padding: 0 7px;

    &:hover {
      background: color-mix(in srgb, var(--lumina-surface-3) 72%, transparent);
      border-color: var(--lumina-card-border);
      color: var(--lumina-text);
    }

    &:disabled {
      color: var(--lumina-text-secondary);
      cursor: not-allowed;
      opacity: 0.55;
    }
  }
}

.check-toolbar__separator {
  background: var(--lumina-card-border);
  height: 16px;
  margin: 0 2px;
  width: 1px;
}

.review-score-action {
  align-items: center;
  display: flex;
  gap: 10px;
  margin-left: auto;
  min-width: 0;
}

.review-score-progress {
  align-items: center;
  display: flex;
  gap: 7px;
  width: 248px;

  &.is-hidden {
    visibility: hidden;
  }

  span {
    flex: 1;
    font-size: 10px;
    overflow: hidden;
    text-align: left;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  i {
    background: color-mix(in srgb, var(--lumina-card-border) 72%, transparent);
    border-radius: 999px;
    display: block;
    height: 3px;
    overflow: hidden;
    width: 92px;
  }

  b {
    background: var(--lumina-primary);
    border-radius: inherit;
    display: block;
    height: 100%;
    transition: width 160ms ease;
  }
}

.review-score-help {
  align-items: center;
  border: 1px solid var(--lumina-card-border);
  border-radius: 50%;
  color: var(--lumina-text-secondary);
  cursor: help;
  display: inline-flex;
  flex: 0 0 auto;
  font-size: 11px !important;
  font-weight: 700;
  height: 18px;
  justify-content: center;
  position: relative;
  width: 18px;

  &:hover,
  &:focus-visible {
    border-color: var(--lumina-primary);
    color: var(--lumina-primary);
    outline: none;
  }
}

.review-score-help__popover {
  background: var(--lumina-surface-1);
  border: 1px solid var(--lumina-card-border);
  border-radius: 8px;
  bottom: calc(100% + 8px);
  box-shadow: var(--lumina-shadow-md);
  color: var(--lumina-text) !important;
  font-size: 11px !important;
  font-weight: 500;
  line-height: 1.55;
  opacity: 0;
  padding: 9px 11px;
  pointer-events: none;
  position: absolute;
  right: -44px;
  text-align: left;
  transform: translateY(3px);
  transition: opacity 120ms ease, transform 120ms ease;
  visibility: hidden;
  white-space: normal;
  width: 300px;
  z-index: 8;
}

.review-score-help:hover .review-score-help__popover,
.review-score-help:focus-visible .review-score-help__popover {
  opacity: 1;
  transform: translateY(0);
  visibility: visible;
}

.check-toolbar .review-score-button {
  background: var(--lumina-primary);
  border-color: var(--lumina-primary);
  color: var(--lumina-on-accent);
  flex: 0 0 auto;
  padding: 0 11px;

  &:hover:not(:disabled) {
    background: var(--lumina-primary-hover, var(--lumina-primary));
    border-color: var(--lumina-primary-hover, var(--lumina-primary));
    color: var(--lumina-on-accent);
  }

  &.is-running:disabled {
    background: color-mix(in srgb, var(--lumina-primary) 78%, var(--lumina-surface-1));
    border-color: color-mix(in srgb, var(--lumina-primary) 78%, var(--lumina-surface-1));
    color: var(--lumina-on-accent);
    cursor: wait;
    opacity: 1;
  }
}

.panel-empty {
  align-items: center;
  background: var(--lumina-empty-bg);
  border: 1px dashed var(--lumina-empty-border);
  border-radius: 8px;
  color: var(--lumina-text-secondary);
  display: grid;
  font-size: 12px;
  gap: 8px;
  justify-content: center;
  margin: 10px;
  min-height: 180px;
  padding: 24px;
  text-align: center;

  strong {
    color: var(--lumina-text);
    font-size: 14px;
  }

  span {
    max-width: 420px;
  }

  button {
    background: var(--lumina-button-secondary-bg);
    border: 1px solid var(--lumina-card-border);
    border-radius: var(--lumina-radius-sm);
    color: var(--lumina-text);
    cursor: pointer;
    height: 30px;
    justify-self: center;
    padding: 0 12px;

    &:hover {
      background: var(--lumina-button-secondary-hover);
    }
  }
}

.file-table {
  min-height: 0;
  overflow: auto;
}

.table-header,
.file-row {
  display: grid;
}

.table-header {
  background: var(--lumina-surface-2);
  border-bottom: 1px solid var(--lumina-card-border);
  color: var(--lumina-text-secondary);
  font-size: 11px;
  font-weight: 650;
  height: 30px;
  position: sticky;
  top: 0;
  z-index: 3;
}

.header-cell {
  align-items: center;
  border-right: 1px solid var(--lumina-card-border);
  display: flex;
  min-width: 0;
  padding: 0 8px;
  position: relative;
  white-space: nowrap;

  &.numeric-header {
    justify-content: flex-end;
  }
}

.column-resizer {
  bottom: 0;
  cursor: col-resize;
  position: absolute;
  right: -3px;
  top: 0;
  width: 6px;
  z-index: 2;
}

.file-row {
  border-bottom: 1px solid color-mix(in srgb, var(--lumina-card-border) 72%, transparent);
  content-visibility: auto;
  contain-intrinsic-size: 0 30px;
  cursor: default;
  min-height: 30px;

  &:hover {
    background: var(--lumina-button-secondary-hover);
  }

  &.active {
    background: color-mix(in srgb, var(--lumina-primary-soft) 42%, var(--lumina-surface-2));
    box-shadow: inset 2px 0 0 var(--lumina-primary);
  }
}

.commit-check,
.path-cell,
.extension-cell,
.status-cell,
.line-cell,
.score-cell,
.review-reason-cell {
  align-items: center;
  border-right: 1px solid color-mix(in srgb, var(--lumina-card-border) 72%, transparent);
  display: flex;
  min-width: 0;
}

.commit-check {
  justify-content: center;
}

.path-cell {
  color: var(--lumina-text);
  font-size: 12px;
  gap: 7px;
  overflow: hidden;
  padding: 0 8px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.extension-cell,
.status-cell,
.line-cell,
.score-cell {
  font-size: 11px;
  padding: 0 8px;
}

.line-cell {
  justify-content: flex-end;
}

.added-lines {
  color: var(--lumina-success);
}

.removed-lines {
  color: var(--lumina-danger);
}

.score-cell {
  color: var(--lumina-text-secondary);
  justify-content: flex-end;
}

.review-reason-cell {
  align-items: center;
  color: var(--lumina-text-secondary);
  display: flex;
  font-size: 11px;
  gap: 5px;
  overflow: hidden;
  padding: 0 8px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.review-category {
  background: color-mix(in srgb, currentColor 9%, transparent);
  border: 1px solid color-mix(in srgb, currentColor 24%, transparent);
  border-radius: 999px;
  display: inline-flex;
  font-size: 10px;
  line-height: 18px;
  padding: 0 7px;
}

.attention-dot {
  background: var(--lumina-primary);
  border-radius: 999px;
  box-shadow: 0 0 0 2px var(--lumina-primary-soft);
  flex: 0 0 auto;
  height: 6px;
  width: 6px;
}

.tone-warning {
  color: var(--lumina-warning);
}

.tone-success {
  color: var(--lumina-success);
}

.tone-danger {
  color: var(--lumina-danger);
}

.tone-info {
  color: var(--lumina-primary);
}

.tone-muted {
  color: var(--lumina-text-secondary);
}

.mono {
  font-family: SFMono-Regular, Consolas, 'Liberation Mono', Menlo, monospace;
}
</style>
