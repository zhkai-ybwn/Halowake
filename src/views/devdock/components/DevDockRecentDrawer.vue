<template>
  <WorkbenchDrawer
    v-if="show"
    fixed
    size="narrow"
    :title="t('devdock.recentCommands.title')"
    :description="t('devdock.recentCommands.description')"
    :close-label="t('common.dismiss')"
    @close="emit('close')"
  >
    <section v-if="history.length" class="recent-command-list">
      <div class="history-actions-bar">
        <span class="history-count">{{ t('devdock.processes.recentRuns') }} ({{ history.length }})</span>
        <button class="clear-history-btn" type="button" @click="emit('clearHistory')">
          {{ t('common.clear') || '清空' }}
        </button>
      </div>

      <article
        v-for="item in history"
        :key="item.id"
        class="recent-command-row"
        :class="item.status"
      >
        <div class="command-info">
          <div class="command-title-row">
            <strong>{{ item.projectName }} · {{ item.commandName }}</strong>
            <span class="status-badge" :class="item.status">
              {{ formatStatus(item.status) }}
            </span>
          </div>
          <div class="command-meta">
            <span>{{ formatTime(item.startedAt) }}</span>
            <span v-if="item.durationMs > 0">· {{ formatDuration(item.durationMs) }}</span>
            <span v-if="item.exitCode !== undefined && item.exitCode !== null">· {{ t('devdock.processes.exitCodeLabel') }}: {{ item.exitCode }}</span>
          </div>
          <p v-if="item.lastLogLine && item.status === 'failed'" class="error-snippet">
            {{ item.lastLogLine }}
          </p>
        </div>
        <button
          class="script-run-btn"
          type="button"
          :class="{ running: isRunning(item) }"
          :disabled="isRunning(item)"
          @click="emit('startCommand', item)"
        >
          {{ isRunning(item) ? t('devdock.actions.running') : t('devdock.actions.run') }}
        </button>
      </article>
    </section>
    <section v-else class="process-empty">
      <strong>{{ t('devdock.recentCommands.emptyTitle') }}</strong>
      <p>{{ t('devdock.recentCommands.emptyDescription') }}</p>
    </section>
  </WorkbenchDrawer>
</template>

<script setup lang="ts">
import WorkbenchDrawer from '@/components/workbench/WorkbenchDrawer.vue'
import { useLocale } from '@/hooks/useLocale'
import type { DevDockRunHistoryRecord } from '@/services/project/project-service'

defineProps<{
  history: DevDockRunHistoryRecord[]
  isRunning: (item: DevDockRunHistoryRecord) => boolean
  show: boolean
}>()

const emit = defineEmits<{
  close: []
  startCommand: [item: DevDockRunHistoryRecord]
  clearHistory: []
}>()

const { t } = useLocale()

function formatStatus(status: string) {
  switch (status) {
    case 'succeeded':
      return t('devdock.processes.statusSucceeded')
    case 'failed':
      return t('devdock.processes.statusFailed')
    case 'stopped':
    case 'exited':
      return t('devdock.processes.statusStopped')
    case 'running':
      return t('devdock.processes.statusRunning')
    default:
      return status
  }
}

function formatTime(timestamp: number) {
  if (!timestamp) return ''
  const date = new Date(timestamp)
  const pad = (n: number) => n.toString().padStart(2, '0')
  return `${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`
}

function formatDuration(ms: number) {
  if (ms < 1000) return `${ms}ms`
  const seconds = (ms / 1000).toFixed(1)
  if (ms < 60000) return `${seconds}s`
  const minutes = Math.floor(ms / 60000)
  const remainingSec = Math.round((ms % 60000) / 1000)
  return `${minutes}m ${remainingSec}s`
}
</script>

<style scoped lang="scss">
.recent-command-list {
  align-content: start;
  display: grid;
  gap: 8px;
  grid-auto-rows: min-content;
  min-height: 0;
  overflow: auto;
  padding: 8px;
}

.history-actions-bar {
  align-items: center;
  display: flex;
  justify-content: space-between;
  padding: 0 4px 4px;

  .history-count {
    color: var(--lumina-text-secondary);
    font-size: 12px;
    font-weight: 500;
  }

  .clear-history-btn {
    background: transparent;
    border: 0;
    color: var(--lumina-text-secondary);
    cursor: pointer;
    font-size: 12px;
    padding: 2px 6px;

    &:hover {
      color: var(--lumina-danger);
    }
  }
}

.recent-command-row {
  align-items: center;
  border: 1px solid var(--lumina-card-border);
  border-radius: var(--lumina-radius-sm);
  display: grid;
  gap: 8px;
  grid-template-columns: minmax(0, 1fr) 76px;
  min-height: 52px;
  padding: 8px 10px;

  &.failed {
    border-color: color-mix(in srgb, var(--lumina-danger) 30%, var(--lumina-card-border));
  }

  .command-info {
    display: grid;
    gap: 4px;
    min-width: 0;
  }

  .command-title-row {
    align-items: center;
    display: flex;
    gap: 6px;
    min-width: 0;

    strong {
      font-size: 12px;
      min-width: 0;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
  }

  .status-badge {
    border-radius: 4px;
    font-size: 10px;
    font-weight: 500;
    padding: 1px 5px;
    white-space: nowrap;

    &.succeeded {
      background: color-mix(in srgb, var(--lumina-success) 15%, transparent);
      color: var(--lumina-success);
    }

    &.failed {
      background: color-mix(in srgb, var(--lumina-danger) 15%, transparent);
      color: var(--lumina-danger);
    }

    &.stopped,
    &.exited {
      background: var(--lumina-surface-2);
      color: var(--lumina-text-secondary);
    }

    &.running {
      background: color-mix(in srgb, var(--lumina-primary) 15%, transparent);
      color: var(--lumina-primary);
    }
  }

  .command-meta {
    color: var(--lumina-text-secondary);
    font-size: 11px;
  }

  .error-snippet {
    color: var(--lumina-danger);
    font-family: var(--lumina-font-mono);
    font-size: 11px;
    line-height: 1.3;
    margin: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .script-run-btn {
    background: var(--lumina-button-secondary-bg);
    border: 1px solid var(--lumina-card-border);
    border-radius: var(--lumina-radius-sm);
    color: var(--lumina-text-secondary);
    cursor: pointer;
    height: 26px;

    &:hover {
      background: var(--lumina-button-secondary-hover);
      color: var(--lumina-text);
    }

    &.running {
      background: color-mix(in srgb, var(--lumina-success) 12%, var(--lumina-surface-1));
      border-color: color-mix(in srgb, var(--lumina-success) 42%, var(--lumina-card-border));
      color: var(--lumina-success);
      cursor: not-allowed;
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
</style>
