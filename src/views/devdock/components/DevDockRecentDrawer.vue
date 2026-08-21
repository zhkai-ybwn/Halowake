<template>
  <WorkbenchDrawer
    v-if="show"
    fixed
    size="narrow"
    :title="t('devdock.recentCommands.title')"
    :description="t('devdock.recentCommands.description')"
    :close-label="t('common.dismiss')"
    @close="$emit('close')"
  >
    <section v-if="commands.length" class="recent-command-list">
      <article v-for="command in commands" :key="`${command.projectPath}:${command.commandId}`" class="recent-command-row">
        <div>
          <strong>{{ command.projectName }} · {{ command.commandName }}</strong>
        </div>
        <button
          class="script-run-btn"
          type="button"
          :class="{ running: isRunning(command) }"
          :disabled="isRunning(command)"
          @click="$emit('startCommand', command)"
        >
          {{ isRunning(command) ? t('devdock.actions.running') : t('devdock.actions.run') }}
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
import type { RecentCommand } from '../types'

defineProps<{
  commands: RecentCommand[]
  isRunning: (command: RecentCommand) => boolean
  show: boolean
}>()

defineEmits<{
  (e: 'close'): void
  (e: 'startCommand', command: RecentCommand): void
}>()

const { t } = useLocale()
</script>

<style scoped lang="scss">
.recent-command-list {
  align-content: start;
  display: grid;
  gap: 6px;
  grid-auto-rows: min-content;
  min-height: 0;
  overflow: auto;
  padding: 8px;
}

.recent-command-row {
  align-items: center;
  border: 1px solid var(--lumina-card-border);
  border-radius: var(--lumina-radius-sm);
  display: grid;
  gap: 8px;
  grid-template-columns: minmax(0, 1fr) 76px;
  min-height: 44px;
  padding: 6px 8px;

  div {
    display: grid;
    gap: 3px;
    min-width: 0;
  }

  strong {
    font-size: 12px;
    min-width: 0;
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
