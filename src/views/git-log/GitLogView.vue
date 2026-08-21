<template>
  <div class="git-log-page">
    <header class="git-log-dialog__header" data-tauri-drag-region>
      <div class="git-log-title">
        <strong>{{ filePath ? t('gitAssistant.log.fileTitle') : t('gitAssistant.log.title') }}</strong>
        <span>{{ filePath || branch || '' }}</span>
      </div>
      <div class="git-log-toolbar">
        <NDatePicker
          v-model:value="logDateFrom"
          class="git-log-date"
          type="date"
          size="small"
          clearable
          :placeholder="t('gitAssistant.log.from')"
        />
        <NDatePicker
          v-model:value="logDateTo"
          class="git-log-date"
          type="date"
          size="small"
          clearable
          :placeholder="t('gitAssistant.log.to')"
        />
        <NInput
          v-model:value="logKeyword"
          class="git-log-search"
          size="small"
          clearable
          :placeholder="t('gitAssistant.log.searchPlaceholder')"
        />
        <NSelect
          v-model:value="logAuthorFilter"
          class="git-log-author"
          size="small"
          :consistent-menu-width="false"
          :options="logAuthorOptions"
        />
        <span class="git-log-count">{{ t('gitAssistant.log.visibleCount', { count: filteredGitLogEntries.length, total: gitLogEntries.length }) }}</span>
      </div>
    </header>

    <WorkbenchEmptyState v-if="logLoading" icon="solar:refresh-circle-linear" :title="t('gitAssistant.log.loading')" />
    <WorkbenchEmptyState v-else-if="!gitLogEntries.length" icon="solar:history-linear" :title="t('gitAssistant.log.empty')" />
    <WorkbenchEmptyState v-else-if="!filteredGitLogEntries.length" icon="solar:magnifer-linear" :title="t('gitAssistant.log.noMatch')" />
    <section v-else class="git-log-content">
      <section class="git-log-revision-table wb-table">
        <div class="git-log-table-head wb-table-head">
          <span>{{ t('gitAssistant.log.columnGraph') }}</span>
          <span>{{ t('gitAssistant.log.columnMessage') }}</span>
          <span>{{ t('gitAssistant.log.columnAuthor') }}</span>
          <span>{{ t('gitAssistant.log.columnDate') }}</span>
          <span>{{ t('gitAssistant.log.columnHash') }}</span>
        </div>
        <button
          v-for="entry in filteredGitLogEntries"
          :key="entry.hash"
          class="git-log-row"
          :class="{ active: activeLogHash === entry.hash }"
          type="button"
          @click="handleSelectLogEntry(entry.hash)"
        >
          <span class="git-log-graph"><i></i></span>
          <strong>{{ entry.subject }}</strong>
          <span>{{ entry.authorName }}</span>
          <span>{{ formatLogDate(entry.date) }}</span>
          <code>{{ entry.shortHash }}</code>
        </button>
      </section>

      <section class="git-log-selected">
        <template v-if="gitLogDetail">
          <div class="git-log-selected__summary">
            <div>
              <div class="git-log-selected__sha mono">SHA-1: {{ gitLogDetail.hash }}</div>
              <strong>{{ gitLogDetail.subject }}</strong>
            </div>
            <span>{{ gitLogDetail.authorName }} &lt;{{ gitLogDetail.authorEmail }}&gt; {{ formatLogDate(gitLogDetail.date) }}</span>
            <small>{{ gitLogDetail.shortStat || t('gitAssistant.log.changedFiles') }}</small>
          </div>
          <pre v-if="gitLogDetail.body" class="git-log-selected__body">{{ gitLogDetail.body }}</pre>
        </template>
        <div v-else class="git-log-empty git-log-empty--compact">
          {{ logDetailLoading ? t('gitAssistant.log.detailLoading') : t('gitAssistant.log.selectCommit') }}
        </div>
      </section>

      <section class="git-log-bottom">
        <section class="git-log-file-table wb-table">
          <div class="git-log-file-head wb-table-head">
            <span>{{ t('gitAssistant.log.columnPath') }}</span>
            <span>{{ t('gitAssistant.log.columnExtension') }}</span>
            <span>{{ t('gitAssistant.log.columnStatus') }}</span>
            <span class="numeric-header">{{ t('gitAssistant.log.columnAdded') }}</span>
            <span class="numeric-header">{{ t('gitAssistant.log.columnRemoved') }}</span>
          </div>
          <button
            v-for="file in gitLogDetail?.changedFiles ?? []"
            :key="`${file.status}-${file.path}`"
            class="git-log-file-row"
            :class="{ active: activeLogFilePath === file.path }"
            type="button"
            @click="handleOpenLogFileDiff(file)"
          >
            <span class="mono" :title="file.path">{{ file.path }}</span>
            <span>{{ getFileExtension(file.path) || '-' }}</span>
            <span class="status-cell" :class="`tone-${logFileStatusMeta(file.status).tone}`">{{ t(logFileStatusMeta(file.status).labelKey) }}</span>
            <span class="line-cell added-lines">{{ formatCommitLineCount(file.added) }}</span>
            <span class="line-cell removed-lines">{{ formatCommitLineCount(file.removed) }}</span>
          </button>
        </section>
      </section>
    </section>

  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { NDatePicker, NInput, NSelect } from 'naive-ui'
import { useLocale } from '@/hooks/useLocale'
import {
  type GitCommitChangedFile,
  type GitCommitDetail,
  type GitLogEntry,
  loadGitCommitDetail,
  loadGitLog,
} from '@/services/git/git-service'
import { STATUS_META } from '@/views/git-assistant/git-assistant.config'
import { emit as tauriEmit, listen } from '@tauri-apps/api/event'
import { openGitDiffWindow } from '@/services/git/git-diff-window'
import WorkbenchEmptyState from '@/components/workbench/WorkbenchEmptyState.vue'

const { t } = useLocale()

// ── State ──
const repoPath = ref('')
const filePath = ref('')
const branch = ref('')
const logLoading = ref(false)
const logDetailLoading = ref(false)
const gitLogEntries = ref<GitLogEntry[]>([])
const gitLogDetail = ref<GitCommitDetail | null>(null)
const activeLogHash = ref('')
const activeLogFilePath = ref('')
const logKeyword = ref('')
const logAuthorFilter = ref('all')
const logDateFrom = ref<number | null>(null)
const logDateTo = ref<number | null>(null)

// ── Helpers ──
function parseGitLogDate(date: string) {
  const timestamp = Date.parse(date.replace(/\//g, '-'))
  return Number.isNaN(timestamp) ? null : timestamp
}

function startOfDay(timestamp: number) {
  const date = new Date(timestamp)
  date.setHours(0, 0, 0, 0)
  return date.getTime()
}

function endOfDay(timestamp: number) {
  const date = new Date(timestamp)
  date.setHours(23, 59, 59, 999)
  return date.getTime()
}

function formatLogDate(date: string) {
  const parsed = new Date(date)
  if (Number.isNaN(parsed.getTime())) return date
  return parsed.toLocaleString()
}

function formatCommitLineCount(value: number | null) {
  return value === null ? '-' : String(value)
}

function getFileExtension(path: string) {
  const name = path.split(/[/\\]/).pop() || path
  const index = name.lastIndexOf('.')
  return index === -1 ? '' : name.slice(index + 1).toLowerCase()
}

function logFileStatusMeta(statusCode: string) {
  const code = statusCode.slice(0, 1)
  const typeMap: Record<string, keyof typeof STATUS_META> = {
    A: 'added', D: 'deleted', R: 'renamed', C: 'copied', M: 'modified', U: 'untracked',
  }
  const type = typeMap[code] ?? 'unknown'
  return STATUS_META[type]
}

function setDefaultLogDateRange(entries: GitLogEntry[]) {
  const timestamps = entries.map(e => parseGitLogDate(e.date)).filter((t): t is number => t !== null).sort((a, b) => a - b)
  logDateFrom.value = timestamps[0] ?? null
  logDateTo.value = Date.now()
}

// ── Computed ──
const logAuthorOptions = computed(() => {
  const authors = [...new Set(gitLogEntries.value.map(e => e.authorName).filter(Boolean))].sort()
  return [{ label: t('gitAssistant.log.allAuthors'), value: 'all' }, ...authors.map(a => ({ label: a, value: a }))]
})

const filteredGitLogEntries = computed(() => {
  const kw = logKeyword.value.trim().toLowerCase()
  const fromTime = logDateFrom.value ? startOfDay(logDateFrom.value) : null
  const toTime = logDateTo.value ? endOfDay(logDateTo.value) : null
  return gitLogEntries.value.filter(entry => {
    if (logAuthorFilter.value !== 'all' && entry.authorName !== logAuthorFilter.value) return false
    const t2 = parseGitLogDate(entry.date)
    if (fromTime !== null && t2 !== null && t2 < fromTime) return false
    if (toTime !== null && t2 !== null && t2 > toTime) return false
    if (!kw) return true
    return [entry.subject, entry.authorName, entry.authorEmail, entry.hash, entry.shortHash, entry.date]
      .some(v => v.toLowerCase().includes(kw))
  })
})

// ── Actions ──
async function loadLog() {
  if (!repoPath.value) return
  logLoading.value = true
  try {
    gitLogEntries.value = await loadGitLog(repoPath.value, filePath.value)
    setDefaultLogDateRange(gitLogEntries.value)
    if (gitLogEntries.value.length) {
      await handleSelectLogEntry(gitLogEntries.value[0].hash)
    }
  } catch (err) {
    console.error(err)
    gitLogEntries.value = []
  } finally {
    logLoading.value = false
  }
}

async function handleSelectLogEntry(hash: string) {
  if (!repoPath.value || !hash) return
  activeLogHash.value = hash
  activeLogFilePath.value = ''
  logDetailLoading.value = true
  try {
    const detail = await loadGitCommitDetail(repoPath.value, hash)
    gitLogDetail.value = detail
    activeLogFilePath.value = filePath.value || detail.changedFiles[0]?.path || ''
  } catch (err) {
    console.error(err)
    gitLogDetail.value = null
  } finally {
    logDetailLoading.value = false
  }
}

async function handleOpenLogFileDiff(file: GitCommitChangedFile) {
  if (!repoPath.value || !activeLogHash.value || !file.path) return
  activeLogFilePath.value = file.path
  await openGitDiffWindow({ kind: 'commit', repoPath: repoPath.value, hash: activeLogHash.value, filePath: file.path })
}

// ── Tauri Events ──
let unlistenInit: (() => void) | null = null

onMounted(async () => {
  unlistenInit = await listen<{ repoPath: string; filePath: string; branch: string }>('git-log-init', async (event) => {
    repoPath.value = event.payload.repoPath
    filePath.value = event.payload.filePath
    branch.value = event.payload.branch
    await loadLog()
  })

  // Request init data from main window
  await tauriEmit('git-log-request-init')
})

onUnmounted(() => {
  unlistenInit?.()
})
</script>

<style scoped lang="scss">
.git-log-page {
  background: var(--lumina-content-bg);
  color: var(--lumina-text);
  display: flex;
  flex-direction: column;
  height: 100vh;
  overflow: hidden;
}

.git-log-dialog__header {
  align-items: center;
  background: var(--lumina-toolbar-bg);
  backdrop-filter: var(--lumina-vibrancy);
  border-bottom: 0.5px solid var(--lumina-separator);
  display: flex;
  gap: 12px;
  justify-content: space-between;
  min-height: 50px;
  padding: 8px 16px;
  flex-shrink: 0;
}

.git-log-title {
  align-items: baseline;
  display: flex;
  flex: 0 0 auto;
  gap: 10px;
  min-width: 0;

  strong {
    color: var(--lumina-text);
    font-size: 14px;
    font-weight: 700;
  }

  span {
    color: var(--lumina-text-secondary);
    font-size: 12px;
    max-width: 420px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
}

.git-log-toolbar {
  align-items: center;
  display: flex;
  flex: 1 1 auto;
  gap: 8px;
  justify-content: flex-end;
  min-width: 0;
}

.git-log-search {
  max-width: 440px;
  min-width: 220px;
}

.git-log-author {
  width: 150px;
}

.git-log-date {
  flex: 0 0 auto;
  width: 128px;
}

.git-log-count {
  color: var(--lumina-text-secondary);
  flex: 0 0 auto;
  font-size: 12px;
}

.git-log-content {
  display: grid;
  grid-template-rows: minmax(240px, 1fr) minmax(120px, auto) minmax(170px, 1fr);
  min-height: 0;
  overflow: hidden;
  flex: 1;
}

.git-log-revision-table {
  min-height: 0;
  overflow: auto;
}

.git-log-table-head,
.git-log-row {
  display: grid;
  grid-template-columns: 46px 720px 170px 190px 96px;
  min-width: 100%;
  width: max-content;
}

.git-log-table-head {
  background: var(--lumina-surface-2);
  border-bottom: 0.5px solid var(--lumina-separator);
  color: var(--lumina-text-secondary);
  font-size: 11px;
  font-weight: 700;
  height: 28px;
  position: sticky;
  top: 0;
  z-index: 3;

  span {
    align-items: center;
    border-right: 0.5px solid var(--lumina-separator);
    display: flex;
    min-width: 0;
    padding: 0 8px;
  }
}

.git-log-row {
  background: transparent;
  border: 0;
  border-bottom: 0.5px solid color-mix(in srgb, var(--lumina-separator) 72%, transparent);
  color: var(--lumina-text);
  cursor: pointer;
  font: inherit;
  min-height: 28px;
  padding: 0;
  text-align: left;
  transition: background 0.12s ease, color 0.12s ease;

  &:hover,
  &.active {
    background: color-mix(in srgb, var(--lumina-primary-soft) 54%, var(--lumina-surface-2));
  }

  > span,
  > strong,
  > code {
    align-items: center;
    border-right: 0.5px solid color-mix(in srgb, var(--lumina-separator) 64%, transparent);
    display: flex;
    min-width: 0;
    overflow: hidden;
    padding: 0 8px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  > strong {
    font-size: 12px;
    font-weight: 600;
  }

  > span {
    color: var(--lumina-text-secondary);
    font-size: 12px;
  }

  > code {
    background: transparent;
    color: var(--lumina-text-secondary);
    font-size: 11px;
  }
}

.git-log-graph {
  justify-content: center;

  i {
    background: var(--lumina-primary);
    border-radius: 999px;
    height: 7px;
    width: 7px;
  }
}

.git-log-selected {
  background: color-mix(in srgb, var(--lumina-surface-2) 82%, transparent);
  border-bottom: 0.5px solid var(--lumina-separator);
  border-top: 0.5px solid var(--lumina-separator);
  display: grid;
  gap: 6px;
  min-height: 0;
  overflow: hidden;
  padding: 8px 10px;
}

.git-log-selected__sha {
  color: var(--lumina-text-secondary);
  font-size: 11px;
  min-width: 0;
  white-space: nowrap;
}

.git-log-selected__summary {
  align-items: start;
  display: grid;
  gap: 6px;
  grid-template-columns: minmax(360px, 1fr) minmax(220px, auto) minmax(170px, auto);
  min-width: 0;

  strong,
  span,
  small {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  strong {
    color: var(--lumina-text);
    display: block;
    font-size: 13px;
    margin-top: 2px;
  }

  span,
  small {
    color: var(--lumina-text-secondary);
    font-size: 11px;
  }
}

.git-log-selected__body {
  color: var(--lumina-text-secondary);
  font-family: inherit;
  font-size: 11px;
  line-height: 1.45;
  margin: 0;
  max-height: 52px;
  min-width: 0;
  overflow: auto;
  white-space: pre-wrap;
}

.git-log-empty {
  align-items: center;
  color: var(--lumina-text-secondary);
  display: flex;
  font-size: 12px;
  justify-content: center;
  min-height: 220px;
  padding: 20px;
}

.git-log-empty--compact {
  min-height: 0;
  padding: 8px 10px;
}

.git-log-bottom {
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
}

.git-log-file-table {
  min-height: 0;
  overflow: auto;
}

.git-log-file-head,
.git-log-file-row {
  display: grid;
  grid-template-columns: 760px 96px 96px 84px 94px;
  min-width: 100%;
  width: max-content;
}

.git-log-file-head {
  background: var(--lumina-surface-2);
  border-bottom: 0.5px solid var(--lumina-separator);
  color: var(--lumina-text-secondary);
  font-size: 11px;
  font-weight: 700;
  height: 28px;
  position: sticky;
  top: 0;
  z-index: 3;

  span {
    align-items: center;
    border-right: 0.5px solid var(--lumina-separator);
    display: flex;
    min-width: 0;
    padding: 0 8px;

    &.numeric-header {
      justify-content: flex-end;
    }
  }
}

.git-log-file-row {
  align-items: center;
  background: transparent;
  border: 0;
  border-bottom: 0.5px solid color-mix(in srgb, var(--lumina-separator) 70%, transparent);
  color: var(--lumina-text);
  cursor: pointer;
  display: grid;
  min-height: 28px;
  padding: 0;
  text-align: left;

  span {
    align-items: center;
    border-right: 0.5px solid color-mix(in srgb, var(--lumina-separator) 64%, transparent);
    display: flex;
    font-size: 12px;
    min-width: 0;
    overflow: hidden;
    padding: 0 8px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  &:hover,
  &.active {
    background: color-mix(in srgb, var(--lumina-primary-soft) 48%, var(--lumina-surface-2));
  }
}

.status-cell {
  font-size: 12px;
  font-weight: 600;
}

.line-cell {
  font-size: 12px;
  justify-content: flex-end;
}

.added-lines {
  color: var(--lumina-success);
}

.removed-lines {
  color: var(--lumina-danger);
}

.tone-warning { color: var(--lumina-warning); }
.tone-success { color: var(--lumina-success); }
.tone-danger { color: var(--lumina-danger); }
.tone-info { color: var(--lumina-primary); }
.tone-muted { color: var(--lumina-text-secondary); }

</style>
