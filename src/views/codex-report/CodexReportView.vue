<template>
  <div class="codex-report-page">
    <!-- macOS Workbench Topbar -->
    <WorkbenchTopbar>
      <WorkbenchIdentity :label="t('codexReport.eyebrow')" :value="t('codexReport.title')" />
      <div class="toolbar-tags" :aria-label="t('codexReport.results')">
        <WorkbenchTag v-if="sessions.length" :label="t('codexReport.totalLabel')" :value="sessions.length" />
        <WorkbenchTag v-if="sessions.length" :label="t('codexReport.includedLabel')" :value="includedSessions.length" tone="primary" />
        <WorkbenchTag v-if="sessions.length" :label="t('codexReport.projectsLabel')" :value="includedProjects" />
        <WorkbenchTag v-if="sessions.length && timeRange !== '—'" :label="t('codexReport.timeRangeLabel')" :value="timeRange" />
      </div>

      <template #actions>
        <WorkbenchButton :disabled="loading" variant="primary" @click="loadSessions">
          <Icon icon="solar:refresh-linear" :class="{ 'spin-anim': loading }" />
          {{ loading ? t('codexReport.loading') : t('codexReport.load') }}
        </WorkbenchButton>
        <WorkbenchButton :disabled="!includedSessions.length" @click="generateWorkRecord">
          <Icon icon="solar:notes-linear" />
          {{ t('codexReport.generate') }}
        </WorkbenchButton>
        <WorkbenchButton @click="openPromptDrawer">
          <Icon icon="solar:document-text-linear" />
          {{ t('codexReport.promptTemplate') }}
        </WorkbenchButton>
      </template>
    </WorkbenchTopbar>

    <!-- macOS Filter & Control Strip -->
    <section class="filter-strip">
      <!-- 1. Date Range Control with Inline Segmented Shortcuts -->
      <div class="filter-item filter-item-date">
        <div class="inline-date-segmented">
          <button
            type="button"
            :class="{ active: isCurrentDay(0) }"
            @click="setDateRange(0)"
          >
            {{ t('codexReport.today') }}
          </button>
          <button
            type="button"
            :class="{ active: isCurrentDay(-1) }"
            @click="setDateRange(-1)"
          >
            {{ t('codexReport.yesterday') }}
          </button>
        </div>
        <NDatePicker
          v-model:value="dateRange"
          type="daterange"
          size="small"
          clearable
          class="compact-date-picker"
        />
      </div>

      <!-- 2. AI Tool Provider Filter -->
      <div class="filter-item filter-item-provider">
        <div class="inline-provider-segmented" role="tablist">
          <button
            type="button"
            :class="{ active: selectedProvider === 'all' }"
            @click="selectedProvider = 'all'"
            :title="t('codexReport.allTools')"
          >
            <Icon icon="solar:layers-minimalistic-linear" class="provider-btn-icon" />
            <span>{{ t('codexReport.allTools') }}</span>
          </button>
          <button
            type="button"
            :class="{ active: selectedProvider === 'codex' }"
            @click="selectedProvider = 'codex'"
            title="Codex CLI"
          >
            <Icon icon="solar:code-square-linear" class="provider-btn-icon icon-codex" />
            <span>Codex</span>
          </button>
          <button
            type="button"
            :class="{ active: selectedProvider === 'claude' }"
            @click="selectedProvider = 'claude'"
            title="Claude Code"
          >
            <Icon icon="solar:magic-stick-3-linear" class="provider-btn-icon icon-claude" />
            <span>Claude</span>
          </button>
          <button
            type="button"
            :class="{ active: selectedProvider === 'antigravity' }"
            @click="selectedProvider = 'antigravity'"
            title="Antigravity"
          >
            <Icon icon="solar:planet-linear" class="provider-btn-icon icon-agy" />
            <span>Antigravity</span>
          </button>
          <button
            type="button"
            :class="{ active: selectedProvider === 'opencode' }"
            @click="selectedProvider = 'opencode'"
            title="OpenCode"
          >
            <Icon icon="solar:terminal-linear" class="provider-btn-icon icon-opencode" />
            <span>OpenCode</span>
          </button>
        </div>
      </div>

      <!-- 3. Project Filter Button -->
      <div class="filter-item filter-item-project">
        <button
          type="button"
          class="project-config-trigger"
          :title="t('codexReport.manageProjects')"
          @click="projectModalOpen = true"
        >
          <div class="trigger-main">
            <Icon icon="solar:folder-with-files-linear" class="trigger-icon" />
            <span class="trigger-text">
              {{
                !selectedProjects.length || selectedProjects.length === allProjectNames.length
                  ? t('codexReport.allProjectsSelected', { count: allProjectNames.length })
                  : t('codexReport.selectedProjectsCount', { count: selectedProjects.length })
              }}
            </span>
          </div>
          <div class="trigger-badge">
            <Icon icon="solar:tuning-2-linear" />
          </div>
        </button>
      </div>

      <!-- 4. Keyword Search -->
      <div class="filter-item filter-item-search">
        <NInput
          v-model:value="keyword"
          size="small"
          clearable
          :placeholder="t('codexReport.searchPlaceholder')"
          class="compact-search-input"
        >
          <template #prefix>
            <Icon icon="solar:magnifer-linear" class="search-prefix-icon" />
          </template>
        </NInput>
      </div>

      <!-- 5. Auto Exclude Toggle -->
      <div class="filter-item filter-item-auto-exclude">
        <NCheckbox v-model:checked="autoExcludeInvalid" size="small">
          <span class="checkbox-label" :title="t('codexReport.autoExcludeHint')">
            {{ t('codexReport.autoExclude') }}
          </span>
        </NCheckbox>
      </div>
    </section>

    <!-- macOS Split Workspace Shell -->
    <section class="workspace-shell">
      <!-- Left Panel: Session Source List -->
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
            @click="sessionView = 'all'"
          >
            {{ t('codexReport.all', { count: filteredSessions.length }) }}
          </button>
          <button
            type="button"
            role="tab"
            :class="{ active: sessionView === 'included' }"
            @click="sessionView = 'included'"
          >
            {{ t('codexReport.included', { count: includedSessions.length }) }}
          </button>
          <button
            type="button"
            role="tab"
            :class="{ active: sessionView === 'excluded' }"
            @click="sessionView = 'excluded'"
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
              <WorkbenchButton variant="primary" @click="loadSessions">
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
          />

          <NCheckboxGroup v-else v-model:value="includedIds" class="session-card-list">
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

      <!-- Right Panel: Editor / Preview Detail Area -->
      <section class="editor-detail-panel">
        <header class="panel-header">
          <div class="panel-title-group">
            <span class="panel-eyebrow">{{ t('codexReport.editable') }}</span>
            <h3>{{ t('codexReport.workRecord') }}</h3>
          </div>

          <!-- macOS Center Segmented Switcher -->
          <div class="segmented-control" role="tablist">
            <button
              type="button"
              role="tab"
              :class="{ active: editorMode === 'markdown' }"
              @click="editorMode = 'markdown'"
            >
              Markdown
            </button>
            <button
              type="button"
              role="tab"
              :class="{ active: editorMode === 'preview' }"
              @click="editorMode = 'preview'"
            >
              {{ t('codexReport.preview') }}
            </button>
          </div>

          <!-- Header Action Buttons -->
          <div class="editor-actions">
            <WorkbenchButton
              :disabled="!workRecord"
              @click="copyText(workRecord, t('codexReport.workRecordCopied'))"
            >
              <Icon icon="solar:copy-linear" />
              {{ t('codexReport.copyWorkRecord') }}
            </WorkbenchButton>
            <WorkbenchButton
              variant="primary"
              :disabled="!workRecord"
              @click="copyText(combinedContent, t('codexReport.combinedCopied'))"
            >
              <Icon icon="solar:copy-linear" />
              {{ t('codexReport.copyForAi') }}
            </WorkbenchButton>
          </div>
        </header>

        <!-- Editor & Preview Body -->
        <div class="editor-body">
          <WorkbenchEmptyState
            v-if="!workRecord"
            icon="solar:document-linear"
            :title="t('codexReport.noWorkRecord')"
            :description="t('codexReport.editorPlaceholder')"
          />
          <template v-else>
            <NInput
              v-if="editorMode === 'markdown'"
              v-model:value="workRecord"
              type="textarea"
              :autosize="false"
              class="macos-report-editor"
              :placeholder="t('codexReport.editorPlaceholder')"
            />
            <article v-else class="macos-markdown-preview" v-html="markdownPreview"></article>
          </template>
        </div>
      </section>
    </section>

    <!-- macOS Project Filter Sheet Modal -->
    <NModal
      v-model:show="projectModalOpen"
      class="project-filter-modal"
      :auto-focus="false"
      :mask-closable="true"
      :trap-focus="false"
    >
      <WorkbenchSheet
        size="normal"
        icon="solar:folder-with-files-linear"
        :title="t('codexReport.projectModalTitle')"
        :description="t('codexReport.projectModalDesc')"
        close-label="Close"
        @close="projectModalOpen = false"
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
            <NCheckboxGroup v-else v-model:value="selectedProjects" class="project-items-group">
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
                      <span v-if="project.provider" class="project-prov-badge" :class="`prov-${project.provider}`">
                        {{ getProviderLabel(project.provider) }}
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
              <WorkbenchButton variant="primary" @click="projectModalOpen = false">
                {{ t('common.confirm') }}
              </WorkbenchButton>
            </div>
          </div>
        </template>
      </WorkbenchSheet>
    </NModal>

    <!-- macOS Prompt Template Drawer -->
    <WorkbenchDrawer
      v-if="promptDrawerOpen"
      :title="t('codexReport.promptTitle')"
      :description="t('codexReport.promptHint')"
      close-label="Close"
      size="normal"
      @close="promptDrawerOpen = false"
    >
      <div class="prompt-drawer-container">
        <!-- Preset Template Quick Switcher -->
        <div class="prompt-preset-row">
          <span class="preset-label">{{ t('codexReport.presetTemplateSelect') }}:</span>
          <div class="preset-buttons">
            <button
              type="button"
              class="preset-chip"
              @click="applyPreset(DEFAULT_WEB_AI_PROMPT)"
            >
              {{ t('codexReport.presetStandard') }}
            </button>
            <button
              type="button"
              class="preset-chip"
              @click="applyPreset(STANDUP_PROMPT_TEMPLATE)"
            >
              {{ t('codexReport.presetStandup') }}
            </button>
            <button
              type="button"
              class="preset-chip"
              @click="applyPreset(TECH_SUMMARY_PROMPT_TEMPLATE)"
            >
              {{ t('codexReport.presetTech') }}
            </button>
          </div>
        </div>

        <NInput
          v-model:value="promptDraft"
          type="textarea"
          :autosize="false"
          class="prompt-editor-textarea"
        />
        <footer class="drawer-footer">
          <WorkbenchButton @click="promptDraft = DEFAULT_WEB_AI_PROMPT">
            {{ t('codexReport.restoreDefault') }}
          </WorkbenchButton>
          <WorkbenchButton variant="primary" @click="savePrompt">
            {{ t('common.save') }}
          </WorkbenchButton>
        </footer>
      </div>
    </WorkbenchDrawer>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { marked } from 'marked'
import { NCheckbox, NCheckboxGroup, NDatePicker, NInput, NModal, useMessage } from 'naive-ui'
import WorkbenchTopbar from '@/components/workbench/WorkbenchTopbar.vue'
import WorkbenchIdentity from '@/components/workbench/WorkbenchIdentity.vue'
import WorkbenchTag from '@/components/workbench/WorkbenchTag.vue'
import WorkbenchButton from '@/components/workbench/WorkbenchButton.vue'
import WorkbenchEmptyState from '@/components/workbench/WorkbenchEmptyState.vue'
import WorkbenchDrawer from '@/components/workbench/WorkbenchDrawer.vue'
import WorkbenchSheet from '@/components/workbench/WorkbenchSheet.vue'
import { loadCodexProjects, loadCodexReportSessions } from '@/services/codex-report-service'
import { useLocale } from '@/hooks/useLocale'
import type { AiToolProvider, CodexProjectInfo, CodexReportSession } from '@/types/codex-report'
import {
  DEFAULT_WEB_AI_PROMPT,
  STANDUP_PROMPT_TEMPLATE,
  TECH_SUMMARY_PROMPT_TEMPLATE,
  formatSessionTime,
  getProjectName,
  getProviderIcon,
  getProviderLabel,
  hasWorkContent,
  matchesKeyword,
  renderWorkRecord,
} from '@/utils/codex-report'

type SessionView = 'all' | 'included' | 'excluded'
type EditorMode = 'markdown' | 'preview'

const PROMPT_STORAGE_KEY = 'lumina.codex-report.prompt.v1'
const SELECTED_PROJECTS_KEY = 'lumina.codex-report.selected-projects.v1'

const message = useMessage()
const { t } = useLocale()
const today = new Date()
today.setHours(0, 0, 0, 0)

const dateRange = ref<[number, number] | null>([today.getTime(), today.getTime() + 86_399_999])
const selectedProvider = ref<AiToolProvider>('all')
const sessions = ref<CodexReportSession[]>([])
const includedIds = ref<string[]>([])
const loading = ref(false)

const availableProjects = ref<CodexProjectInfo[]>([])
const projectsLoading = ref(false)
const selectedProjects = ref<string[]>(readSavedProjects())
const projectModalOpen = ref(false)
const projectSearch = ref('')

const keyword = ref('')
const autoExcludeInvalid = ref(true)
const sessionView = ref<SessionView>('all')
const editorMode = ref<EditorMode>('markdown')
const workRecord = ref('')
const promptDrawerOpen = ref(false)
const webAiPrompt = ref(readSavedPrompt())
const promptDraft = ref(webAiPrompt.value)

const allProjectList = computed<CodexProjectInfo[]>(() => {
  const map = new Map<string, CodexProjectInfo>()
  for (const p of availableProjects.value) {
    if (p.name) map.set(p.name, { ...p })
  }
  for (const s of sessions.value) {
    const name = s.projectName || getProjectName(s.cwd)
    if (!map.has(name)) {
      map.set(name, {
        name,
        cwd: s.cwd || name,
        sessionCount: 1,
        lastActiveAt: s.startedAt,
        provider: s.provider,
      })
    }
  }
  return [...map.values()].sort((a, b) => a.name.localeCompare(b.name))
})

const allProjectNames = computed(() => allProjectList.value.map(p => p.name))

const searchedProjectList = computed(() => {
  const query = projectSearch.value.trim().toLowerCase()
  if (!query) return allProjectList.value
  return allProjectList.value.filter(
    p => p.name.toLowerCase().includes(query) || (p.cwd && p.cwd.toLowerCase().includes(query))
  )
})

const filteredSessions = computed(() =>
  sessions.value.filter(session => {
    const matchesProv = selectedProvider.value === 'all' || session.provider === selectedProvider.value
    const projName = session.projectName || getProjectName(session.cwd)
    const matchesProj = !selectedProjects.value.length || selectedProjects.value.includes(projName)
    const matchesKey = matchesKeyword(session, keyword.value)
    return matchesProv && matchesProj && matchesKey
  })
)

const includedSessions = computed(() =>
  filteredSessions.value.filter(session => includedIds.value.includes(session.id))
)

const excludedSessions = computed(() =>
  filteredSessions.value.filter(session => !includedIds.value.includes(session.id))
)

const listSessions = computed(() =>
  sessionView.value === 'included'
    ? filteredSessions.value.filter(session => includedIds.value.includes(session.id))
    : sessionView.value === 'excluded'
      ? excludedSessions.value
      : filteredSessions.value
)

const includedProjects = computed(
  () => new Set(includedSessions.value.map(session => session.projectName || getProjectName(session.cwd))).size
)

const timeRange = computed(() => {
  const timestamps = includedSessions.value
    .flatMap(session => [session.startedAt, session.endedAt])
    .filter(Boolean)
    .sort()
  return timestamps.length
    ? `${formatSessionTime(timestamps[0])}–${formatSessionTime(timestamps.at(-1) ?? timestamps[0])}`
    : '—'
})

const combinedContent = computed(
  () => `${webAiPrompt.value.trim()}\n\n---\n\n${workRecord.value.trim()}`
)

const markdownPreview = computed(() =>
  marked.parse(escapeHtml(workRecord.value || t('codexReport.noWorkRecord')), { async: false })
)

watch(selectedProjects, val => {
  try {
    localStorage.setItem(SELECTED_PROJECTS_KEY, JSON.stringify(val))
  } catch {
    // Ignore storage errors
  }
}, { deep: true })

onMounted(async () => {
  await fetchProjects()
  await loadSessions()
})

async function fetchProjects() {
  projectsLoading.value = true
  try {
    availableProjects.value = await loadCodexProjects()
  } catch (error) {
    console.error('Failed to load projects:', error)
  } finally {
    projectsLoading.value = false
  }
}

function selectAllProjects() {
  selectedProjects.value = [...allProjectNames.value]
}

function clearSelectedProjects() {
  selectedProjects.value = []
}

function invertProjectSelection() {
  const current = new Set(selectedProjects.value)
  selectedProjects.value = allProjectNames.value.filter(name => !current.has(name))
}

async function loadSessions() {
  if (!dateRange.value) {
    message.warning(t('codexReport.selectDateRange'))
    return
  }
  loading.value = true
  try {
    const providers = selectedProvider.value === 'all'
      ? ['codex', 'claude', 'antigravity', 'opencode']
      : [selectedProvider.value]

    sessions.value = await loadCodexReportSessions({
      from: new Date(dateRange.value[0]).toISOString(),
      to: new Date(dateRange.value[1]).toISOString(),
      providers,
    })
    includedIds.value = sessions.value
      .filter(session => !autoExcludeInvalid.value || hasWorkContent(session))
      .map(session => session.id)
    sessionView.value = 'all'
    generateWorkRecord()
  } catch (error) {
    message.error(error instanceof Error ? error.message : String(error))
  } finally {
    loading.value = false
  }
}

function generateWorkRecord() {
  if (dateRange.value)
    workRecord.value = renderWorkRecord(includedSessions.value, dateRange.value[0])
}

function setDateRange(dayOffset: number) {
  const day = new Date()
  day.setDate(day.getDate() + dayOffset)
  day.setHours(0, 0, 0, 0)
  dateRange.value = [day.getTime(), day.getTime() + 86_399_999]
}

function isCurrentDay(dayOffset: number) {
  if (!dateRange.value) return false
  const target = new Date()
  target.setDate(target.getDate() + dayOffset)
  target.setHours(0, 0, 0, 0)
  const start = target.getTime()
  const end = start + 86_399_999
  return Math.abs(dateRange.value[0] - start) < 1000 && Math.abs(dateRange.value[1] - end) < 1000
}

function openPromptDrawer() {
  promptDraft.value = webAiPrompt.value
  promptDrawerOpen.value = true
}

function applyPreset(template: string) {
  promptDraft.value = template
}

function savePrompt() {
  const value = promptDraft.value.trim() || DEFAULT_WEB_AI_PROMPT
  webAiPrompt.value = value
  localStorage.setItem(PROMPT_STORAGE_KEY, value)
  promptDrawerOpen.value = false
  message.success(t('codexReport.promptSaved'))
}

function readSavedPrompt() {
  try {
    return localStorage.getItem(PROMPT_STORAGE_KEY) || DEFAULT_WEB_AI_PROMPT
  } catch {
    return DEFAULT_WEB_AI_PROMPT
  }
}

function readSavedProjects(): string[] {
  try {
    const raw = localStorage.getItem(SELECTED_PROJECTS_KEY)
    if (!raw) return []
    const parsed = JSON.parse(raw)
    return Array.isArray(parsed) ? parsed : []
  } catch {
    return []
  }
}

function escapeHtml(value: string) {
  return value.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;')
}

async function copyText(text: string, successMessage: string) {
  try {
    await navigator.clipboard.writeText(text)
    message.success(successMessage)
  } catch (error) {
    message.error(t('codexReport.copyFailed', { error: error instanceof Error ? error.message : String(error) }))
  }
}
</script>

<style scoped lang="scss">
.codex-report-page {
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  gap: 8px;
  height: 100%;
  min-height: 0;
  overflow: hidden;
  padding: 8px 12px;
}

.toolbar-tags {
  align-items: center;
  display: flex;
  gap: 6px;
  margin-left: 4px;
}

.spin-anim {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}

/* macOS Filter Strip */
.filter-strip {
  align-items: center;
  background: var(--lumina-surface-2);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-md);
  box-shadow: var(--lumina-shadow-sm);
  display: flex;
  flex: 0 0 auto;
  gap: 10px;
  min-height: 42px;
  padding: 6px 12px;
}

.filter-item {
  align-items: center;
  display: flex;
}

.filter-item-date {
  display: flex;
  flex: 0 0 auto;
  gap: 6px;
}

.inline-date-segmented {
  background: color-mix(in srgb, var(--lumina-surface-3) 80%, transparent);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-sm);
  display: inline-flex;
  gap: 1px;
  padding: 2px;

  button {
    background: transparent;
    border: 0;
    border-radius: calc(var(--lumina-radius-sm) - 2px);
    color: var(--lumina-text-secondary);
    cursor: pointer;
    font-size: 11px;
    font-weight: 500;
    padding: 3px 8px;
    transition: all var(--lumina-duration-fast) var(--lumina-ease-out);
    white-space: nowrap;

    &:hover {
      color: var(--lumina-text);
    }

    &.active {
      background: var(--lumina-surface-elevated);
      box-shadow: 0 1px 2px rgba(0, 0, 0, 0.06);
      color: var(--lumina-primary);
      font-weight: 600;
    }
  }
}

.compact-date-picker {
  width: 270px !important;
}

.filter-item-provider {
  display: flex;
  flex: 0 0 auto;
}

.inline-provider-segmented {
  background: color-mix(in srgb, var(--lumina-surface-3) 80%, transparent);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-sm);
  display: inline-flex;
  gap: 1px;
  padding: 2px;

  button {
    align-items: center;
    background: transparent;
    border: 0;
    border-radius: calc(var(--lumina-radius-sm) - 2px);
    color: var(--lumina-text-secondary);
    cursor: pointer;
    display: inline-flex;
    font-size: 11px;
    font-weight: 500;
    gap: 4px;
    padding: 3px 8px;
    transition: all var(--lumina-duration-fast) var(--lumina-ease-out);
    white-space: nowrap;

    &:hover {
      color: var(--lumina-text);
    }

    &.active {
      background: var(--lumina-surface-elevated);
      box-shadow: 0 1px 2px rgba(0, 0, 0, 0.06);
      color: var(--lumina-primary);
      font-weight: 600;
    }
  }
}

.provider-btn-icon {
  font-size: 13px;

  &.icon-codex {
    color: #10b981;
  }
  &.icon-claude {
    color: #f97316;
  }
  &.icon-agy {
    color: #6366f1;
  }
  &.icon-opencode {
    color: #a855f7;
  }
}

.filter-item-project {
  flex: 0 0 auto;
  width: 170px;
}

.project-config-trigger {
  align-items: center;
  background: var(--lumina-input-bg, var(--lumina-surface-elevated));
  border: 0.5px solid var(--lumina-separator-strong);
  border-radius: var(--lumina-radius-sm);
  color: var(--lumina-text);
  cursor: pointer;
  display: flex;
  height: 28px;
  justify-content: space-between;
  padding: 0 8px;
  text-align: left;
  transition: all var(--lumina-duration-fast) var(--lumina-ease-out);
  width: 100%;

  &:hover {
    background: var(--lumina-button-secondary-hover);
    border-color: var(--lumina-primary);
  }

  &:focus-visible {
    box-shadow: 0 0 0 3px var(--lumina-accent-ring);
    outline: none;
  }
}

.trigger-main {
  align-items: center;
  display: flex;
  gap: 6px;
  min-width: 0;
  overflow: hidden;
}

.trigger-icon {
  color: var(--lumina-primary);
  flex: 0 0 14px;
  font-size: 14px;
}

.trigger-text {
  font-size: 11.5px;
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.trigger-badge {
  color: var(--lumina-text-tertiary);
  display: flex;
  font-size: 13px;
}

.filter-item-search {
  flex: 1 1 180px;
  max-width: 280px;
}

.compact-search-input {
  width: 100%;
}

.search-prefix-icon {
  color: var(--lumina-text-tertiary);
  font-size: 14px;
}

.filter-item-auto-exclude {
  flex: 0 0 auto;
  margin-left: auto;
  padding-right: 2px;
}

.checkbox-label {
  color: var(--lumina-text);
  font-size: 11.5px;
  font-weight: 500;
}


/* macOS Split Workspace Shell */
.workspace-shell {
  display: flex;
  flex: 1;
  gap: 8px;
  min-height: 0;
  overflow: hidden;
}

.session-source-panel,
.editor-detail-panel {
  background: var(--lumina-surface-2);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-lg);
  box-shadow: var(--lumina-shadow-sm);
  display: flex;
  flex-direction: column;
  min-width: 0;
  overflow: hidden;
}

.session-source-panel {
  flex: 0 0 clamp(280px, 28%, 340px);
}

.editor-detail-panel {
  flex: 1;
}

.panel-header {
  align-items: center;
  border-bottom: 0.5px solid var(--lumina-separator);
  display: flex;
  gap: 10px;
  justify-content: space-between;
  min-height: 44px;
  padding: 8px 12px;
}

.panel-title-group {
  display: flex;
  flex-direction: column;
  gap: 1px;

  .panel-eyebrow {
    color: var(--lumina-primary);
    font-size: 9.5px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  h3 {
    color: var(--lumina-text);
    font-size: 13.5px;
    font-weight: 600;
    margin: 0;
  }
}

.count-pill {
  background: color-mix(in srgb, var(--lumina-surface-3) 85%, transparent);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-xs);
  color: var(--lumina-text-secondary);
  font-size: 11px;
  font-weight: 500;
  padding: 2px 7px;
}

/* macOS Segmented Control */
.segmented-control {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-surface-3) 80%, transparent);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-sm);
  display: inline-flex;
  gap: 2px;
  margin: 6px 12px 2px;
  padding: 2px;

  button {
    background: transparent;
    border: 0;
    border-radius: calc(var(--lumina-radius-sm) - 2px);
    color: var(--lumina-text-secondary);
    cursor: pointer;
    flex: 1;
    font-size: 11px;
    font-weight: 500;
    padding: 4px 10px;
    text-align: center;
    transition: all var(--lumina-duration-fast) var(--lumina-ease-out);
    white-space: nowrap;

    &:hover {
      color: var(--lumina-text);
    }

    &.active {
      background: var(--lumina-surface-elevated);
      box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
      color: var(--lumina-text);
      font-weight: 650;
    }
  }
}

.editor-detail-panel .segmented-control {
  margin: 0;
}

.editor-actions {
  align-items: center;
  display: flex;
  gap: 6px;
}

/* Session List Styling */
.session-list-container {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-height: 0;
  overflow-y: auto;
  padding: 6px 8px;
}

.session-card-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
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
  transition:
    background var(--lumina-duration-fast) var(--lumina-ease-out),
    border-color var(--lumina-duration-fast) var(--lumina-ease-out),
    transform var(--lumina-duration-fast) var(--lumina-ease-out);

  &:hover {
    background: color-mix(in srgb, var(--lumina-button-secondary-hover) 65%, var(--lumina-surface-elevated));
    border-color: var(--lumina-separator-strong);
  }

  &.is-included {
    border-left: 2.5px solid var(--lumina-primary);
  }
}

.card-checkbox {
  flex: 0 0 auto;
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
  gap: 6px;
}

.card-topline-left {
  align-items: center;
  display: flex;
  gap: 6px;
  min-width: 0;
  overflow: hidden;
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

/* Editor & Preview Area */
.editor-body {
  display: flex;
  flex: 1;
  min-height: 0;
  overflow: hidden;
  padding: 10px 12px;
}

.macos-report-editor {
  background: var(--lumina-surface-elevated);
  border-radius: var(--lumina-radius-sm);
  flex: 1;
  height: 100%;
}

.editor-detail-panel :deep(.macos-report-editor .n-input-wrapper),
.editor-detail-panel :deep(.macos-report-editor .n-input__textarea-el) {
  height: 100%;
}

.editor-detail-panel :deep(textarea) {
  font-family: var(--lumina-font-mono, ui-monospace, SFMono-Regular, Consolas, monospace);
  font-size: 12px;
  line-height: 1.7;
}

.macos-markdown-preview {
  background: var(--lumina-surface-elevated);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-sm);
  box-sizing: border-box;
  color: var(--lumina-text);
  flex: 1;
  font-size: 13px;
  line-height: 1.7;
  min-height: 0;
  overflow-y: auto;
  padding: 16px 20px;

  :deep(h1),
  :deep(h2),
  :deep(h3) {
    margin: 14px 0 8px;
  }

  :deep(h1) {
    border-bottom: 0.5px solid var(--lumina-separator);
    font-size: 18px;
    padding-bottom: 6px;
  }

  :deep(h2) {
    font-size: 15px;
  }

  :deep(h3) {
    font-size: 13.5px;
  }

  :deep(p),
  :deep(ul),
  :deep(ol) {
    margin: 6px 0;
  }

  :deep(code) {
    background: var(--lumina-surface-3);
    border-radius: 4px;
    font-family: var(--lumina-font-mono, monospace);
    font-size: 11.5px;
    padding: 1px 5px;
  }
}

/* Prompt Drawer Container */
.prompt-drawer-container {
  display: flex;
  flex-direction: column;
  gap: 12px;
  height: 100%;
  padding: 12px;
}

.prompt-preset-row {
  align-items: center;
  display: flex;
  gap: 8px;
}

.preset-label {
  color: var(--lumina-text-secondary);
  font-size: 11.5px;
  font-weight: 500;
  white-space: nowrap;
}

.preset-buttons {
  display: flex;
  gap: 6px;
}

.preset-chip {
  background: var(--lumina-surface-3);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-xs);
  color: var(--lumina-text);
  cursor: pointer;
  font-size: 11px;
  font-weight: 500;
  padding: 3px 8px;
  transition: all var(--lumina-duration-fast) var(--lumina-ease-out);

  &:hover {
    background: var(--lumina-surface-elevated);
    border-color: var(--lumina-primary);
    color: var(--lumina-primary);
  }
}

.prompt-editor-textarea {
  flex: 1;
  min-height: 0;
}

.prompt-drawer-container :deep(.prompt-editor-textarea .n-input-wrapper),
.prompt-drawer-container :deep(.prompt-editor-textarea .n-input__textarea-el) {
  height: 100%;
}

.drawer-footer {
  display: flex;
  justify-content: space-between;
}

/* Project Filter Sheet Modal */
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

@media (max-width: 1024px) {
  .filter-strip {
    flex-wrap: wrap;
  }
  .session-source-panel {
    flex-basis: 260px;
  }
}
</style>

