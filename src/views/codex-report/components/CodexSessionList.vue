<template>
  <aside class="session-source-panel">
    <header class="panel-header">
      <div class="panel-title-group">
        <span class="panel-eyebrow">{{ t('codexReport.results') }}</span>
        <h3>{{ t('codexReport.sessions') }}</h3>
      </div>
      <span class="count-pill">{{ t('codexReport.items', { count: listSessions.length }) }}</span>
    </header>

    <!-- macOS Segmented Control -->
    <div class="segmented-control" role="tablist" :aria-label="t('codexReport.sessionFilterLabel')">
      <button
        type="button"
        role="tab"
        :class="{ active: sessionView === 'all' }"
        @click="emit('update:sessionView', 'all')"
      >
        {{ t('codexReport.all', { count: filteredSessions.length }) }}
      </button>
      <button
        type="button"
        role="tab"
        :class="{ active: sessionView === 'included' }"
        @click="emit('update:sessionView', 'included')"
      >
        {{ t('codexReport.included', { count: includedSessions.length }) }}
      </button>
      <button
        type="button"
        role="tab"
        :class="{ active: sessionView === 'excluded' }"
        @click="emit('update:sessionView', 'excluded')"
      >
        {{ t('codexReport.excluded', { count: excludedSessions.length }) }}
      </button>
    </div>

    <!-- Session List / Empty states -->
    <div class="session-list-container">
      <WorkbenchEmptyState
        v-if="!loading && !sessions.length"
        icon="solar:notes-linear"
        :title="t('codexReport.emptyTitle')"
        :description="t('codexReport.emptyLoad')"
      >
        <template #actions>
          <WorkbenchButton variant="primary" @click="emit('load')">
            <Icon icon="solar:refresh-linear" />
            {{ t('codexReport.load') }}
          </WorkbenchButton>
        </template>
      </WorkbenchEmptyState>

      <WorkbenchEmptyState
        v-else-if="!loading && !listSessions.length"
        icon="solar:filter-linear"
        :title="t('codexReport.emptyFilterTitle')"
        :description="t('codexReport.emptyFilter')"
      >
        <template
          v-if="selectedProjects.length && selectedProjects.length < allProjectCount"
          #actions
        >
          <WorkbenchButton variant="secondary" @click="emit('reset-projects')">
            <Icon icon="solar:folder-with-files-linear" />
            {{ t('codexReport.showAllProjects') }}
          </WorkbenchButton>
        </template>
      </WorkbenchEmptyState>

      <NCheckboxGroup
        v-else
        :value="includedIds"
        class="session-card-list"
        @update:value="onSelectionChange"
      >
        <label
          v-for="session in listSessions"
          :key="session.id"
          class="session-card"
          :class="{ 'is-included': includedIds.includes(session.id) }"
        >
          <div class="card-checkbox">
            <NCheckbox :value="session.id" />
          </div>
          <div class="card-content">
            <div class="card-topline">
              <div class="card-topline-left">
                <span class="provider-pill" :class="`prov-${session.provider}`">
                  <Icon :icon="getProviderIcon(session.provider)" />
                  {{ getProviderLabel(session.provider) }}
                </span>
                <span class="project-badge">{{ session.projectName || getProjectName(session.cwd) }}</span>
              </div>
              <span class="session-time">{{ formatSessionTime(session.startedAt) }}</span>
            </div>
            <p class="session-snippet">
              {{ session.userMessages[0] || session.assistantMessages[0] || t('codexReport.noFacts') }}
            </p>
            <div class="card-status">
              <span
                class="status-tag"
                :class="includedIds.includes(session.id) ? 'tag-included' : hasWorkContent(session) ? 'tag-excluded' : 'tag-auto-excluded'"
              >
                {{
                  includedIds.includes(session.id)
                    ? t('codexReport.statusIncluded')
                    : hasWorkContent(session)
                      ? t('codexReport.statusExcluded')
                      : t('codexReport.statusAutoExcluded')
                }}
              </span>
            </div>
          </div>
        </label>
      </NCheckboxGroup>
    </div>
  </aside>
</template>

<script setup lang="ts">
import { NCheckbox, NCheckboxGroup } from 'naive-ui'
import WorkbenchButton from '@/components/workbench/WorkbenchButton.vue'
import WorkbenchEmptyState from '@/components/workbench/WorkbenchEmptyState.vue'
import { useLocale } from '@/composables/useLocale'
import type { CodexReportSession } from '@/types/codex-report'
import {
  formatSessionTime,
  getProjectName,
  getProviderIcon,
  getProviderLabel,
  hasWorkContent,
} from '@/utils/codex-report'

defineProps<{
  loading: boolean
  sessions: CodexReportSession[]
  filteredSessions: CodexReportSession[]
  includedSessions: CodexReportSession[]
  excludedSessions: CodexReportSession[]
  listSessions: CodexReportSession[]
  includedIds: string[]
  sessionView: 'all' | 'included' | 'excluded'
  selectedProjects: string[]
  allProjectCount: number
}>()

const emit = defineEmits<{
  (e: 'update:includedIds', value: string[]): void
  (e: 'update:sessionView', value: 'all' | 'included' | 'excluded'): void
  (e: 'load'): void
  (e: 'reset-projects'): void
}>()

const { t } = useLocale()

function onSelectionChange(val: (string | number)[]) {
  emit('update:includedIds', val as string[])
}
</script>

<style scoped lang="scss">
.session-source-panel {
  background: var(--lumina-surface-2);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-md);
  box-shadow: var(--lumina-shadow-sm);
  display: flex;
  flex: 0 0 340px;
  flex-direction: column;
  min-height: 0;
  min-width: 0;
  overflow: hidden;
}

.panel-header {
  align-items: center;
  border-bottom: 0.5px solid var(--lumina-separator);
  display: flex;
  flex: 0 0 auto;
  justify-content: space-between;
  padding: 8px 12px;
}

.panel-title-group {
  display: flex;
  flex-direction: column;
  gap: 1px;

  .panel-eyebrow {
    color: var(--lumina-text-tertiary);
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.05em;
    text-transform: uppercase;
  }

  h3 {
    color: var(--lumina-text);
    font-size: 13px;
    font-weight: 600;
    margin: 0;
  }
}

.count-pill {
  background: var(--lumina-surface-3);
  border-radius: 10px;
  color: var(--lumina-text-secondary);
  font-family: var(--lumina-font-mono, monospace);
  font-size: 10.5px;
  font-weight: 500;
  padding: 2px 7px;
}

.segmented-control {
  background: var(--lumina-surface-3);
  border-radius: var(--lumina-radius-xs);
  display: flex;
  gap: 2px;
  margin: 6px 12px;
  padding: 2px;

  button {
    background: transparent;
    border: none;
    border-radius: var(--lumina-radius-xs);
    color: var(--lumina-text-secondary);
    cursor: pointer;
    flex: 1;
    font-size: 11px;
    font-weight: 500;
    padding: 3px 0;
    text-align: center;
    transition: all var(--lumina-duration-fast) var(--lumina-ease-out);

    &:hover {
      color: var(--lumina-text);
    }

    &.active {
      background: var(--lumina-surface-elevated);
      box-shadow: var(--lumina-shadow-xs);
      color: var(--lumina-primary);
      font-weight: 600;
    }
  }
}

.session-list-container {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-height: 0;
  overflow-y: auto;
  padding: 4px 8px 8px;
}

.session-card-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  width: 100%;
}

.session-card {
  align-items: flex-start;
  background: var(--lumina-surface-elevated);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-sm);
  cursor: pointer;
  display: flex;
  gap: 8px;
  padding: 8px 10px;
  transition: all var(--lumina-duration-fast) var(--lumina-ease-out);

  &:hover {
    background: var(--lumina-surface-3);
    border-color: var(--lumina-separator-strong);
  }

  &.is-included {
    border-left: 2.5px solid var(--lumina-primary);
  }
}

.card-checkbox {
  flex-shrink: 0;
  margin-top: 1px;
}

.card-content {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}

.card-topline {
  align-items: center;
  display: flex;
  justify-content: space-between;
  width: 100%;
}

.card-topline-left {
  align-items: center;
  display: flex;
  gap: 6px;
  max-width: 70%;
}

.provider-pill {
  align-items: center;
  border-radius: var(--lumina-radius-xs);
  display: inline-flex;
  font-size: 10px;
  font-weight: 600;
  gap: 3px;
  padding: 1px 5px;
  white-space: nowrap;

  svg {
    font-size: 11px;
  }

  &.prov-codex {
    background: rgba(16, 185, 129, 0.12);
    color: #10b981;
  }

  &.prov-claude {
    background: rgba(249, 115, 22, 0.12);
    color: #f97316;
  }

  &.prov-antigravity {
    background: rgba(99, 102, 241, 0.12);
    color: #6366f1;
  }

  &.prov-opencode {
    background: rgba(168, 85, 247, 0.12);
    color: #a855f7;
  }
}

.project-badge {
  color: var(--lumina-text);
  font-size: 12px;
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.session-time {
  color: var(--lumina-text-tertiary);
  font-family: var(--lumina-font-mono, monospace);
  font-size: 11px;
}

.session-snippet {
  color: var(--lumina-text-secondary);
  display: -webkit-box;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
  font-size: 11px;
  line-height: 1.45;
  margin: 0;
  overflow: hidden;
}

.card-status {
  display: flex;
  justify-content: flex-end;
  margin-top: 2px;
}

.status-tag {
  border-radius: var(--lumina-radius-xs);
  font-size: 10px;
  font-weight: 500;
  padding: 1px 6px;

  &.tag-included {
    background: color-mix(in srgb, var(--lumina-primary-soft) 60%, var(--lumina-surface-2));
    color: var(--lumina-primary);
  }

  &.tag-excluded {
    background: var(--lumina-surface-3);
    color: var(--lumina-text-tertiary);
  }

  &.tag-auto-excluded {
    background: color-mix(in srgb, var(--lumina-warning) 12%, transparent);
    color: var(--lumina-warning);
  }
}

@media (max-width: 1024px) {
  .session-source-panel {
    flex-basis: 260px;
  }
}
</style>
