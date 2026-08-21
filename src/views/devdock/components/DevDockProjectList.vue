<template>
  <main class="project-list-panel">
    <header class="panel-header">
      <div class="panel-title">
        <span>{{ t('devdock.projects.title') }}</span>
        <strong>{{ t('devdock.projects.subtitle') }}</strong>
      </div>
      <div class="command-toolbar">
        <label class="command-search">
          <Icon icon="solar:magnifer-linear" />
          <input ref="searchInput" :value="scriptSearch" type="search" :placeholder="t('devdock.scripts.searchPlaceholder')" @input="$emit('update:scriptSearch', ($event.target as HTMLInputElement).value.trim())" />
        </label>
        <select :value="scriptSort" class="command-sort" :aria-label="t('devdock.scripts.sortLabel')" @change="handleSortChange">
          <option value="priority">{{ t('devdock.scripts.sortPriority') }}</option>
          <option value="name">{{ t('devdock.scripts.sortName') }}</option>
          <option value="recent">{{ t('devdock.scripts.sortRecent') }}</option>
        </select>
        <button
          class="pin-mode-btn"
          type="button"
          :class="{ active: pinEditing }"
          :title="pinEditing ? t('devdock.scripts.finishPinning') : t('devdock.scripts.managePinning')"
          :aria-label="pinEditing ? t('devdock.scripts.finishPinning') : t('devdock.scripts.managePinning')"
          :aria-pressed="pinEditing"
          @click="$emit('update:pinEditing', !pinEditing)"
        >
          <Icon :icon="pinEditing ? 'solar:check-circle-linear' : 'solar:pin-linear'" />
        </button>
        <button class="recent-command-btn" type="button" :aria-label="t('devdock.recentCommands.open')" @click="$emit('openRecent')">
          <Icon icon="solar:history-linear" />
          <b>{{ recentCount }}</b>
        </button>
      </div>
    </header>

    <section v-if="!hasProjects" class="empty-page">
      <strong>{{ t('devdock.empty.title') }}</strong>
      <p>{{ t('devdock.empty.description') }}</p>
      <button class="tool-btn primary" type="button" @click="$emit('addProject')">
        {{ t('devdock.actions.addProject') }}
      </button>
    </section>

    <section v-else class="project-list">
      <div v-if="!projects.length" class="project-empty compact">
        {{ t('devdock.scripts.noSearchResults') }}
      </div>
      <article v-for="project in projects" :key="project.path" class="project-row">
        <header class="project-row-summary">
          <div class="project-identity">
            <div v-if="editingAliasPath === project.path" class="project-alias-edit">
              <input
                :ref="el => setAliasInputRef(el as HTMLInputElement | null, project.path)"
                class="project-alias-input"
                type="text"
                :value="project.name"
                :placeholder="t('devdock.project.aliasPlaceholder')"
                @input="$emit('renameProject', project.path, ($event.target as HTMLInputElement).value)"
                @blur="$emit('finishEditAlias', project)"
                @keydown.enter="$emit('finishEditAlias', project)"
                @keydown.escape="$emit('cancelEditAlias')"
                @click.stop
                @mousedown.stop
                @keydown.stop
              />
            </div>
            <template v-else>
              <span class="project-alias-text" :title="project.name || project.path">{{ project.name || t('devdock.project.aliasPlaceholder') }}</span>
            </template>
            <span :title="project.path">{{ project.path }}</span>
          </div>

          <div class="project-row-actions" @click.stop>
            <button class="row-action" type="button" :title="t('devdock.actions.configureCommands')" :aria-label="t('devdock.actions.configureCommands')" @click="$emit('configureCommands', project)">
              <Icon icon="solar:tuning-2-linear" />
            </button>
            <button class="row-action" type="button" :title="t('devdock.actions.rename')" :aria-label="t('devdock.actions.rename')" @click="$emit('startEditAlias', project.path)">
              <Icon icon="solar:pen-2-linear" />
            </button>
            <button class="row-action" type="button" :title="t('devdock.actions.scan')" :aria-label="t('devdock.actions.scan')" :disabled="project.loading" @click="$emit('scanProject', project)">
              <Icon :icon="project.loading ? 'solar:refresh-circle-bold' : 'solar:refresh-linear'" />
            </button>
            <button class="row-action danger" type="button" :title="t('devdock.actions.removeProject')" :aria-label="t('devdock.actions.removeProject')" @click="$emit('removeProject', project.path)">
              <Icon icon="solar:trash-bin-trash-linear" />
            </button>
          </div>
        </header>

        <section v-if="project.error" class="project-error">
          <span>{{ project.error }}</span>
          <button type="button" :aria-label="t('common.dismiss')" @click="$emit('dismissProjectError', project)">
            <span class="close-glyph" aria-hidden="true">×</span>
          </button>
        </section>

        <section class="project-row-details">
          <div v-if="project.loading" class="project-loading-state">
            <Icon icon="solar:refresh-circle-bold" class="loading-spinner" />
            <div class="loading-content">
              <strong>{{ t('devdock.actions.scanning') }}</strong>
              <span>{{ t('devdock.loadingHint') }}</span>
            </div>
          </div>
          <div v-else-if="filteredScripts(project).length" class="script-list">
            <article
              v-for="script in displayedScripts(project)"
              :key="script.id"
              class="script-row"
              :class="{ running: isScriptRunning(project.path, script.id), starting: isScriptStarting(project.path, script.id), 'pin-editing': pinEditing }"
            >
              <span class="script-status-dot" aria-hidden="true"></span>
              <strong :title="`${script.name} · ${script.sourceLabel}`">{{ script.name }}</strong>
              <button
                v-if="pinEditing"
                class="pin-btn"
                type="button"
                :class="{ active: isScriptPinned(project.path, script.id) }"
                :aria-label="isScriptPinned(project.path, script.id) ? t('devdock.actions.unpinScript') : t('devdock.actions.pinScript')"
                @click="$emit('togglePinnedScript', project.path, script.id)"
              >
                <Icon :icon="isScriptPinned(project.path, script.id) ? 'solar:pin-bold' : 'solar:pin-linear'" />
              </button>
              <button
                class="script-run-btn"
                type="button"
                :class="{ stop: isScriptRunning(project.path, script.id) }"
                :title="scriptActionLabel(project.path, script.id)"
                :aria-label="scriptActionLabel(project.path, script.id)"
                :disabled="isScriptStarting(project.path, script.id)"
                @click="$emit('toggleScript', project, script)"
              >
                <Icon :icon="isScriptRunning(project.path, script.id) ? 'solar:stop-bold' : 'solar:play-bold'" />
              </button>
            </article>
            <button
              v-if="!scriptSearch && (hiddenScriptCount(project) > 0 || isProjectCommandsExpanded(project.path))"
              class="more-scripts-btn"
              type="button"
              @click="$emit('toggleProjectCommands', project.path)"
            >
              <Icon :icon="isProjectCommandsExpanded(project.path) ? 'solar:alt-arrow-up-linear' : 'solar:add-circle-linear'" />
              {{ isProjectCommandsExpanded(project.path) ? t('devdock.scripts.collapse') : t('devdock.scripts.more', { count: hiddenScriptCount(project) }) }}
            </button>
          </div>
          <div v-else-if="project.manifest?.candidates?.length" class="project-candidates-guide">
            <div class="candidates-guide-info">
              <div class="guide-icon-badge">
                <Icon icon="solar:lightbulb-bolt-bold" />
              </div>
              <div class="guide-text">
                <strong>{{ t('devdock.candidates.foundTitle', { count: project.manifest.candidates.length }) }}</strong>
                <p>{{ t('devdock.candidates.foundDescription') }}</p>
              </div>
            </div>
            <div class="candidates-preview-tags">
              <span v-for="cand in project.manifest.candidates.slice(0, 4)" :key="cand.suggestedId" class="candidate-tag" :title="cand.reason">
                {{ cand.name }}
              </span>
              <span v-if="project.manifest.candidates.length > 4" class="candidate-tag more">
                +{{ project.manifest.candidates.length - 4 }}
              </span>
            </div>
            <button class="tool-btn primary candidate-action-btn" type="button" @click="$emit('configureCommands', project)">
              <Icon icon="solar:tuning-2-linear" />
              <span>{{ t('devdock.candidates.configureNow') }}</span>
            </button>
          </div>
          <div v-else-if="scriptSearch" class="project-empty compact">
            {{ t('devdock.scripts.noSearchResults') }}
          </div>
          <div v-else class="project-empty-guide">
            <div class="empty-guide-info">
              <Icon icon="solar:folder-open-linear" class="empty-icon" />
              <p>{{ t('devdock.empty.noManifest') }}</p>
            </div>
            <button class="tool-btn secondary empty-action-btn" type="button" @click="$emit('configureCommands', project)">
              <Icon icon="solar:tuning-2-linear" />
              <span>{{ t('devdock.overview.configureCommands') }}</span>
            </button>
          </div>
        </section>
      </article>
    </section>
  </main>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { useLocale } from '@/hooks/useLocale'
import type { ProjectCommand } from '@/services/project/project-service'
import type { DevDockProject, ScriptSort } from '../types'
import { hasPrimaryModifier } from '@/utils/platform-shortcuts'

defineProps<{
  displayedScripts: (project: DevDockProject) => ProjectCommand[]
  editingAliasPath: string | null
  filteredScripts: (project: DevDockProject) => ProjectCommand[]
  hasProjects: boolean
  hiddenScriptCount: (project: DevDockProject) => number
  isProjectCommandsExpanded: (path: string) => boolean
  isScriptPinned: (projectPath: string, scriptName: string) => boolean
  isScriptRunning: (projectPath: string, scriptName: string) => boolean
  isScriptStarting: (projectPath: string, scriptName: string) => boolean
  pinEditing: boolean
  projects: DevDockProject[]
  recentCount: number
  scriptActionLabel: (projectPath: string, scriptName: string) => string
  scriptSearch: string
  scriptSort: ScriptSort
  setAliasInputRef: (el: HTMLInputElement | null, path: string) => void
}>()

const emit = defineEmits<{
  (e: 'addProject'): void
  (e: 'cancelEditAlias'): void
  (e: 'configureCommands', project: DevDockProject): void
  (e: 'dismissProjectError', project: DevDockProject): void
  (e: 'finishEditAlias', project: DevDockProject): void
  (e: 'openRecent'): void
  (e: 'removeProject', path: string): void
  (e: 'renameProject', path: string, name: string): void
  (e: 'scanProject', project: DevDockProject): void
  (e: 'startEditAlias', path: string): void
  (e: 'togglePinnedScript', projectPath: string, scriptName: string): void
  (e: 'toggleProjectCommands', path: string): void
  (e: 'toggleScript', project: DevDockProject, script: ProjectCommand): void
  (e: 'update:pinEditing', value: boolean): void
  (e: 'update:scriptSearch', value: string): void
  (e: 'update:scriptSort', value: ScriptSort): void
}>()

const { t } = useLocale()
const searchInput = ref<HTMLInputElement | null>(null)

function handleFindShortcut(event: KeyboardEvent) {
  if (!hasPrimaryModifier(event) || event.key.toLowerCase() !== 'f') return
  if (!searchInput.value?.offsetParent) return
  event.preventDefault()
  searchInput.value.focus()
  searchInput.value.select()
}

onMounted(() => window.addEventListener('keydown', handleFindShortcut))
onUnmounted(() => window.removeEventListener('keydown', handleFindShortcut))

function handleSortChange(event: Event) {
  emit('update:scriptSort', (event.target as HTMLSelectElement).value as ScriptSort)
}
</script>

<style scoped lang="scss">
.project-list-panel {
  background: var(--lumina-surface-1);
  border: 1px solid var(--lumina-card-border);
  border-radius: var(--lumina-radius-lg);
  box-shadow: var(--lumina-shadow-sm);
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  min-height: 0;
  overflow: hidden;
}

.panel-header {
  align-items: center;
  border-bottom: 1px solid var(--lumina-card-border);
  display: flex;
  justify-content: space-between;
  min-height: 44px;
  padding: 8px 12px;

  .panel-title {
    display: grid;
    gap: 2px;
  }

  span {
    color: var(--lumina-text-secondary);
    font-size: 11px;
  }

  strong {
    font-size: 14px;
  }
}

.command-toolbar {
  align-items: center;
  display: flex;
  gap: 6px;
}

.command-search {
  align-items: center;
  background: var(--lumina-input-bg);
  border: 1px solid var(--lumina-card-border);
  border-radius: var(--lumina-radius-sm);
  display: flex;
  gap: 6px;
  height: 30px;
  padding: 0 8px;
  width: 210px;

  &:focus-within {
    border-color: var(--lumina-primary);
    box-shadow: 0 0 0 2px var(--lumina-accent-ring);
  }

  svg {
    color: var(--lumina-text-secondary);
    flex: 0 0 auto;
    height: 15px;
    width: 15px;
  }

  input {
    background: transparent;
    border: 0;
    color: var(--lumina-text);
    font: inherit;
    font-size: 12px;
    min-width: 0;
    outline: 0;
    width: 100%;
  }
}

.command-sort {
  background: var(--lumina-button-secondary-bg);
  border: 1px solid var(--lumina-card-border);
  border-radius: var(--lumina-radius-sm);
  color: var(--lumina-text);
  font: inherit;
  font-size: 12px;
  height: 30px;
  padding: 0 26px 0 8px;
}

.pin-mode-btn,
.recent-command-btn {
  align-items: center;
  background: var(--lumina-button-secondary-bg);
  border: 1px solid var(--lumina-card-border);
  border-radius: var(--lumina-radius-sm);
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: inline-flex;
  height: 30px;
  justify-content: center;
}

.pin-mode-btn {
  padding: 0;
  width: 30px;

  &:hover,
  &.active {
    background: color-mix(in srgb, var(--lumina-primary) 10%, var(--lumina-surface-1));
    border-color: color-mix(in srgb, var(--lumina-primary) 36%, var(--lumina-card-border));
    color: var(--lumina-primary);
  }
}

.recent-command-btn {
  gap: 6px;
  padding: 0 8px;

  &:hover {
    background: var(--lumina-button-secondary-hover);
    color: var(--lumina-text);
  }

  b {
    color: var(--lumina-primary);
    font-size: 12px;
  }
}

.pin-mode-btn svg,
.recent-command-btn svg {
  height: 15px;
  width: 15px;
}

.empty-page,
.project-empty {
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

.tool-btn,
.row-action {
  align-items: center;
  background: var(--lumina-button-secondary-bg);
  border: 1px solid var(--lumina-card-border);
  border-radius: var(--lumina-radius-sm);
  color: var(--lumina-text);
  cursor: pointer;
  display: inline-flex;
  flex: 0 0 auto;
  font-size: 11px;
  height: 28px;
  justify-content: center;
  padding: 0 10px;
  white-space: nowrap;

  &:hover:not(:disabled) {
    background: var(--lumina-button-secondary-hover);
  }

  &:disabled {
    cursor: not-allowed;
    opacity: 0.56;
  }

  &.danger {
    color: var(--lumina-danger);

    &:hover:not(:disabled) {
      background: color-mix(in srgb, var(--lumina-danger) 8%, transparent);
    }
  }
}

.primary {
  background: var(--lumina-primary);
  border-color: var(--lumina-primary);
  color: var(--lumina-on-accent);
  justify-self: center;
}

.project-list {
  min-height: 0;
  overflow: auto;
  padding: 8px;
}

.project-row {
  background: var(--lumina-surface-1);
  border: 1px solid var(--lumina-card-border);
  border-radius: var(--lumina-radius-md);
  overflow: hidden;

  & + & {
    margin-top: 8px;
  }
}

.project-row-summary {
  align-items: center;
  display: grid;
  gap: 8px;
  grid-template-columns: minmax(0, 1fr) auto;
  min-height: 42px;
  padding: 6px 10px;
}

.project-row-actions {
  display: flex;
  gap: 6px;
  justify-self: end;
}

@media (hover: hover) {
  .project-row-actions {
    opacity: 0;
    pointer-events: none;
    transform: translateX(4px);
    transition: opacity 140ms ease, transform 140ms ease;
  }

  .project-row:hover .project-row-actions,
  .project-row:focus-within .project-row-actions {
    opacity: 1;
    pointer-events: auto;
    transform: translateX(0);
  }
}

.row-action {
  padding: 0;
  width: 28px;

  svg {
    height: 14px;
    width: 14px;
  }
}

.project-identity {
  display: grid;
  gap: 4px;
  min-width: 0;

  span {
    color: var(--lumina-text-secondary);
    font-size: 11px;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
}

.project-alias-text {
  color: var(--lumina-text);
  font-size: 13px;
  font-weight: 700;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.project-alias-edit {
  min-width: 0;
  width: 100%;
}

.project-alias-input {
  background: transparent;
  border: 1px solid transparent;
  border-radius: var(--lumina-radius-sm);
  color: var(--lumina-text);
  font: inherit;
  font-size: 13px;
  font-weight: 700;
  height: 26px;
  min-width: 0;
  padding: 0 6px;
  width: 100%;

  &::placeholder {
    color: var(--lumina-text-secondary);
    font-weight: 500;
  }

  &:hover {
    background: var(--lumina-surface-2);
    border-color: var(--lumina-card-border);
  }

  &:focus {
    background: var(--lumina-input-bg);
    border-color: var(--lumina-primary);
    box-shadow: 0 0 0 2px var(--lumina-accent-ring);
    outline: none;
  }
}

.project-error {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-danger) 10%, var(--lumina-surface-2));
  border-top: 1px solid color-mix(in srgb, var(--lumina-danger) 24%, transparent);
  color: var(--lumina-danger);
  display: flex;
  font-size: 11px;
  gap: 8px;
  justify-content: space-between;
  min-height: 30px;
  padding: 5px 10px 5px 44px;

  button {
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
      background: var(--lumina-button-secondary-hover);
      color: var(--lumina-text);
    }

    .close-glyph {
      font-size: 19px;
      font-weight: 300;
      line-height: 1;
      transform: translateY(-1px);
    }
  }
}

.project-row-details {
  border-top: 1px solid var(--lumina-card-border);
  padding: 8px 10px;
}

.project-loading-state {
  align-items: center;
  background: var(--lumina-surface-2);
  border: 1px dashed var(--lumina-card-border);
  border-radius: var(--lumina-radius-md);
  display: flex;
  gap: 12px;
  min-height: 80px;
  padding: 16px 20px;

  .loading-spinner {
    animation: spin 1.4s linear infinite;
    color: var(--lumina-primary);
    flex: 0 0 auto;
    font-size: 24px;
    height: 24px;
    width: 24px;
  }

  .loading-content {
    display: grid;
    gap: 3px;

    strong {
      color: var(--lumina-text);
      font-size: 13px;
    }

    span {
      color: var(--lumina-text-secondary);
      font-size: 11px;
    }
  }
}

@keyframes spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}

.project-candidates-guide {
  background: color-mix(in srgb, var(--lumina-primary) 6%, var(--lumina-surface-2));
  border: 1px solid color-mix(in srgb, var(--lumina-primary) 22%, var(--lumina-card-border));
  border-radius: var(--lumina-radius-md);
  display: grid;
  gap: 10px;
  padding: 12px 14px;

  .candidates-guide-info {
    align-items: flex-start;
    display: flex;
    gap: 10px;

    .guide-icon-badge {
      align-items: center;
      background: color-mix(in srgb, var(--lumina-primary) 16%, transparent);
      border-radius: var(--lumina-radius-sm);
      color: var(--lumina-primary);
      display: inline-flex;
      flex: 0 0 auto;
      height: 28px;
      justify-content: center;
      width: 28px;

      svg {
        height: 16px;
        width: 16px;
      }
    }

    .guide-text {
      display: grid;
      gap: 2px;

      strong {
        color: var(--lumina-text);
        font-size: 13px;
      }

      p {
        color: var(--lumina-text-secondary);
        font-size: 11px;
        line-height: 1.4;
        margin: 0;
      }
    }
  }

  .candidates-preview-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;

    .candidate-tag {
      background: var(--lumina-surface-1);
      border: 1px solid var(--lumina-card-border);
      border-radius: var(--lumina-radius-sm);
      color: var(--lumina-text);
      font-family: monospace;
      font-size: 11px;
      padding: 2px 8px;

      &.more {
        color: var(--lumina-primary);
        font-weight: bold;
      }
    }
  }

  .candidate-action-btn {
    align-self: start;
    gap: 6px;
    height: 30px;
    justify-self: start;
    padding: 0 12px;

    svg {
      height: 14px;
      width: 14px;
    }
  }
}

.project-empty-guide {
  align-items: center;
  background: var(--lumina-empty-bg);
  border: 1px dashed var(--lumina-empty-border);
  border-radius: var(--lumina-radius-md);
  display: grid;
  gap: 10px;
  justify-items: center;
  min-height: 100px;
  padding: 16px;
  text-align: center;

  .empty-guide-info {
    align-items: center;
    display: grid;
    gap: 6px;
    justify-items: center;

    .empty-icon {
      color: var(--lumina-text-secondary);
      height: 22px;
      opacity: 0.6;
      width: 22px;
    }

    p {
      color: var(--lumina-text-secondary);
      font-size: 12px;
      line-height: 1.4;
      margin: 0;
      max-width: 440px;
    }
  }

  .empty-action-btn {
    gap: 6px;
    height: 28px;
    padding: 0 10px;

    svg {
      height: 13px;
      width: 13px;
    }
  }
}

.project-empty {
  background: var(--lumina-empty-bg);
  border: 1px dashed var(--lumina-empty-border);
  border-radius: var(--lumina-radius-md);
  min-height: 120px;
  padding: 18px;

  &.compact {
    min-height: 96px;
  }
}

.script-list {
  display: grid;
  gap: 5px;
  grid-template-columns: repeat(auto-fill, minmax(168px, 1fr));
}

.script-row {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-surface-2) 76%, transparent);
  border: 0.5px solid var(--lumina-card-border);
  border-radius: var(--lumina-radius-sm);
  display: grid;
  gap: 6px;
  grid-template-columns: 7px minmax(0, 1fr) 26px;
  min-height: 36px;
  padding: 4px 5px 4px 8px;
  transition: background var(--lumina-duration-fast) var(--lumina-ease-out), border-color var(--lumina-duration-fast) var(--lumina-ease-out), box-shadow var(--lumina-duration-fast) var(--lumina-ease-out);

  &:hover,
  &:focus-within {
    background: var(--lumina-control-hover);
    border-color: var(--lumina-separator-strong);
  }

  &.pin-editing {
    grid-template-columns: 7px minmax(0, 1fr) 26px 26px;
  }

  strong {
    font-size: 12px;
    font-weight: 550;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  &.running {
    background: color-mix(in srgb, var(--lumina-success) 9%, var(--lumina-surface-1));
    border-color: color-mix(in srgb, var(--lumina-success) 42%, var(--lumina-card-border));

    strong {
      color: color-mix(in srgb, var(--lumina-success) 78%, var(--lumina-text));
    }
  }

  &.starting {
    opacity: 0.68;
  }

  .pin-btn,
  .script-run-btn {
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

    svg {
      height: 14px;
      width: 14px;
    }
  }

  .pin-btn {
    &:hover,
    &.active {
      background: color-mix(in srgb, var(--lumina-primary) 10%, var(--lumina-surface-1));
      color: var(--lumina-primary);
    }
  }

  .script-run-btn {
    color: var(--lumina-primary);

    &:hover:not(:disabled) {
      background: color-mix(in srgb, var(--lumina-primary) 12%, transparent);
    }

    &:disabled {
      cursor: not-allowed;
      opacity: 0.5;
    }

    &.stop {
      color: var(--lumina-danger);

      &:hover:not(:disabled) {
        background: color-mix(in srgb, var(--lumina-danger) 10%, transparent);
      }
    }
  }
}

.script-status-dot {
  background: color-mix(in srgb, var(--lumina-text-secondary) 42%, transparent);
  border-radius: 999px;
  height: 6px;
  width: 6px;

  .running & {
    background: var(--lumina-success);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--lumina-success) 14%, transparent);
  }

  .starting & {
    background: var(--lumina-warning);
  }
}

.more-scripts-btn {
  align-items: center;
  background: transparent;
  border: 1px dashed color-mix(in srgb, var(--lumina-card-border) 78%, transparent);
  border-radius: var(--lumina-radius-sm);
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: inline-flex;
  font: inherit;
  font-size: 11px;
  gap: 6px;
  justify-content: center;
  min-height: 36px;
  padding: 0 10px;

  &:hover,
  &:focus-visible {
    background: var(--lumina-button-secondary-hover);
    border-color: var(--lumina-card-border);
    color: var(--lumina-text);
    outline: none;
  }

  svg {
    height: 14px;
    width: 14px;
  }
}
</style>
