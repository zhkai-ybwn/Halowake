<template>
  <aside class="process-panel">
    <header class="panel-header">
      <div class="panel-header-title">
        <span>{{ t('devdock.processes.title') }}</span>
        <strong>{{ t('devdock.processes.subtitle') }}</strong>
      </div>
      <div class="panel-header-actions">
        <button
          v-if="completedCount > 0"
          class="clear-completed-btn"
          type="button"
          :title="t('devdock.processes.clearCompleted')"
          @click="$emit('clearCompleted')"
        >
          <Icon icon="solar:trash-bin-trash-linear" />
          <span>{{ t('devdock.processes.clearCompleted') }}</span>
        </button>
        <button class="inspector-close" type="button" :aria-label="t('common.close')" @click="$emit('close')">
          <span class="close-glyph" aria-hidden="true">×</span>
        </button>
      </div>
    </header>

    <!-- Filter Strip -->
    <div v-if="processes.length" class="process-filter-bar">
      <div class="process-filter-segmented" role="tablist">
        <button
          type="button"
          :class="{ active: activeFilter === 'all' }"
          @click="activeFilter = 'all'"
        >
          <span>{{ t('devdock.processes.filterAll') }}</span>
          <b>{{ processes.length }}</b>
        </button>
        <button
          type="button"
          :class="{ active: activeFilter === 'active' }"
          @click="activeFilter = 'active'"
        >
          <span>{{ t('devdock.processes.filterActive') }}</span>
          <b>{{ activeCount }}</b>
        </button>
        <button
          type="button"
          :class="{ active: activeFilter === 'completed' }"
          @click="activeFilter = 'completed'"
        >
          <span>{{ t('devdock.processes.filterCompleted') }}</span>
          <b>{{ completedCount }}</b>
        </button>
      </div>
    </div>

    <section v-if="!processes.length" class="process-empty">
      <strong>{{ t('devdock.processes.emptyTitle') }}</strong>
      <p>{{ t('devdock.processes.emptyDescription') }}</p>
    </section>
    <section v-else-if="!filteredProcesses.length" class="process-empty compact">
      <p>{{ emptyFilterMessage }}</p>
    </section>
    <section v-else class="process-list">
      <section v-for="group in runGroups" :key="group.state" class="run-group">
        <div class="run-group-header">
          <h4>{{ group.state === 'active' ? t('devdock.processes.activeRuns') : t('devdock.processes.recentRuns') }}</h4>
          <span class="run-group-count">{{ group.runs.length }}</span>
        </div>

        <article
          v-for="process in group.runs"
          :key="process.id"
          class="process-row"
          :class="[process.status.state, { 'is-completed': isCompleted(process) }]"
        >
          <div class="process-row-head">
            <div class="process-row-main">
              <div class="process-title-row">
                <strong class="process-name">{{ process.projectName }} · {{ process.commandName || process.scriptName }}</strong>
                <span class="status-badge" :class="process.status.state">
                  <i class="status-dot"></i>
                  {{ processStatusLabel(process) }}
                </span>
              </div>
              <span class="process-meta">
                {{ t('devdock.processes.pid', { pid: process.pid }) }}
                <template v-if="process.ports.length"> · {{ t('devdock.processes.ports', { ports: process.ports.join(', ') }) }}</template>
              </span>
            </div>

            <div class="process-head-actions">
              <button
                v-if="!isCompleted(process)"
                class="process-open-btn"
                type="button"
                :disabled="!processUrl(process)"
                @click="$emit('openUrl', process)"
              >
                {{ t('devdock.actions.open') }}
              </button>
              <button
                v-else
                class="process-dismiss-btn"
                type="button"
                :title="t('devdock.processes.dismiss')"
                @click="$emit('dismiss', process.id)"
              >
                <span class="close-glyph" aria-hidden="true">×</span>
              </button>
            </div>
          </div>

          <div class="process-link-row" :class="{ disabled: isCompleted(process) && !processUrl(process) }">
            <code>{{ processUrl(process) || process.commandPreview || process.command }}</code>
            <button type="button" :disabled="!processUrl(process)" @click="$emit('copyUrl', process)">
              {{ t('devdock.actions.copy') }}
            </button>
          </div>

          <p v-if="process.warning" class="process-warning">{{ process.warning }}</p>
          <p v-else-if="process.status.state === 'failed' && process.lastLogLine" class="process-failure">{{ process.lastLogLine }}</p>

          <div class="process-actions">
            <button type="button" class="btn-log" @click="$emit('openLogs', process.id)">
              {{ t('devdock.actions.logs') }}
            </button>

            <button
              type="button"
              class="btn-restart"
              :disabled="isBusy(process.id)"
              @click="$emit('restart', process.id)"
            >
              {{ !isCompleted(process) ? t('devdock.actions.restart') : t('devdock.actions.rerunTask') }}
            </button>

            <button
              v-if="!isCompleted(process)"
              class="btn-stop danger"
              type="button"
              :disabled="process.status.state !== 'running' || isBusy(process.id)"
              @click="$emit('stop', process.id)"
            >
              {{ t('devdock.actions.stop') }}
            </button>

            <button
              v-else
              class="btn-dismiss"
              type="button"
              @click="$emit('dismiss', process.id)"
            >
              {{ t('devdock.processes.dismiss') }}
            </button>
          </div>
        </article>
      </section>
    </section>
  </aside>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useLocale } from '@/hooks/useLocale'
import type { ProjectProcessSnapshot } from '@/services/project/project-service'

type ProcessFilter = 'all' | 'active' | 'completed'

const props = defineProps<{
  isBusy: (processId: string) => boolean
  processStatusLabel: (process: ProjectProcessSnapshot) => string
  processUrl: (process: ProjectProcessSnapshot) => string
  processes: ProjectProcessSnapshot[]
}>()

defineEmits<{
  (e: 'copyUrl', process: ProjectProcessSnapshot): void
  (e: 'openLogs', processId: string): void
  (e: 'openUrl', process: ProjectProcessSnapshot): void
  (e: 'restart', processId: string): void
  (e: 'stop', processId: string): void
  (e: 'dismiss', processId: string): void
  (e: 'clearCompleted'): void
  (e: 'close'): void
}>()

const { t } = useLocale()
const activeFilter = ref<ProcessFilter>('all')

function isCompleted(process: ProjectProcessSnapshot) {
  return process.status.state !== 'running' && process.status.state !== 'starting'
}

const activeCount = computed(() => props.processes.filter(p => !isCompleted(p)).length)
const completedCount = computed(() => props.processes.filter(p => isCompleted(p)).length)

const filteredProcesses = computed(() => {
  if (activeFilter.value === 'active') {
    return props.processes.filter(p => !isCompleted(p))
  }
  if (activeFilter.value === 'completed') {
    return props.processes.filter(p => isCompleted(p))
  }
  return props.processes
})

const emptyFilterMessage = computed(() => {
  if (activeFilter.value === 'active') return t('devdock.processes.emptyTitle')
  return t('devdock.processes.completedWithoutLogs') || '暂无已完成的进程记录'
})

const runGroups = computed(() => {
  const list = filteredProcesses.value
  const activeRuns = list.filter(process => !isCompleted(process))
  const recentRuns = list.filter(process => isCompleted(process))

  const groups = []
  if (activeRuns.length) {
    groups.push({ state: 'active', runs: activeRuns })
  }
  if (recentRuns.length) {
    groups.push({ state: 'recent', runs: recentRuns })
  }
  return groups
})
</script>

<style scoped lang="scss">
.process-panel {
  background: var(--lumina-surface-1);
  border: 1px solid var(--lumina-card-border);
  border-block: 0;
  border-radius: 0;
  border-right: 0;
  box-shadow: none;
  display: grid;
  grid-template-rows: auto auto minmax(0, 1fr);
  min-height: 0;
  overflow: hidden;
}

.panel-header {
  align-items: center;
  border-bottom: 1px solid var(--lumina-card-border);
  display: flex;
  justify-content: space-between;
  min-height: 48px;
  padding: 8px 12px;
}

.panel-header-title {
  display: grid;
  gap: 2px;

  span {
    color: var(--lumina-text-secondary);
    font-size: 11px;
  }

  strong {
    font-size: 14px;
    color: var(--lumina-text);
  }
}

.panel-header-actions {
  align-items: center;
  display: flex;
  gap: 6px;
}

.clear-completed-btn {
  align-items: center;
  background: var(--lumina-control-bg);
  border: 0.5px solid var(--lumina-card-border);
  border-radius: var(--lumina-radius-sm);
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: inline-flex;
  font-size: 11px;
  gap: 4px;
  height: 26px;
  padding: 0 8px;
  transition: all 0.15s ease;

  svg {
    font-size: 12px;
  }

  &:hover {
    background: var(--lumina-control-hover);
    color: var(--lumina-text);
  }
}

.inspector-close {
  align-items: center;
  background: transparent;
  border: 0;
  border-radius: var(--lumina-radius-sm);
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: inline-flex;
  height: 26px;
  justify-content: center;
  padding: 0;
  width: 26px;

  &:hover {
    background: var(--lumina-control-hover);
    color: var(--lumina-text);
  }

  .close-glyph {
    font-size: 20px;
    font-weight: 300;
    line-height: 1;
    transform: translateY(-1px);
  }
}

/* Process Filter Bar */
.process-filter-bar {
  background: color-mix(in srgb, var(--lumina-surface-2) 65%, var(--lumina-surface-1));
  border-bottom: 1px solid var(--lumina-card-border);
  padding: 6px 10px;
}

.process-filter-segmented {
  background: var(--lumina-control-bg);
  border: 0.5px solid var(--lumina-card-border);
  border-radius: 6px;
  display: grid;
  gap: 2px;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  padding: 2px;

  button {
    align-items: center;
    background: transparent;
    border: 0;
    border-radius: 4px;
    color: var(--lumina-text-secondary);
    cursor: pointer;
    display: inline-flex;
    font-size: 11px;
    gap: 5px;
    height: 24px;
    justify-content: center;
    line-height: 1;
    padding: 0 6px;
    transition: all 0.15s ease;

    span {
      align-items: center;
      display: inline-flex;
      line-height: 1;
    }

    b {
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
      box-shadow: 0 1px 2px rgba(0, 0, 0, 0.06);
      color: var(--lumina-text);
      font-weight: 600;
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

  &.compact {
    padding: 36px 16px;
    font-size: 12px;
  }
}

.process-list {
  align-content: start;
  display: grid;
  gap: 8px;
  grid-auto-rows: min-content;
  min-height: 0;
  overflow: auto;
  padding: 8px;
}

.run-group {
  display: grid;
  gap: 6px;
}

.run-group-header {
  align-items: center;
  display: flex;
  justify-content: space-between;
  padding: 2px 4px 0;

  h4 {
    color: var(--lumina-text-secondary);
    font-size: 10.5px;
    font-weight: 700;
    margin: 0;
    text-transform: uppercase;
    letter-spacing: 0.02em;
  }

  .run-group-count {
    color: var(--lumina-text-tertiary);
    font-size: 10px;
    font-weight: 600;
  }
}

/* Process Card Layout */
.process-row {
  background: var(--lumina-surface-1);
  border: 1px solid var(--lumina-card-border);
  border-radius: var(--lumina-radius-sm);
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.03);
  display: grid;
  gap: 6px;
  padding: 9px;
  position: relative;
  transition: all 0.15s ease;

  /* Completed / Inactive Process Card Muted Style */
  &.is-completed {
    background: color-mix(in srgb, var(--lumina-surface-2) 60%, var(--lumina-surface-1));
    border-color: color-mix(in srgb, var(--lumina-card-border) 80%, transparent);
    opacity: 0.92;

    .process-name {
      color: var(--lumina-text-secondary);
    }
  }

  &:hover {
    border-color: color-mix(in srgb, var(--lumina-text) 16%, var(--lumina-card-border));
  }
}

.process-row-head {
  align-items: start;
  display: grid;
  gap: 8px;
  grid-template-columns: minmax(0, 1fr) auto;
  min-width: 0;
}

.process-row-main {
  display: grid;
  gap: 3px;
  min-width: 0;
}

.process-title-row {
  align-items: center;
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
  min-width: 0;
}

.process-name {
  color: var(--lumina-text);
  font-size: 12.5px;
  font-weight: 650;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.status-badge {
  align-items: center;
  border-radius: 999px;
  display: inline-flex;
  font-size: 10px;
  font-weight: 600;
  gap: 4px;
  padding: 1px 6px;
  background: var(--lumina-control-bg);
  color: var(--lumina-text-secondary);

  .status-dot {
    background: currentColor;
    border-radius: 50%;
    height: 5px;
    width: 5px;
  }

  &.running,
  &.starting {
    background: color-mix(in srgb, var(--lumina-success) 12%, var(--lumina-surface-1));
    color: var(--lumina-success);

    .status-dot {
      animation: pulse-dot 1.8s infinite cubic-bezier(0.4, 0, 0.6, 1);
    }
  }

  &.succeeded {
    background: color-mix(in srgb, var(--lumina-success) 12%, var(--lumina-surface-1));
    color: var(--lumina-success);
  }

  &.failed {
    background: color-mix(in srgb, var(--lumina-danger) 12%, var(--lumina-surface-1));
    color: var(--lumina-danger);
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

.process-meta {
  color: var(--lumina-text-secondary);
  font-family: var(--lumina-font-mono, SFMono-Regular, Consolas, monospace);
  font-size: 11px;
}

.process-head-actions {
  align-items: center;
  display: flex;
  gap: 4px;
}

.process-open-btn {
  background: color-mix(in srgb, var(--lumina-primary) 12%, var(--lumina-surface-1));
  border: 1px solid color-mix(in srgb, var(--lumina-primary) 46%, var(--lumina-card-border));
  border-radius: var(--lumina-radius-sm);
  color: var(--lumina-primary);
  cursor: pointer;
  font-size: 11px;
  font-weight: 650;
  height: 26px;
  min-width: 48px;
  padding: 0 8px;

  &:hover:not(:disabled) {
    background: color-mix(in srgb, var(--lumina-primary) 18%, var(--lumina-surface-1));
  }

  &:disabled {
    cursor: not-allowed;
    opacity: 0.52;
  }
}

.process-dismiss-btn {
  align-items: center;
  background: transparent;
  border: 0;
  border-radius: var(--lumina-radius-sm);
  color: var(--lumina-text-tertiary);
  cursor: pointer;
  display: inline-flex;
  height: 22px;
  justify-content: center;
  padding: 0;
  width: 22px;

  &:hover {
    background: var(--lumina-control-hover);
    color: var(--lumina-danger);
  }

  .close-glyph {
    font-size: 16px;
    line-height: 1;
  }
}

.process-link-row {
  align-items: center;
  background: var(--lumina-surface-2);
  border: 1px solid color-mix(in srgb, var(--lumina-card-border) 72%, transparent);
  border-radius: var(--lumina-radius-sm);
  display: grid;
  gap: 6px;
  grid-template-columns: minmax(0, 1fr) auto;
  min-height: 26px;
  padding: 2px 4px 2px 8px;

  &.disabled {
    opacity: 0.7;
  }

  code {
    color: var(--lumina-text-secondary);
    font-family: var(--lumina-font-mono, SFMono-Regular, Consolas, monospace);
    font-size: 10.5px;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  button {
    background: transparent;
    border: 0;
    border-radius: var(--lumina-radius-sm);
    color: var(--lumina-text-secondary);
    cursor: pointer;
    font-size: 10.5px;
    height: 20px;
    padding: 0 6px;

    &:hover:not(:disabled) {
      background: var(--lumina-button-secondary-hover);
      color: var(--lumina-text);
    }

    &:disabled {
      cursor: not-allowed;
      opacity: 0.5;
    }
  }
}

.process-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
  justify-content: flex-end;
  margin-top: 2px;

  button {
    background: var(--lumina-button-secondary-bg);
    border: 1px solid var(--lumina-card-border);
    border-radius: var(--lumina-radius-sm);
    color: var(--lumina-text-secondary);
    cursor: pointer;
    font-size: 11px;
    font-weight: 500;
    height: 25px;
    min-width: 44px;
    padding: 0 8px;
    transition: all 0.15s ease;

    &:hover:not(:disabled) {
      background: var(--lumina-button-secondary-hover);
      color: var(--lumina-text);
    }

    &:disabled {
      cursor: not-allowed;
      opacity: 0.56;
    }

    &.danger:hover:not(:disabled) {
      border-color: color-mix(in srgb, var(--lumina-danger) 45%, var(--lumina-card-border));
      color: var(--lumina-danger);
    }

    &.btn-dismiss:hover {
      border-color: color-mix(in srgb, var(--lumina-danger) 35%, var(--lumina-card-border));
      color: var(--lumina-danger);
    }
  }
}

.process-warning {
  color: var(--lumina-warning);
  font-size: 10px;
  line-height: 1.45;
  margin: 0;
}

.process-failure {
  color: var(--lumina-danger);
  font-size: 10px;
  line-height: 1.45;
  margin: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>

