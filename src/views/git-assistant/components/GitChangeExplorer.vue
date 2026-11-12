<template>
  <section class="change-explorer">
    <header class="change-header">
      <div>
        <span>{{ t('gitAssistant.files.title') }}</span>
        <small>{{ t('gitAssistant.files.refreshHint') }}</small>
      </div>
      <strong>{{ t('gitAssistant.files.selectedTotal', { selected: selectedRaws.length, total: totalCount }) }}</strong>
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
    </div>

    <div class="check-toolbar">
      <span>{{ t('gitAssistant.files.check') }}:</span>
      <button type="button" @click="$emit('set-file-selection', visibleRaws)">
        {{ t('gitAssistant.files.selectAll') }}
      </button>
      <button type="button" @click="$emit('set-file-selection', [])">
        {{ t('gitAssistant.files.selectNone') }}
      </button>
      <button type="button" @click="$emit('set-file-selection', unversionedRaws)">
        {{ t('gitAssistant.files.filters.untracked') }}
      </button>
      <button type="button" @click="$emit('set-file-selection', versionedRaws)">
        {{ t('gitAssistant.files.filters.versioned') }}
      </button>
      <button type="button" @click="$emit('set-file-selection', addedRaws)">
        {{ t('gitAssistant.files.filters.added') }}
      </button>
      <button type="button" @click="$emit('set-file-selection', deletedRaws)">
        {{ t('gitAssistant.files.filters.deleted') }}
      </button>
      <button type="button" @click="$emit('set-file-selection', modifiedRaws)">
        {{ t('gitAssistant.files.filters.modified') }}
      </button>
      <button type="button" @click="$emit('set-file-selection', conflictedRaws)">
        {{ t('gitAssistant.files.filters.conflicted') }}
      </button>
      <button type="button" @click="$emit('set-file-selection', visibleRaws)">
        {{ t('gitAssistant.files.filters.files') }}
      </button>
    </div>

    <WorkbenchEmptyState v-if="!hasSnapshot && !loading" icon="solar:folder-open-linear" :title="t('gitAssistant.files.emptyNoRepoTitle')" :description="t('gitAssistant.files.emptyNoRepo')" />
    <WorkbenchEmptyState v-else-if="loading" icon="solar:refresh-circle-linear" :title="t('gitAssistant.files.emptyLoadingTitle')" :description="t('gitAssistant.files.emptyLoading')" />
    <WorkbenchEmptyState v-else-if="!files.length" icon="solar:check-circle-linear" :title="totalCount ? t('gitAssistant.files.emptyNoMatchTitle') : t('gitAssistant.files.emptyCleanTitle')" :description="totalCount ? t('gitAssistant.files.emptyNoMatch') : t('gitAssistant.files.emptyClean')">
      <template #actions><WorkbenchButton @click="$emit('request-refresh')">{{ t('gitAssistant.repo.refreshRepo') }}</WorkbenchButton></template>
    </WorkbenchEmptyState>

    <div v-else ref="fileTable" class="file-table" @scroll.passive="measureFileTable">
      <div class="table-header" :style="gridStyle">
        <span class="check-cell header-cell">
          <input
            ref="headerCheckbox"
            :checked="allVisibleSelected"
            type="checkbox"
            @change="$emit('set-file-selection', allVisibleSelected ? [] : visibleRaws)"
          />
        </span>
        <span
          v-for="column in resizableColumns"
          :key="column.key"
          class="header-cell"
          :class="{ 'numeric-header': column.key === 'added' || column.key === 'removed' }"
        >
          {{ t(column.labelKey) }}
          <i class="column-resizer" @mousedown.prevent="startColumnResize(column.key, $event)"></i>
        </span>
      </div>

      <div v-if="rowWindow.start" :style="{ height: `${rowWindow.start * FILE_ROW_HEIGHT}px` }" aria-hidden="true"></div>
      <div
        v-for="file in renderedFiles"
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
            :checked="selectedRawSet.has(file.raw)"
            type="checkbox"
            @change="
              $emit('toggle-file-selection', {
                raw: file.raw,
                checked: ($event.target as HTMLInputElement).checked,
              })
            "
          />
        </label>

        <div class="path-cell mono" :title="file.path">
          {{ file.path }}
        </div>
        <div class="extension-cell mono">{{ file.extension || '-' }}</div>
        <div class="status-cell" :class="`tone-${statusMeta[file.type].tone}`">
          {{ t(statusMeta[file.type].labelKey) }}
        </div>
        <div class="line-cell added-lines">{{ formatLineCount(file.addedLines) }}</div>
        <div class="line-cell removed-lines">{{ formatLineCount(file.removedLines) }}</div>
      </div>
      <div v-if="rowWindow.end < files.length" :style="{ height: `${(files.length - rowWindow.end) * FILE_ROW_HEIGHT}px` }" aria-hidden="true"></div>
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
import { computed, h, onMounted, onUnmounted, reactive, ref, watch, watchEffect } from 'vue'
import { NDropdown, type DropdownOption } from 'naive-ui'
import { Icon } from '@iconify/vue'
import { useLocale } from '@/composables/useLocale'
import { STATUS_FILTER_OPTIONS, STATUS_META } from '../git-assistant.config'
import WorkbenchEmptyState from '@/components/workbench/WorkbenchEmptyState.vue'
import WorkbenchButton from '@/components/workbench/WorkbenchButton.vue'
import { hasPrimaryModifier } from '@/utils/platform-shortcuts'
import type {
  GitAssistantFileView,
  GitAssistantStatusFilter,
} from '../git-assistant.types'

const props = defineProps<{
  hasSnapshot: boolean
  loading: boolean
  keyword: string
  statusFilter: GitAssistantStatusFilter
  files: GitAssistantFileView[]
  totalCount: number
  activeFileRaw: string | null
  selectedRaws: string[]
}>()

const emit = defineEmits<{
  (e: 'update:keyword', value: string): void
  (e: 'update:status-filter', value: string): void
  (e: 'select-file', raw: string): void
  (e: 'open-diff', raw: string): void
  (e: 'request-refresh'): void
  (e: 'file-action', payload: { action: 'open-diff' | 'diff-previous' | 'file-history' | 'open-external' | 'mark-resolved' | 'revert' | 'stage' | 'unstage'; raw: string }): void
  (e: 'toggle-file-selection', payload: { raw: string; checked: boolean }): void
  (e: 'set-file-selection', raws: string[]): void
}>()

const statusMeta = STATUS_META
const statusFilterOptions = STATUS_FILTER_OPTIONS
const { t } = useLocale()
const searchInput = ref<HTMLInputElement | null>(null)
const FILE_ROW_HEIGHT = 30
const fileTable = ref<HTMLElement | null>(null)
const fileTableTop = ref(0)
const fileTableHeight = ref(0)
const rowWindow = computed(() => {
  const total = props.files.length
  if (total <= 200) return { start: 0, end: total }
  const visibleCount = Math.ceil((fileTableHeight.value || 480) / FILE_ROW_HEIGHT)
  const first = Math.min(Math.floor(fileTableTop.value / FILE_ROW_HEIGHT), Math.max(0, total - visibleCount))
  return { start: Math.max(0, first - 12), end: Math.min(total, first + visibleCount + 12) }
})
const renderedFiles = computed(() => props.files.slice(rowWindow.value.start, rowWindow.value.end))

function measureFileTable() {
  fileTableTop.value = fileTable.value?.scrollTop ?? 0
  fileTableHeight.value = fileTable.value?.clientHeight ?? 0
}

watch(fileTable, (table, _oldTable, onCleanup) => {
  measureFileTable()
  if (!table || typeof ResizeObserver === 'undefined') return
  const observer = new ResizeObserver(measureFileTable)
  observer.observe(table)
  onCleanup(() => observer.disconnect())
})

watch(() => [props.keyword, props.statusFilter], () => {
  if (fileTable.value) fileTable.value.scrollTop = 0
  measureFileTable()
})

const visibleRaws = computed(() => props.files.map(file => file.raw))
const selectedRawSet = computed(() => new Set(props.selectedRaws))
const selectedVisibleCount = computed(() => visibleRaws.value.filter(raw => selectedRawSet.value.has(raw)).length)
const allVisibleSelected = computed(() => visibleRaws.value.length > 0 && selectedVisibleCount.value === visibleRaws.value.length)
const partiallyVisibleSelected = computed(() => selectedVisibleCount.value > 0 && !allVisibleSelected.value)
const unversionedRaws = computed(() => props.files.filter(file => file.type === 'untracked').map(file => file.raw))
const versionedRaws = computed(() => props.files.filter(file => file.type !== 'untracked').map(file => file.raw))
const addedRaws = computed(() => props.files.filter(file => file.type === 'added').map(file => file.raw))
const deletedRaws = computed(() => props.files.filter(file => file.type === 'deleted').map(file => file.raw))
const modifiedRaws = computed(() => props.files.filter(file => file.type === 'modified').map(file => file.raw))
const conflictedRaws = computed(() => props.files.filter(file => file.type === 'updated-but-unmerged').map(file => file.raw))
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
})

const resizableColumns = [
  { key: 'path', labelKey: 'gitAssistant.files.tablePath' },
  { key: 'extension', labelKey: 'gitAssistant.files.tableExtension' },
  { key: 'status', labelKey: 'gitAssistant.files.tableStatus' },
  { key: 'added', labelKey: 'gitAssistant.files.tableAdded' },
  { key: 'removed', labelKey: 'gitAssistant.files.tableRemoved' },
] as const

type ResizableColumnKey = keyof typeof columnWidths

const gridStyle = computed(() => ({
  gridTemplateColumns: `34px ${columnWidths.path}px ${columnWidths.extension}px ${columnWidths.status}px ${columnWidths.added}px ${columnWidths.removed}px`,
}))
const contextFile = computed(() => props.files.find(file => file.raw === contextFileRaw.value) ?? null)

function menuIcon(icon: string) {
  return () => h(Icon, { icon, size: 16 })
}

const contextMenuOptions = computed<DropdownOption[]>(() => {
  const file = contextFile.value
  if (!file) return []

  const options: DropdownOption[] = [
    {
      label: t('gitAssistant.files.menu.openDiff'),
      key: 'open-diff',
      icon: menuIcon('solar:code-file-linear'),
    },
    {
      label: t('gitAssistant.files.menu.openExternal'),
      key: 'open-external',
      icon: menuIcon('solar:square-arrow-right-up-linear'),
      disabled: file.type === 'deleted',
    },
  ]

  const workingTreeActions: DropdownOption[] = []
  if (file.unstaged) {
    workingTreeActions.push({
      label: t('gitAssistant.files.menu.stage'),
      key: 'stage',
      icon: menuIcon('solar:add-square-linear'),
    })
  }
  if (file.staged) {
    workingTreeActions.push({
      label: t('gitAssistant.files.menu.unstage'),
      key: 'unstage',
      icon: menuIcon('solar:minus-square-linear'),
    })
  }
  if (file.type === 'updated-but-unmerged') {
    workingTreeActions.push({
      label: t('gitAssistant.files.menu.markResolved'),
      key: 'mark-resolved',
      icon: menuIcon('solar:check-circle-linear'),
    })
  }
  if (workingTreeActions.length) {
    options.push({ type: 'divider', key: 'divider-working-tree' }, ...workingTreeActions)
  }

  if (file.type !== 'untracked') {
    options.push(
      { type: 'divider', key: 'divider-history' },
      {
        label: t('gitAssistant.files.menu.diffPrevious'),
        key: 'diff-previous',
        icon: menuIcon('solar:history-2-linear'),
      },
      {
        label: t('gitAssistant.files.menu.fileHistory'),
        key: 'file-history',
        icon: menuIcon('solar:history-linear'),
      }
    )
  }

  options.push(
    { type: 'divider', key: 'divider-copy' },
    {
      label: t('gitAssistant.files.menu.copyPath'),
      key: 'copy-path',
      icon: menuIcon('solar:copy-linear'),
    },
    { type: 'divider', key: 'divider-danger' },
    {
      label: t(
        file.type === 'untracked'
          ? 'gitAssistant.files.menu.deleteUntracked'
          : 'gitAssistant.files.menu.discardChanges'
      ),
      key: 'revert',
      icon: menuIcon(file.type === 'untracked' ? 'solar:trash-bin-trash-linear' : 'solar:undo-left-linear'),
      props: { class: 'git-context-menu-danger' },
    }
  )

  return options
})

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
  if (document.querySelector('.macos-skills-window') || document.querySelector('.n-modal-container')) return
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
  grid-template-columns: minmax(120px, 1fr) 104px;
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
  box-sizing: border-box;
  cursor: default;
  height: 30px;

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
.line-cell {
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
.line-cell {
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

:global(.git-context-menu-danger .n-dropdown-option-body__label),
:global(.git-context-menu-danger .n-dropdown-option-body__prefix) {
  color: var(--lumina-danger);
}

:global(.git-context-menu-danger .n-dropdown-option-body:hover) {
  background: color-mix(in srgb, var(--lumina-danger) 10%, transparent);
}
</style>
