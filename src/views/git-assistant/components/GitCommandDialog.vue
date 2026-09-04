<template>
  <div v-if="visible" class="dialog-mask">
    <section class="command-dialog">
      <button
        class="dialog-close"
        type="button"
        :aria-label="closeLabel"
        :disabled="running"
        @click="$emit('close')"
      >
        <span class="close-glyph" aria-hidden="true">×</span>
      </button>
      <header class="dialog-header">
        <div>
          <span class="dialog-kicker">Halowake Git Operation</span>
          <h3>{{ title }}</h3>
          <p>{{ repoPath }}</p>
        </div>
        <div class="status-chip" :class="statusTone">
          <span></span>
          {{ statusLabel }}
        </div>
      </header>

      <section class="progress-summary">
        <div class="progress-summary__heading">
          <strong>{{ currentActivity }}</strong>
          <code>{{ activeCommand || commandSummary }}</code>
        </div>
        <div class="progress-track" :aria-label="currentActivity">
          <div class="progress-bar" :class="[statusTone, { determinate: progressPercent !== null }]" :style="progressStyle"></div>
        </div>
        <div class="progress-details">
          <span v-if="progressPercent !== null">{{ progressPercent }}%</span>
          <span v-if="transfer">{{ transfer }}</span>
          <span>{{ elapsedLabel }}</span>
        </div>
      </section>

      <section class="log-panel" :class="statusTone">
        <div class="log-toolbar">
          <span>{{ t('gitAssistant.gitCommand.output') }}</span>
          <span>{{ elapsedLabel }}</span>
        </div>
        <div ref="logBox" class="log-box">
          <template v-for="(line, idx) in logLines" :key="idx">
            <div v-if="idx > 0" class="log-separator"></div>
            <div class="log-line" :class="`log-${line.type}`">{{ line.text }}</div>
          </template>
          <div v-if="!logLines.length && running" class="log-line log-info">
            {{ t('gitAssistant.gitCommand.waitingOutput') }}
          </div>
        </div>
      </section>

      <footer class="dialog-actions">
        <button
          v-if="nextActionLabel"
          class="action-btn next-action"
          :disabled="running"
          type="button"
          @click="$emit('next-action')"
        >
          {{ nextActionLabel }}
        </button>
        <span v-else class="dialog-actions__spacer"></span>
        <button class="action-btn primary" :disabled="running" type="button" @click="$emit('close')">
          {{ closeLabel }}
        </button>
        <button class="action-btn" disabled type="button">{{ abortLabel }}</button>
      </footer>
    </section>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useLocale } from '@/hooks/useLocale'

const { t } = useLocale()

const props = defineProps<{
  visible: boolean
  title: string
  repoPath: string
  phase: string
  running: boolean
  success: boolean | null
  command: string
  activeCommand: string
  stdout: string
  stderr: string
  message: string
  suggestion: string
  startedAt: number
  finishedAt: number | null
  progressPercent: number | null
  progressPhase: string
  transfer: string
  nextActionLabel: string
  closeLabel: string
  abortLabel: string
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'next-action'): void
}>()

interface LogLine {
  type: 'command' | 'stdout' | 'stderr' | 'message' | 'suggestion'
  text: string
}

const logLines = computed<LogLine[]>(() => {
  const lines: LogLine[] = []
  if (props.command) {
    lines.push(...props.command.split('\n').map(command => ({ type: 'command' as const, text: `> ${command}` })))
  }
  if (props.stdout.trim()) {
    lines.push({ type: 'stdout', text: props.stdout.trim() })
  }
  if (props.stderr.trim()) {
    lines.push({ type: 'stderr', text: props.stderr.trim() })
  }
  if (props.message.trim() && !props.stdout.includes(props.message)) {
    lines.push({ type: 'message', text: props.message.trim() })
  }
  if (props.suggestion.trim()) {
    lines.push({ type: 'suggestion', text: props.suggestion.trim() })
  }
  return lines
})

const statusTone = computed(() => {
  if (props.running) return 'running'
  if (props.success === false) return 'failed'
  if (props.success === true) return 'done'
  return ''
})

const statusLabel = computed(() => {
  if (props.running) return t('gitAssistant.gitCommand.running')
  if (props.success === false) return t('gitAssistant.gitCommand.failed')
  if (props.success === true) return t('gitAssistant.gitCommand.completed')
  return t('gitAssistant.gitCommand.ready')
})

const commandSummary = computed(() => {
  const lastCommand = props.command.split('\n').filter(Boolean).at(-1)
  return lastCommand || props.message || 'preparing'
})

const currentActivity = computed(() => props.progressPhase || props.phase || statusLabel.value)

const ticker = ref(Date.now())
const logBox = ref<HTMLElement | null>(null)
let timerId: number | undefined

function stopTicker() {
  if (timerId === undefined) return
  window.clearInterval(timerId)
  timerId = undefined
}

watch(
  () => [props.visible, props.running] as const,
  ([visible, running]) => {
    stopTicker()
    if (visible && running) {
      ticker.value = Date.now()
      timerId = window.setInterval(() => {
        ticker.value = Date.now()
      }, 1000)
    }
  },
  { immediate: true },
)

onBeforeUnmount(stopTicker)

function handleEscape(event: KeyboardEvent) {
  if (event.key === 'Escape' && props.visible && !props.running) emit('close')
}

onMounted(() => window.addEventListener('keydown', handleEscape))
onBeforeUnmount(() => window.removeEventListener('keydown', handleEscape))

watch(
  () => [props.command, props.stdout, props.stderr] as const,
  () => {
    void nextTick(() => {
      if (logBox.value) logBox.value.scrollTop = logBox.value.scrollHeight
    })
  },
)

function formatDuration(ms: number) {
  const seconds = Math.max(0, Math.round(ms / 1000))
  if (seconds < 60) return `${seconds}s`
  const minutes = Math.floor(seconds / 60)
  const restSeconds = seconds % 60
  return `${minutes}m ${restSeconds}s`
}

const elapsedLabel = computed(() => {
  if (!props.startedAt) return '0s'
  const end = props.finishedAt ?? ticker.value
  return formatDuration(end - props.startedAt)
})

const progressStyle = computed(() => {
  if (props.progressPercent === null) return undefined
  return { transform: `scaleX(${Math.max(0, Math.min(100, props.progressPercent)) / 100})` }
})

</script>

<style scoped lang="scss">
.dialog-mask {
  align-items: center;
  background: var(--lumina-overlay);
  backdrop-filter: blur(2px);
  display: flex;
  inset: 0;
  justify-content: center;
  padding: 24px;
  position: absolute;
  z-index: 40;
}

.command-dialog {
  background: color-mix(in srgb, var(--lumina-surface-elevated) 94%, transparent);
  backdrop-filter: saturate(180%) blur(24px);
  border: 0.5px solid var(--lumina-separator-strong);
  border-radius: 12px;
  box-shadow: 0 0 0 0.5px color-mix(in srgb, var(--lumina-text) 8%, transparent), 0 2px 8px rgb(0 0 0 / 10%), 0 18px 52px rgb(0 0 0 / 18%);
  display: grid;
  gap: 14px;
  grid-template-rows: auto auto minmax(240px, 1fr) auto;
  height: min(680px, 86vh);
  max-height: min(680px, 86vh);
  overflow: hidden;
  position: relative;
  width: min(880px, 92vw);
}

.dialog-close {
  align-items: center;
  background: transparent;
  border: 0;
  border-radius: var(--lumina-radius-sm);
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: inline-flex;
  height: 28px;
  justify-content: center;
  padding: 0;
  position: absolute;
  right: 12px;
  top: 12px;
  width: 28px;
  z-index: 3;

  &:hover:not(:disabled) {
    background: var(--lumina-button-secondary-hover);
    color: var(--lumina-text);
  }

  &:focus-visible {
    box-shadow: 0 0 0 3px var(--lumina-accent-ring);
    outline: none;
  }

  &:disabled {
    cursor: not-allowed;
    opacity: 0.45;
  }

  .close-glyph {
    font-size: 20px;
    font-weight: 300;
    line-height: 1;
    transform: translateY(-1px);
  }
}

.dialog-header {
  align-items: flex-start;
  border-bottom: 0.5px solid var(--lumina-separator);
  display: flex;
  gap: 14px;
  justify-content: space-between;
  padding: 16px 56px 16px 18px;

  .dialog-kicker {
    color: var(--lumina-primary);
    display: block;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.04em;
    margin-bottom: 4px;
    text-transform: uppercase;
  }

  h3 {
    font-size: 18px;
    letter-spacing: 0;
    margin: 0 0 4px;
  }

  p {
    color: var(--lumina-text-secondary);
    font-size: 12px;
    margin: 0;
  }
}

.status-chip {
  align-items: center;
  border: 0.5px solid var(--lumina-separator);
  border-radius: 7px;
  color: var(--lumina-text-secondary);
  display: inline-flex;
  font-size: 12px;
  gap: 7px;
  height: 30px;
  padding: 0 10px;

  span {
    background: currentcolor;
    border-radius: 999px;
    height: 7px;
    width: 7px;
  }

  &.running {
    background: color-mix(in srgb, var(--lumina-primary) 8%, transparent);
    border-color: color-mix(in srgb, var(--lumina-primary) 28%, transparent);
    color: var(--lumina-primary);
  }

  &.done {
    background: color-mix(in srgb, var(--lumina-success) 10%, transparent);
    border-color: color-mix(in srgb, var(--lumina-success) 28%, transparent);
    color: var(--lumina-success);
  }

  &.failed {
    background: color-mix(in srgb, var(--lumina-danger) 9%, transparent);
    border-color: color-mix(in srgb, var(--lumina-danger) 28%, transparent);
    color: var(--lumina-danger);
  }
}

.progress-summary {
  display: grid;
  gap: 8px;
  padding: 0 18px;
}

.progress-summary__heading {
  align-items: baseline;
  display: flex;
  gap: 12px;
  min-width: 0;

  strong {
    color: var(--lumina-text);
    flex: 0 0 auto;
    font-size: 13px;
  }

  code {
    color: var(--lumina-text-secondary);
    font-family: Consolas, 'Liberation Mono', Menlo, monospace;
    font-size: 11px;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
}

.progress-track {
  background: var(--lumina-surface-2);
  border: 0.5px solid var(--lumina-separator);
  border-radius: 999px;
  height: 8px;
  overflow: hidden;
}

.progress-details {
  color: var(--lumina-text-secondary);
  display: flex;
  font-family: Consolas, 'Liberation Mono', Menlo, monospace;
  font-size: 11px;
  gap: 12px;
}

.progress-bar {
  background: var(--lumina-primary);
  height: 100%;
  transform: scaleX(0);
  transform-origin: left center;
  transition: transform 280ms cubic-bezier(0.2, 0.8, 0.2, 1);
  width: 100%;
  will-change: transform;

  &.running {
    animation: none;
  }

  &.determinate {
    animation: none;
  }

  &.failed {
    background: var(--lumina-danger);
  }

  &.done {
    background: var(--lumina-success);
  }
}

.log-box {
  background: transparent;
  border: 0;
  color: var(--lumina-text);
  font-family: Consolas, 'Liberation Mono', Menlo, monospace;
  font-size: 13px;
  line-height: 1.45;
  margin: 0;
  max-height: 320px;
  overflow: auto;
  padding: 10px;
  white-space: pre-wrap;
}

.log-separator {
  height: 8px;
}

.log-line {
  white-space: pre-wrap;
  word-break: break-all;
}

.log-command {
  color: var(--lumina-text-secondary);
}

.log-stdout {
  color: var(--lumina-text);
}

.log-stderr {
  color: var(--lumina-text);
}

.log-message {
  color: var(--lumina-text-secondary);
}

.log-suggestion {
  color: var(--lumina-warning);
}

.log-panel.done {
  .log-message {
    color: var(--lumina-success);
  }
}

.log-panel.failed {
  .log-stderr,
  .log-message {
    color: var(--lumina-danger);
  }
}

.log-panel {
  background: var(--lumina-diff-bg);
  border: 1px solid var(--lumina-card-border);
  border-radius: 10px;
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  margin: 0 18px;
  min-height: 0;
  overflow: hidden;
}

.log-toolbar {
  align-items: center;
  border-bottom: 0.5px solid var(--lumina-separator);
  color: var(--lumina-text-secondary);
  display: flex;
  font-size: 11px;
  justify-content: space-between;
  min-height: 32px;
  padding: 0 10px;

}

.dialog-actions {
  align-items: center;
  display: flex;
  gap: 12px;
  justify-content: flex-end;
  padding: 0 18px 16px;
}

.dialog-actions__spacer {
  flex: 1 1 auto;
}

.action-btn {
  background: var(--lumina-button-secondary-bg);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-sm);
  color: var(--lumina-text);
  cursor: pointer;
  height: 28px;
  padding: 0 12px;

  &:disabled {
    cursor: not-allowed;
    opacity: 0.55;
  }
}

.next-action {
  margin-right: auto;
  min-width: 132px;
}

.primary {
  background: var(--lumina-primary);
  border-color: var(--lumina-primary);
  box-shadow: none;
  color: var(--lumina-on-accent);
}

</style>
