<template>
  <NModal
    :show="show"
    class="process-log-modal"
    :auto-focus="false"
    :mask-closable="true"
    :trap-focus="false"
    @update:show="handleShowUpdate"
    @after-leave="$emit('afterLeave')"
  >
    <WorkbenchModalPanel size="log" :close-label="t('common.dismiss')" @close="$emit('close')">
      <section class="process-log-dialog">
        <header class="process-log-dialog__header">
          <div>
            <h3>{{ logs?.process.projectName ?? '' }} · {{ logs?.process.scriptName ?? '' }}</h3>
            <p>{{ description }}</p>
          </div>
          <div class="process-log-controls">
            <div class="process-log-search" @click="focusSearch">
              <Icon icon="solar:magnifer-linear" />
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
              <span>{{ t('devdock.processes.logMatches', { count: visibleLines.length }) }}</span>
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
                <span>{{ option.label }}</span>
                <strong>{{ option.count }}</strong>
              </button>
            </div>
          </div>
        </header>
        <section v-if="logs" ref="viewportRef" class="process-log-list wb-log" aria-live="polite">
          <pre
            v-for="line in visibleLines"
            :key="`${line.timestamp}:${line.stream}:${line.text}`"
            class="wb-log-line"
            :class="[line.stream, line.level]"
          ><span class="wb-log-level">{{ line.level }}</span><span class="wb-log-stream">{{ line.showStream ? line.stream : '' }}</span><span v-html="renderLogLine(line.text)" /></pre>
          <pre
            v-if="!visibleLines.length"
            class="wb-log-line log-pending"
          ><span class="wb-log-level">WAIT</span><span class="wb-log-stream"></span><span>{{ search || levelFilter !== 'all' ? t('devdock.processes.noLogMatches') : t('devdock.processes.waitingLogs') }}</span></pre>
        </section>
        <section v-else class="process-empty">
          <strong>{{ t('devdock.processes.emptyLogsTitle') }}</strong>
          <p>{{ t('devdock.processes.emptyLogsDescription') }}</p>
        </section>
      </section>
    </WorkbenchModalPanel>
  </NModal>
</template>

<script setup lang="ts">
import { AnsiUp } from 'ansi_up'
import { computed, nextTick, ref, watch } from 'vue'
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
const ansiUp = new AnsiUp()
ansiUp.escape_html = true

const description = computed(() => {
  if (!props.logs) return ''
  const ports = props.logs.process.ports
  return ports.length
    ? `${props.logs.process.command} · ${t('devdock.processes.ports', { ports: ports.join(', ') })}`
    : props.logs.process.command
})

const classifiedLines = computed(() =>
  (props.logs?.lines.filter(line => line.text.trim()) ?? []).map(line => ({
    ...line,
    level: classifyLogLevel(line.stream, line.text),
  }))
)

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

watch(
  () => props.show,
  show => {
    if (show) {
      search.value = ''
      levelFilter.value = 'all'
      scrollToBottom()
    }
  }
)

watch(
  () => props.logs?.lines.length,
  () => {
    if (!search.value) {
      scrollToBottom()
    }
  }
)

watch([search, levelFilter], ([searchValue, levelValue]) => {
  if (searchValue || levelValue !== 'all') {
    scrollToTop()
  } else {
    scrollToBottom()
  }
})

function handleShowUpdate(show: boolean) {
  if (!show) {
    emit('close')
  }
}

function focusSearch() {
  searchInputRef.value?.focus()
}

function renderLogLine(text: string) {
  return ansiUp.ansi_to_html(text)
}

function classifyLogLevel(stream: 'stdout' | 'stderr', text: string): LogLevel {
  const content = text.replace(/\u001B\[[0-?]*[ -/]*[@-~]/g, '').toLowerCase()
  if (/\b(error|err|failed|failure|fatal|exception|panic)\b/.test(content)) return 'error'
  if (/\b(warn|warning|deprecated)\b/.test(content)) return 'warning'
  return stream === 'stderr' ? 'error' : 'info'
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
  grid-template-rows: auto minmax(0, 1fr);
  height: 100%;
  min-height: 0;
}

.process-log-dialog__header {
  align-items: center;
  background: var(--lumina-surface-2);
  border-bottom: 1px solid var(--lumina-card-border);
  display: grid;
  gap: 12px;
  grid-template-columns: minmax(0, 1fr) minmax(300px, 420px);
  min-height: 70px;
  padding: 12px 54px 12px 16px;

  h3,
  p {
    margin: 0;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  h3 {
    font-size: 15px;
    line-height: 1.3;
  }

  p {
    color: var(--lumina-text-secondary);
    font-family: SFMono-Regular, Consolas, 'Liberation Mono', Menlo, monospace;
    font-size: 11px;
    line-height: 1.45;
    margin-top: 4px;
  }
}

.process-log-controls {
  display: grid;
  gap: 8px;
}

.process-log-search {
  align-items: center;
  background: var(--lumina-surface-1);
  border: 1px solid var(--lumina-card-border);
  border-radius: var(--lumina-radius-sm);
  display: grid;
  gap: 8px;
  grid-template-columns: 16px minmax(0, 1fr) auto;
  height: 34px;
  padding: 0 10px;
  pointer-events: auto;
  position: relative;
  z-index: 4;

  svg {
    color: var(--lumina-text-secondary);
    height: 15px;
    width: 15px;
  }

  input {
    background: transparent;
    border: 0;
    color: var(--lumina-text);
    font-size: 12px;
    min-width: 0;
    outline: none;
    pointer-events: auto;
    position: relative;
    z-index: 1;
  }

  span {
    color: var(--lumina-text-secondary);
    font-size: 11px;
    white-space: nowrap;
  }

  &:focus-within {
    border-color: var(--lumina-primary);
    box-shadow: 0 0 0 3px var(--lumina-accent-ring);
  }
}

.process-log-filters {
  display: grid;
  gap: 5px;
  grid-template-columns: repeat(4, minmax(0, 1fr));
}

.process-log-filter {
  align-items: center;
  background: var(--lumina-surface-1);
  border: 1px solid var(--lumina-card-border);
  border-radius: var(--lumina-radius-sm);
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: flex;
  font-size: 10px;
  gap: 5px;
  height: 28px;
  justify-content: center;
  min-width: 0;
  padding: 0 6px;

  strong {
    color: inherit;
    font-size: 10px;
  }

  &:hover {
    background: var(--lumina-button-secondary-hover);
    color: var(--lumina-text);
  }

  &.active {
    background: color-mix(in srgb, var(--lumina-primary) 12%, var(--lumina-surface-1));
    border-color: color-mix(in srgb, var(--lumina-primary) 52%, var(--lumina-card-border));
    color: var(--lumina-primary);
  }

  &.warning.active {
    background: color-mix(in srgb, #e5a100 14%, var(--lumina-surface-1));
    border-color: color-mix(in srgb, #e5a100 52%, var(--lumina-card-border));
    color: #d99500;
  }

  &.error.active {
    background: color-mix(in srgb, var(--lumina-danger) 12%, var(--lumina-surface-1));
    border-color: color-mix(in srgb, var(--lumina-danger) 52%, var(--lumina-card-border));
    color: var(--lumina-danger);
  }
}

.process-log-list {
  padding: 12px 14px 14px;

  .wb-log-line {
    font-size: 13px;
    grid-template-columns: 54px 48px minmax(0, 1fr);
    line-height: 1.4;
    min-height: 19px;
  }

  .wb-log-stream {
    font-size: 10px;
    line-height: 1.42;
  }

  .wb-log-level {
    color: var(--lumina-text-secondary);
    font-size: 10px;
    font-weight: 700;
    line-height: 1.42;
    text-transform: uppercase;
  }

  .warning .wb-log-level {
    color: #d99500;
  }

  .error {
    color: #ffb4a8;

    .wb-log-level {
      color: var(--lumina-danger);
    }
  }
}

.process-empty {
  align-content: center;
  color: var(--lumina-text-secondary);
  display: grid;
  gap: 10px;
  justify-content: center;
  min-height: 0;
  padding: 24px;
  text-align: center;

  strong {
    color: var(--lumina-text);
    font-size: 15px;
  }

  p {
    line-height: 1.55;
    margin: 0;
    max-width: 520px;
  }
}

@media (max-width: 920px) {
  .process-log-dialog__header {
    align-items: stretch;
    grid-template-columns: 1fr;
  }
}
</style>
