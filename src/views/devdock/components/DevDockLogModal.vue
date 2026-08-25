<template>
  <NModal
    :show="show"
    class="process-log-modal"
    :mask-closable="true"
    @update:show="handleShowUpdate"
    @after-leave="$emit('afterLeave')"
  >
    <WorkbenchModalPanel size="log" :close-label="t('common.dismiss')" @close="$emit('close')">
      <section class="process-log-dialog">
        <!-- Window Titlebar & Control Bar -->
        <header class="process-log-dialog__header">
          <div class="process-log-header-main">
            <div class="process-title-group">
              <div class="process-title-row">
                <h3 class="process-project-name">{{ logs?.process.projectName ?? '' }}</h3>
                <span class="process-cmd-badge">
                  <Icon icon="solar:programming-linear" class="cmd-icon" />
                  {{ logs?.process.commandName || logs?.process.scriptName || '' }}
                </span>
              </div>
              <div class="process-subtitle-row">
                <span class="process-command-preview" :title="description">
                  <Icon icon="solar:terminal-linear" class="sub-icon" />
                  {{ description }}
                </span>
              </div>
            </div>
          </div>

          <div class="process-log-controls">
            <div class="process-log-search" @click="focusSearch">
              <Icon icon="solar:magnifer-linear" class="search-icon" />
              <input
                ref="searchInputRef"
                :value="search"
                type="search"
                :placeholder="t('devdock.processes.searchLogs')"
                @click.stop
                @mousedown.stop
                @input="search = ($event.target as HTMLInputElement).value.trim()"
                @keydown.stop
              />
              <button
                v-if="search"
                type="button"
                class="search-clear-btn"
                :title="t('devdock.processes.clearSearch')"
                @click.stop="clearSearch"
              >
                <Icon icon="solar:close-circle-bold" />
              </button>
              <span class="search-match-badge">{{ t('devdock.processes.logMatches', { count: visibleLines.length }) }}</span>
            </div>

            <div
              class="process-log-filters"
              role="group"
              :aria-label="t('devdock.processes.logLevelFilter')"
            >
              <button
                v-for="option in filterOptions"
                :key="option.level"
                type="button"
                :class="[
                  'process-log-filter',
                  option.level,
                  { active: levelFilter === option.level },
                ]"
                @click="levelFilter = option.level"
              >
                <span class="filter-label">{{ option.label }}</span>
                <strong class="filter-count">{{ option.count }}</strong>
              </button>
            </div>
          </div>
        </header>

        <!-- Terminal Canvas -->
        <div class="process-log-canvas-wrapper">
          <section
            v-if="logs"
            ref="viewportRef"
            class="process-log-list mac-terminal"
            aria-live="polite"
            @scroll="onScroll"
          >
            <pre
              v-for="(line, idx) in visibleLines"
              :key="`${line.timestamp}:${line.stream}:${idx}`"
              class="mac-log-line"
              :class="[line.stream, line.level]"
            ><span class="log-gutter"><span v-if="line.level === 'warning'" class="log-badge warning">WARN</span><span v-else-if="line.level === 'error'" class="log-badge error">ERR</span><span v-else-if="line.stream === 'system'" class="log-badge system">SYS</span></span><span class="log-content-text" v-html="renderLogLine(line.text)" /></pre>

            <pre
              v-if="!visibleLines.length"
              class="mac-log-line log-empty-state"
            ><span class="log-gutter"></span><span class="log-content-text empty-msg">{{ emptyLogMessage }}</span></pre>
          </section>

          <section v-else class="process-empty">
            <Icon icon="solar:terminal-linear" class="empty-icon" />
            <strong>{{ t('devdock.processes.emptyLogsTitle') }}</strong>
            <p>{{ t('devdock.processes.emptyLogsDescription') }}</p>
          </section>
        </div>

        <!-- Terminal Status & Action Footer -->
        <footer class="process-log-dialog__footer">
          <div class="footer-meta">
            <span class="footer-state-indicator" :class="processState">
              <i class="dot"></i>
              <span class="state-text">{{ processStateLabel }}</span>
            </span>
          </div>

          <div class="footer-actions">
            <button
              type="button"
              class="mac-action-btn"
              :class="{ active: autoScroll }"
              :title="t('devdock.processes.autoScroll')"
              @click="toggleAutoScroll"
            >
              <Icon icon="solar:arrow-down-linear" />
              <span>{{ t('devdock.processes.autoScroll') }}</span>
            </button>

            <button
              type="button"
              class="mac-action-btn"
              :title="t('devdock.processes.scrollToBottom')"
              @click="scrollToBottom"
            >
              <Icon icon="solar:double-alt-arrow-down-linear" />
              <span>{{ t('devdock.processes.scrollToBottom') }}</span>
            </button>

            <button
              type="button"
              class="mac-action-btn copy-btn"
              :class="{ copied }"
              :disabled="!classifiedLines.length"
              :title="t('devdock.processes.copyLogs')"
              @click="copyAllLogs"
            >
              <Icon :icon="copied ? 'solar:check-circle-bold' : 'solar:copy-linear'" />
              <span>{{ copied ? t('devdock.processes.copiedLogs') : t('devdock.processes.copyLogs') }}</span>
            </button>
          </div>
        </footer>
      </section>
    </WorkbenchModalPanel>
  </NModal>
</template>

<script setup lang="ts">
import { AnsiUp } from 'ansi_up'
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import WorkbenchModalPanel from '@/components/workbench/WorkbenchModalPanel.vue'
import { useLocale } from '@/hooks/useLocale'
import type { ProjectProcessLogs } from '@/services/project/project-service'

type LogLevel = 'info' | 'warning' | 'error'
type LogLevelFilter = 'all' | LogLevel

const props = defineProps<{
  logs: ProjectProcessLogs | null
  show: boolean
}>()

const emit = defineEmits<{
  (e: 'afterLeave'): void
  (e: 'close'): void
}>()

const { t } = useLocale()
const search = ref('')
const levelFilter = ref<LogLevelFilter>('all')
const searchInputRef = ref<HTMLInputElement | null>(null)
const viewportRef = ref<HTMLElement | null>(null)
const autoScroll = ref(true)
const copied = ref(false)
let copyTimer: number | undefined

const ansiUp = new AnsiUp()
ansiUp.use_classes = true
ansiUp.escape_html = true

const description = computed(() => {
  if (!props.logs) return ''
  const ports = props.logs.process.ports
  return ports.length
    ? `${props.logs.process.commandPreview || props.logs.process.command} · ${t('devdock.processes.ports', { ports: ports.join(', ') })}`
    : props.logs.process.commandPreview || props.logs.process.command
})

const processState = computed(() => {
  return props.logs?.process.status.state ?? 'unknown'
})

const processStateLabel = computed(() => {
  const state = processState.value
  if (state === 'running') return t('devdock.processes.running')
  if (state === 'succeeded') return t('devdock.processes.succeeded')
  if (state === 'failed') return t('devdock.processes.failed', { code: props.logs?.process.status.exitCode ?? '--' })
  if (state === 'stopped') return t('devdock.processes.stopped')
  if (state === 'exited') return t('devdock.processes.exited', { code: props.logs?.process.status.exitCode ?? '--' })
  return t('devdock.processes.unknown')
})

const classifiedLines = computed(() => {
  const raw = props.logs?.lines ?? []
  const result: Array<{
    timestamp: number
    stream: 'stdout' | 'stderr' | 'system'
    text: string
    level: LogLevel
  }> = []

  for (const line of raw) {
    const textChunks = line.text
      .replace(/\r\n/g, '\n')
      .replace(/\r/g, '\n')
      .split('\n')

    for (const chunk of textChunks) {
      if (!chunk.trim()) continue
      result.push({
        timestamp: line.timestamp,
        stream: line.stream,
        text: chunk,
        level: classifyLogLevel(line.stream, chunk),
      })
    }
  }

  return result
})

const filterOptions = computed(() => {
  const counts = classifiedLines.value.reduce(
    (result, line) => {
      result[line.level] += 1
      return result
    },
    { info: 0, warning: 0, error: 0 }
  )
  const total = classifiedLines.value.length

  return [
    { level: 'all' as const, label: t('devdock.processes.logLevelAll'), count: total },
    { level: 'info' as const, label: t('devdock.processes.logLevelInfo'), count: counts.info },
    {
      level: 'warning' as const,
      label: t('devdock.processes.logLevelWarning'),
      count: counts.warning,
    },
    { level: 'error' as const, label: t('devdock.processes.logLevelError'), count: counts.error },
  ]
})

const visibleLines = computed(() => {
  const keyword = search.value.trim().toLowerCase()
  const lines = classifiedLines.value.filter(
    line =>
      (levelFilter.value === 'all' || line.level === levelFilter.value) &&
      (!keyword || line.text.toLowerCase().includes(keyword))
  )
  return lines.map((line, index) => ({
    ...line,
    showStream: index === 0 || lines[index - 1].stream !== line.stream,
  }))
})

const emptyLogMessage = computed(() => {
  if (search.value || levelFilter.value !== 'all') return t('devdock.processes.noLogMatches')
  const state = props.logs?.process.status.state
  if (state === 'succeeded' || state === 'exited') return t('devdock.processes.completedWithoutLogs')
  if (state === 'failed') return t('devdock.processes.failedWithoutLogs')
  if (state === 'stopped') return t('devdock.processes.stoppedWithoutLogs')
  return t('devdock.processes.runningWithoutLogs')
})

watch(
  () => props.show,
  show => {
    if (show) {
      search.value = ''
      levelFilter.value = 'all'
      autoScroll.value = true
      scrollToBottom()
    }
  }
)

watch(
  () => props.logs?.lines.length,
  () => {
    if (autoScroll.value && !search.value) {
      scrollToBottom()
    }
  }
)

watch([search, levelFilter], ([searchValue, levelValue]) => {
  if (searchValue || levelValue !== 'all') {
    autoScroll.value = false
    scrollToTop()
  } else {
    autoScroll.value = true
    scrollToBottom()
  }
})

onBeforeUnmount(() => {
  if (copyTimer) clearTimeout(copyTimer)
})

function handleShowUpdate(show: boolean) {
  if (!show) {
    emit('close')
  }
}

function focusSearch() {
  searchInputRef.value?.focus()
}

function clearSearch() {
  search.value = ''
  focusSearch()
}

function renderLogLine(text: string) {
  return ansiUp.ansi_to_html(text)
}

function formatLevelLabel(level: string) {
  if (level === 'warning') return 'WARN'
  if (level === 'error') return 'ERR'
  return 'INFO'
}

function classifyLogLevel(stream: 'stdout' | 'stderr' | 'system', text: string): LogLevel {
  const content = text.replace(/\u001B\[[0-?]*[ -/]*[@-~]/g, '').toLowerCase()
  const nonZeroExit = stream === 'system' && (
    /退出码\s+(?!0\b)\d+/.test(content) ||
    /exit code\s+(?!0\b)\d+/.test(content)
  )
  if (nonZeroExit || /\b(error|err|failed|failure|fatal|exception|panic|traceback)\b|错误|失败|异常/.test(content)) {
    return 'error'
  }
  if (/\b(warn|warning|deprecated|outdated)\b|browsers data.+\bold\b|ignored browsers|警告|已弃用/.test(content)) {
    return 'warning'
  }
  return 'info'
}

function onScroll() {
  const viewport = viewportRef.value
  if (!viewport) return
  const isAtBottom = viewport.scrollHeight - viewport.scrollTop - viewport.clientHeight < 32
  autoScroll.value = isAtBottom
}

function toggleAutoScroll() {
  autoScroll.value = !autoScroll.value
  if (autoScroll.value) {
    scrollToBottom()
  }
}

async function copyAllLogs() {
  if (!visibleLines.value.length) return
  const plainText = visibleLines.value
    .map(line => `[${line.level.toUpperCase()}] [${line.stream}] ${line.text.replace(/\u001B\[[0-?]*[ -/]*[@-~]/g, '')}`)
    .join('\n')
  try {
    await navigator.clipboard.writeText(plainText)
    copied.value = true
    if (copyTimer) clearTimeout(copyTimer)
    copyTimer = window.setTimeout(() => {
      copied.value = false
    }, 2200)
  } catch (err) {
    console.error('Failed to copy logs', err)
  }
}

function scrollToBottom() {
  nextTick(() => {
    const viewport = viewportRef.value
    if (viewport) {
      viewport.scrollTop = viewport.scrollHeight
    }
  })
}

function scrollToTop() {
  nextTick(() => {
    const viewport = viewportRef.value
    if (viewport) {
      viewport.scrollTop = 0
    }
  })
}
</script>

<style scoped lang="scss">
.process-log-dialog {
  display: grid;
  grid-template-rows: auto minmax(0, 1fr) auto;
  height: 100%;
  min-height: 0;
  background: var(--lumina-surface-elevated);
}

/* macOS Window Header */
.process-log-dialog__header {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-surface-2) 92%, var(--lumina-surface-1));
  border-bottom: 1px solid var(--lumina-card-border);
  display: grid;
  gap: 16px;
  grid-template-columns: minmax(0, 1fr) minmax(360px, 480px);
  min-height: 74px;
  padding: 12px 52px 12px 16px;
  position: relative;
  z-index: 5;
}

.process-log-header-main {
  display: flex;
  align-items: flex-start;
  min-width: 0;
}

.process-title-group {
  display: grid;
  gap: 4px;
  min-width: 0;
  flex: 1;
}

.process-title-row {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  min-width: 0;
}

.process-project-name {
  font-size: 15px;
  font-weight: 700;
  color: var(--lumina-text);
  margin: 0;
  letter-spacing: -0.01em;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.process-cmd-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  background: color-mix(in srgb, var(--lumina-primary) 12%, var(--lumina-surface-1));
  border: 1px solid color-mix(in srgb, var(--lumina-primary) 32%, var(--lumina-card-border));
  border-radius: 6px;
  color: var(--lumina-primary);
  font-family: var(--lumina-font-mono, SFMono-Regular, Consolas, monospace);
  font-size: 11.5px;
  font-weight: 600;
  padding: 2px 7px;
  white-space: nowrap;

  .cmd-icon {
    font-size: 13px;
  }
}

@keyframes pulse-dot {
  0%, 100% {
    opacity: 1;
    transform: scale(1);
  }
  50% {
    opacity: 0.4;
    transform: scale(1.25);
  }
}

.process-subtitle-row {
  display: flex;
  align-items: center;
  min-width: 0;
}

.process-command-preview {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  color: var(--lumina-text-secondary);
  font-family: var(--lumina-font-mono, SFMono-Regular, Consolas, 'Liberation Mono', Menlo, monospace);
  font-size: 11px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;

  .sub-icon {
    font-size: 12px;
    flex-shrink: 0;
  }
}

.process-log-controls {
  display: grid;
  gap: 7px;
}

/* macOS Spotlight Search Input */
.process-log-search {
  align-items: center;
  background: var(--lumina-surface-1);
  border: 1px solid var(--lumina-card-border);
  border-radius: 7px;
  box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.03);
  display: grid;
  gap: 6px;
  grid-template-columns: 16px minmax(0, 1fr) auto auto;
  height: 32px;
  padding: 0 8px 0 9px;
  position: relative;
  transition: border-color 0.15s ease, box-shadow 0.15s ease;

  .search-icon {
    color: var(--lumina-text-secondary);
    font-size: 14px;
  }

  input {
    background: transparent;
    border: 0;
    color: var(--lumina-text);
    font-size: 12px;
    min-width: 0;
    outline: none;
    padding: 0;

    &::placeholder {
      color: var(--lumina-text-tertiary);
    }
  }

  .search-clear-btn {
    align-items: center;
    background: transparent;
    border: 0;
    color: var(--lumina-text-tertiary);
    cursor: pointer;
    display: inline-flex;
    font-size: 14px;
    justify-content: center;
    padding: 0;

    &:hover {
      color: var(--lumina-text);
    }
  }

  .search-match-badge {
    align-items: center;
    background: var(--lumina-control-bg);
    border-radius: 4px;
    color: var(--lumina-text-secondary);
    display: inline-flex;
    font-family: var(--lumina-font-mono);
    font-size: 10.5px;
    font-weight: 600;
    line-height: 1;
    padding: 2px 6px;
    white-space: nowrap;
  }

  &:focus-within {
    border-color: var(--lumina-primary);
    box-shadow: 0 0 0 3px var(--lumina-accent-ring);
  }
}

/* macOS Segmented Filter Control */
.process-log-filters {
  background: var(--lumina-control-bg);
  border: 0.5px solid var(--lumina-card-border);
  border-radius: 7px;
  display: grid;
  gap: 2px;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  padding: 2px;
}

.process-log-filter {
  align-items: center;
  background: transparent;
  border: 0;
  border-radius: 5px;
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: inline-flex;
  font-size: 11px;
  font-weight: 500;
  gap: 5px;
  height: 25px;
  justify-content: center;
  line-height: 1;
  min-width: 0;
  padding: 0 6px;
  transition: all 0.15s ease;

  .filter-label {
    align-items: center;
    display: inline-flex;
    line-height: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .filter-count {
    align-items: center;
    display: inline-flex;
    font-family: var(--lumina-font-mono);
    font-size: 10.5px;
    font-weight: 700;
    line-height: 1;
    opacity: 0.85;
  }

  &:hover:not(.active) {
    background: var(--lumina-control-hover);
    color: var(--lumina-text);
  }

  &.active {
    background: var(--lumina-surface-1);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08), 0 0.5px 1px rgba(0, 0, 0, 0.05);
    color: var(--lumina-text);
    font-weight: 650;
  }

  &.warning.active {
    color: var(--lumina-warning);
    .filter-count {
      color: var(--lumina-warning);
    }
  }

  &.error.active {
    color: var(--lumina-danger);
    .filter-count {
      color: var(--lumina-danger);
    }
  }
}

/* macOS Terminal Canvas Wrapper */
.process-log-canvas-wrapper {
  background: #0d1117;
  display: grid;
  grid-template-rows: minmax(0, 1fr);
  min-height: 0;
  position: relative;
}

/* Dedicated macOS Dark Terminal Log Canvas */
.mac-terminal {
  background: #0d1117;
  box-sizing: border-box;
  color: #e6edf3;
  display: block;
  font-family: var(--lumina-font-mono);
  font-size: 12.5px;
  line-height: 1.6;
  min-height: 0;
  overflow-x: auto;
  overflow-y: scroll;
  padding: 12px 14px;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  text-rendering: optimizeLegibility;

  /* Custom macOS Dark Scrollbar */
  &::-webkit-scrollbar {
    height: 8px;
    width: 8px;
  }

  &::-webkit-scrollbar-track {
    background: transparent;
  }

  &::-webkit-scrollbar-thumb {
    background: rgba(255, 255, 255, 0.16);
    border-radius: 4px;

    &:hover {
      background: rgba(255, 255, 255, 0.28);
    }
  }
}

.mac-log-line {
  align-items: flex-start;
  border-radius: 4px;
  box-sizing: border-box;
  display: grid;
  flex-shrink: 0;
  gap: 8px;
  grid-template-columns: 36px minmax(0, 1fr);
  line-height: 1.6;
  margin: 0;
  min-height: 24px;
  overflow-wrap: anywhere;
  padding: 2px 8px;
  white-space: pre-wrap;
  word-break: break-word;

  &:hover {
    background: rgba(255, 255, 255, 0.04);
  }

  .log-gutter {
    align-items: center;
    box-sizing: border-box;
    display: inline-flex;
    height: 20px;
    justify-content: flex-start;
    user-select: none;
    width: 36px;
  }

  .log-badge {
    align-items: center;
    border-radius: 3px;
    box-sizing: border-box;
    display: inline-flex;
    font-size: 9px;
    font-weight: 750;
    justify-content: center;
    letter-spacing: 0.03em;
    line-height: 1;
    padding: 2px 4px;
    text-align: center;
    text-transform: uppercase;
    user-select: none;

    &.warning {
      background: rgba(245, 158, 11, 0.2);
      border: 0.5px solid rgba(245, 158, 11, 0.4);
      color: #fbbf24;
    }

    &.error {
      background: rgba(239, 68, 68, 0.2);
      border: 0.5px solid rgba(239, 68, 68, 0.4);
      color: #f87171;
    }

    &.system {
      background: rgba(148, 163, 184, 0.12);
      border: 0.5px solid rgba(148, 163, 184, 0.2);
      color: #8b949e;
    }
  }

  .log-content-text {
    color: #e6edf3;
    min-width: 0;
  }

  &.warning {
    background: rgba(245, 158, 11, 0.04);
    .log-content-text {
      color: #fef08a;
    }
  }

  &.error {
    background: rgba(239, 68, 68, 0.06);
    .log-content-text {
      color: #fca5a5;
    }
  }

  &.system {
    .log-content-text {
      color: #8b949e;
    }
  }

  &.log-empty-state {
    opacity: 0.75;
  }
}

/* Apple Terminal & macOS Pro High-Contrast ANSI Colors Palette */
:deep(.ansi-black-fg) { color: #484f58; }
:deep(.ansi-red-fg) { color: #ff7b72; }
:deep(.ansi-green-fg) { color: #3fb950; }
:deep(.ansi-yellow-fg) { color: #d29922; }
:deep(.ansi-blue-fg) { color: #58a6ff; }
:deep(.ansi-magenta-fg) { color: #bc8cff; }
:deep(.ansi-cyan-fg) { color: #39c5cf; }
:deep(.ansi-white-fg) { color: #e6edf3; }

:deep(.ansi-bright-black-fg) { color: #8b949e; } /* Ensures Angular/Webpack metadata like sizes, hashes are crystal clear */
:deep(.ansi-bright-red-fg) { color: #ffa198; }
:deep(.ansi-bright-green-fg) { color: #56d364; }
:deep(.ansi-bright-yellow-fg) { color: #e3b341; }
:deep(.ansi-bright-blue-fg) { color: #79c0ff; }
:deep(.ansi-bright-magenta-fg) { color: #d2a8ff; }
:deep(.ansi-bright-cyan-fg) { color: #56d4dd; }
:deep(.ansi-bright-white-fg) { color: #ffffff; }

:deep(.ansi-black-bg) { background-color: #21262d; }
:deep(.ansi-red-bg) { background-color: rgba(248, 81, 73, 0.22); }
:deep(.ansi-green-bg) { background-color: rgba(46, 160, 67, 0.22); }
:deep(.ansi-yellow-bg) { background-color: rgba(187, 128, 9, 0.22); }
:deep(.ansi-blue-bg) { background-color: rgba(56, 139, 253, 0.22); }
:deep(.ansi-magenta-bg) { background-color: rgba(163, 113, 247, 0.22); }
:deep(.ansi-cyan-bg) { background-color: rgba(57, 197, 207, 0.22); }
:deep(.ansi-white-bg) { background-color: rgba(240, 246, 252, 0.22); }

/* Empty Placeholder */
.process-empty {
  align-content: center;
  color: #8b949e;
  display: grid;
  gap: 10px;
  justify-content: center;
  min-height: 0;
  padding: 32px;
  text-align: center;

  .empty-icon {
    font-size: 36px;
    margin: 0 auto;
    opacity: 0.5;
  }

  strong {
    color: #f0f3f6;
    font-size: 15px;
  }

  p {
    font-size: 12px;
    line-height: 1.5;
    margin: 0;
    max-width: 480px;
  }
}

/* macOS Terminal Footer & Status Bar */
.process-log-dialog__footer {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-surface-2) 90%, var(--lumina-surface-1));
  border-top: 1px solid var(--lumina-card-border);
  display: flex;
  gap: 12px;
  justify-content: space-between;
  min-height: 40px;
  padding: 6px 14px;
}

.footer-meta {
  align-items: center;
  display: inline-flex;
  gap: 8px;
  font-size: 11.5px;
  line-height: 1;
  color: var(--lumina-text-secondary);
}

.footer-state-indicator {
  align-items: center;
  display: inline-flex;
  font-weight: 600;
  gap: 6px;
  line-height: 1;

  .dot {
    background: currentColor;
    border-radius: 50%;
    display: inline-block;
    flex-shrink: 0;
    height: 6px;
    width: 6px;
  }

  .state-text {
    line-height: 1;
  }

  &.running {
    color: var(--lumina-success);

    .dot {
      animation: pulse-dot 1.8s infinite cubic-bezier(0.4, 0, 0.6, 1);
    }
  }

  &.succeeded {
    color: var(--lumina-success);
  }

  &.failed {
    color: var(--lumina-danger);
  }
}

.footer-actions {
  align-items: center;
  display: flex;
  gap: 6px;
}

.mac-action-btn {
  align-items: center;
  background: var(--lumina-surface-1);
  border: 1px solid var(--lumina-card-border);
  border-radius: 6px;
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: inline-flex;
  font-size: 11px;
  font-weight: 500;
  gap: 5px;
  height: 26px;
  padding: 0 9px;
  transition: all 0.15s ease;

  &:hover:not(:disabled) {
    background: var(--lumina-button-secondary-hover);
    color: var(--lumina-text);
  }

  &:disabled {
    cursor: not-allowed;
    opacity: 0.45;
  }

  &.active {
    background: color-mix(in srgb, var(--lumina-primary) 12%, var(--lumina-surface-1));
    border-color: color-mix(in srgb, var(--lumina-primary) 42%, var(--lumina-card-border));
    color: var(--lumina-primary);
    font-weight: 600;
  }

  &.copied {
    background: color-mix(in srgb, var(--lumina-success) 14%, var(--lumina-surface-1));
    border-color: color-mix(in srgb, var(--lumina-success) 42%, var(--lumina-card-border));
    color: var(--lumina-success);
  }
}

@media (max-width: 920px) {
  .process-log-dialog__header {
    align-items: stretch;
    grid-template-columns: 1fr;
    padding-right: 48px;
  }
}
</style>
