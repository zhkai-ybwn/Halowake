<template>
  <NModal
    :show="show"
    class="project-filter-modal"
    :mask-closable="true"
    :auto-focus="false"
    :trap-focus="false"
    @update:show="emit('update:show', $event)"
  >
    <WorkbenchSheet
      size="normal"
      icon="solar:folder-with-files-linear"
      :title="t('codexReport.projectModalTitle')"
      :description="t('codexReport.projectModalDesc')"
      :close-label="t('common.close')"
      @close="emit('update:show', false)"
    >
      <div class="project-modal-body">
        <!-- Search & Action Row -->
        <div class="modal-search-row">
          <NInput
            v-model:value="projectSearch"
            size="small"
            clearable
            :placeholder="t('codexReport.projectSearchPlaceholder')"
            class="project-search-input"
          >
            <template #prefix>
              <Icon icon="solar:magnifer-linear" class="search-prefix-icon" />
            </template>
          </NInput>
          <div class="modal-shortcuts">
            <button type="button" class="shortcut-btn" @click="selectAllProjects">
              {{ t('codexReport.selectAllProjects') }}
            </button>
            <button type="button" class="shortcut-btn" @click="clearSelectedProjects">
              {{ t('codexReport.deselectAllProjects') }}
            </button>
            <button type="button" class="shortcut-btn" @click="invertProjectSelection">
              {{ t('codexReport.invertSelection') }}
            </button>
          </div>
        </div>

        <!-- Project Checkbox List -->
        <div class="modal-project-list">
          <WorkbenchEmptyState
            v-if="!searchedProjectList.length"
            icon="solar:magnifer-linear"
            :title="t('codexReport.noProjectsFound')"
          />
          <NCheckboxGroup v-else :value="selectedProjects" class="project-items-group" @update:value="onSelectionChange">
            <label
              v-for="project in searchedProjectList"
              :key="project.name"
              class="project-select-card"
              :class="{ 'is-checked': selectedProjects.includes(project.name) }"
            >
              <div class="project-card-left">
                <NCheckbox :value="project.name" />
                <div class="project-info">
                  <div class="project-name-row">
                    <span class="project-name">{{ project.name }}</span>
                    <span
                      v-for="prov in (project.providers?.length ? project.providers : (project.provider ? [project.provider] : []))"
                      :key="prov"
                      class="project-prov-badge"
                      :class="`prov-${prov}`"
                    >
                      {{ getProviderLabel(prov) }}
                    </span>
                    <span v-if="project.sessionCount" class="project-count-badge">
                      {{ t('codexReport.sessionsCountBadge', { count: project.sessionCount }) }}
                    </span>
                  </div>
                  <span class="project-path" :title="project.cwd">{{ project.cwd }}</span>
                </div>
              </div>
              <span v-if="project.lastActiveAt" class="project-time">
                {{ formatSessionTime(project.lastActiveAt) }}
              </span>
            </label>
          </NCheckboxGroup>
        </div>
      </div>

      <template #footer>
        <div class="modal-footer-content">
          <span class="modal-selected-summary">
            {{
              t('codexReport.projectSelectedSummary', {
                selected: selectedProjects.length || allProjectNames.length,
                total: allProjectNames.length,
              })
            }}
          </span>
          <div class="modal-footer-actions">
            <WorkbenchButton variant="primary" @click="emit('update:show', false)">
              {{ t('common.confirm') }}
            </WorkbenchButton>
          </div>
        </div>
      </template>
    </WorkbenchSheet>
  </NModal>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { NCheckbox, NCheckboxGroup, NInput, NModal } from 'naive-ui'
import WorkbenchSheet from '@/components/workbench/WorkbenchSheet.vue'
import WorkbenchEmptyState from '@/components/workbench/WorkbenchEmptyState.vue'
import WorkbenchButton from '@/components/workbench/WorkbenchButton.vue'
import { useLocale } from '@/composables/useLocale'
import type { CodexProjectInfo } from '@/types/codex-report'
import { formatSessionTime, getProviderLabel } from '@/utils/codex-report'

const props = defineProps<{
  show: boolean
  projects: CodexProjectInfo[]
  selectedProjects: string[]
}>()

const emit = defineEmits<{
  (e: 'update:show', value: boolean): void
  (e: 'update:selectedProjects', value: string[]): void
}>()

const { t } = useLocale()
const projectSearch = ref('')

const allProjectNames = computed(() => props.projects.map(p => p.name))

const searchedProjectList = computed(() => {
  const query = projectSearch.value.trim().toLowerCase()
  if (!query) return props.projects
  return props.projects.filter(
    p => p.name.toLowerCase().includes(query) || (p.cwd && p.cwd.toLowerCase().includes(query))
  )
})

function onSelectionChange(value: (string | number)[]) {
  emit('update:selectedProjects', value as string[])
}

function selectAllProjects() {
  emit('update:selectedProjects', [...allProjectNames.value])
}

function clearSelectedProjects() {
  emit('update:selectedProjects', [])
}

function invertProjectSelection() {
  const current = new Set(props.selectedProjects)
  emit('update:selectedProjects', allProjectNames.value.filter(name => !current.has(name)))
}
</script>

<style scoped lang="scss">
.project-modal-body {
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-height: 0;
  padding: 12px 16px;
}

.modal-search-row {
  align-items: center;
  display: flex;
  gap: 10px;
  justify-content: space-between;
}

.project-search-input {
  flex: 1;
}

.modal-shortcuts {
  display: flex;
  gap: 6px;
}

.shortcut-btn {
  background: var(--lumina-surface-3);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-xs);
  color: var(--lumina-text-secondary);
  cursor: pointer;
  font-size: 11px;
  font-weight: 500;
  padding: 3px 8px;
  transition: all var(--lumina-duration-fast) var(--lumina-ease-out);

  &:hover {
    background: var(--lumina-surface-elevated);
    color: var(--lumina-primary);
  }
}

.modal-project-list {
  display: flex;
  flex: 1;
  flex-direction: column;
  max-height: 380px;
  min-height: 200px;
  overflow-y: auto;
  padding: 2px;
}

.project-items-group {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.project-select-card {
  align-items: center;
  background: var(--lumina-surface-2);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-sm);
  cursor: pointer;
  display: flex;
  gap: 10px;
  justify-content: space-between;
  padding: 8px 12px;
  transition: all var(--lumina-duration-fast) var(--lumina-ease-out);

  &:hover {
    background: var(--lumina-surface-3);
    border-color: var(--lumina-separator-strong);
  }

  &.is-checked {
    border-left: 2.5px solid var(--lumina-primary);
  }
}

.project-card-left {
  align-items: center;
  display: flex;
  flex: 1;
  gap: 10px;
  min-width: 0;
}

.project-info {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.project-name-row {
  align-items: center;
  display: flex;
  gap: 8px;
}

.project-name {
  color: var(--lumina-text);
  font-size: 12.5px;
  font-weight: 600;
}

.project-prov-badge {
  border-radius: var(--lumina-radius-xs);
  font-size: 9.5px;
  font-weight: 600;
  padding: 1px 5px;

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

.project-count-badge {
  background: color-mix(in srgb, var(--lumina-primary-soft) 60%, var(--lumina-surface-3));
  border-radius: var(--lumina-radius-xs);
  color: var(--lumina-primary);
  font-size: 10.5px;
  font-weight: 600;
  padding: 1px 6px;
}

.project-path {
  color: var(--lumina-text-tertiary);
  font-family: var(--lumina-font-mono, monospace);
  font-size: 10.5px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.project-time {
  color: var(--lumina-text-tertiary);
  font-family: var(--lumina-font-mono, monospace);
  font-size: 11px;
  white-space: nowrap;
}

.modal-footer-content {
  align-items: center;
  display: flex;
  justify-content: space-between;
  width: 100%;
}

.modal-selected-summary {
  color: var(--lumina-text-secondary);
  font-size: 12px;
}

.modal-footer-actions {
  align-items: center;
  display: flex;
  gap: 8px;
}
</style>
