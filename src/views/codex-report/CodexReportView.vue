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
        <WorkbenchButton :disabled="loading" variant="primary" @click="reloadReport">
          <Icon icon="solar:refresh-linear" :class="{ 'spin-anim': loading }" />
          {{ loading ? t('codexReport.loading') : t('codexReport.load') }}
        </WorkbenchButton>
        <WorkbenchButton :disabled="loading || !includedSessions.length" @click="generateWorkRecord">
          <Icon icon="solar:notes-linear" />
          {{ t('codexReport.generate') }}
        </WorkbenchButton>
        <WorkbenchButton @click="promptDrawerOpen = true">
          <Icon icon="solar:document-text-linear" />
          {{ t('codexReport.promptTemplate') }}
        </WorkbenchButton>
      </template>
    </WorkbenchTopbar>

    <!-- macOS Filter & Control Strip -->
    <CodexReportFilterStrip
      v-model:date-range="dateRange"
      v-model:selected-provider="selectedProvider"
      v-model:keyword="keyword"
      v-model:auto-exclude-invalid="autoExcludeInvalid"
      :selected-projects="selectedProjects"
      :all-project-count="allProjectList.length"
      @open-project-modal="projectModalOpen = true"
    />

    <div v-if="loadWarnings.length" class="report-warning" role="status">
      {{ t('codexReport.partialLoad') }}
      <ul><li v-for="warning in loadWarnings" :key="warning">{{ warning }}</li></ul>
    </div>

    <!-- macOS Split Workspace Shell -->
    <section class="workspace-shell">
      <!-- Left Panel: Session Source List -->
      <CodexSessionList
        v-model:included-ids="includedIds"
        v-model:session-view="sessionView"
        :loading="loading"
        :sessions="sessions"
        :filtered-sessions="filteredSessions"
        :included-sessions="includedSessions"
        :excluded-sessions="excludedSessions"
        :list-sessions="listSessions"
        :selected-projects="selectedProjects"
        :all-project-count="allProjectList.length"
        @load="reloadReport"
        @reset-projects="selectedProjects = null"
      />

      <!-- Right Panel: Editor / Preview Detail Area -->
      <CodexReportEditor
        v-model:work-record="workRecord"
        v-model:editor-mode="editorMode"
        :combined-content="combinedContent"
      />
    </section>

    <!-- macOS Project Filter Sheet Modal -->
    <CodexProjectFilterModal
      v-model:show="projectModalOpen"
      v-model:selected-projects="selectedProjects"
      :projects="allProjectList"
    />

    <!-- macOS Prompt Template Drawer -->
    <CodexPromptDrawer
      v-model:show="promptDrawerOpen"
      v-model:active-prompt="webAiPrompt"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useMessage } from 'naive-ui'
import WorkbenchTopbar from '@/components/workbench/WorkbenchTopbar.vue'
import WorkbenchIdentity from '@/components/workbench/WorkbenchIdentity.vue'
import WorkbenchTag from '@/components/workbench/WorkbenchTag.vue'
import WorkbenchButton from '@/components/workbench/WorkbenchButton.vue'
import { loadCodexProjects, loadCodexReportSessions } from '@/services/codex-report-service'
import { useLocale } from '@/composables/useLocale'
import type { AiToolProvider, CodexProjectInfo, CodexReportSession } from '@/types/codex-report'
import {
  DEFAULT_WEB_AI_PROMPT,
  formatSessionTime,
  getProjectName,
  hasWorkContent,
  sessionSelectionKey,
  matchesKeyword,
  renderWorkRecord,
} from '@/utils/codex-report'
import CodexReportFilterStrip from './components/CodexReportFilterStrip.vue'
import CodexSessionList from './components/CodexSessionList.vue'
import CodexReportEditor from './components/CodexReportEditor.vue'
import CodexProjectFilterModal from './components/CodexProjectFilterModal.vue'
import CodexPromptDrawer from './components/CodexPromptDrawer.vue'

type SessionView = 'all' | 'included' | 'excluded'
type EditorMode = 'markdown' | 'preview'

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
let loadSequence = 0
let lastGeneratedRecord = ''
let staleRecordWarningShown = false
let loadTimer: ReturnType<typeof setTimeout> | undefined

const availableProjects = ref<CodexProjectInfo[]>([])
const projectsLoading = ref(false)
const projectWarnings = ref<string[]>([])
const sessionWarnings = ref<string[]>([])
const loadWarnings = computed(() => [...new Set([...projectWarnings.value, ...sessionWarnings.value])])
const selectedProjects = ref<string[] | null>(readSavedProjects())
const projectModalOpen = ref(false)

const keyword = ref('')
const autoExcludeInvalid = ref(true)
const sessionView = ref<SessionView>('all')
const editorMode = ref<EditorMode>('markdown')
const workRecord = ref('')
const promptDrawerOpen = ref(false)
const webAiPrompt = ref(DEFAULT_WEB_AI_PROMPT)

const allProjectList = computed<CodexProjectInfo[]>(() => {
  const map = new Map<string, CodexProjectInfo>()
  for (const p of availableProjects.value) {
    if (!p.name) continue
    const existing = map.get(p.name)
    if (!existing) {
      map.set(p.name, {
        ...p,
        providers: p.providers || (p.provider ? [p.provider] : []),
      })
    } else {
      const combinedProviders = Array.from(
        new Set([...(existing.providers || []), ...(p.providers || []), ...(p.provider ? [p.provider] : [])])
      )
      map.set(p.name, {
        name: p.name,
        cwd: existing.cwd || p.cwd,
        sessionCount: (existing.sessionCount || 0) + (p.sessionCount || 0),
        lastActiveAt: [existing.lastActiveAt, p.lastActiveAt].filter(Boolean).sort().at(-1) ?? null,
        provider: combinedProviders.length === 1 ? combinedProviders[0] : 'multi',
        providers: combinedProviders,
      })
    }
  }
  for (const s of sessions.value) {
    const name = s.projectName || getProjectName(s.cwd)
    const existing = map.get(name)
    if (!existing) {
      map.set(name, {
        name,
        cwd: s.cwd || name,
        sessionCount: 1,
        lastActiveAt: s.startedAt,
        provider: s.provider,
        providers: s.provider ? [s.provider] : [],
      })
    } else if (s.provider && !existing.providers?.includes(s.provider)) {
      existing.providers = [...(existing.providers || []), s.provider]
      if (existing.providers.length > 1) {
        existing.provider = 'multi'
      }
    }
  }
  return [...map.values()].sort((a, b) => a.name.localeCompare(b.name))
})

const includedSet = computed(() => new Set(includedIds.value))
const selectedProjectSet = computed(() => selectedProjects.value === null ? null : new Set(selectedProjects.value))

const filteredSessions = computed(() =>
  sessions.value.filter(session => {
    const matchesProv = selectedProvider.value === 'all' || session.provider === selectedProvider.value
    const projName = session.projectName || getProjectName(session.cwd)
    const matchesProj = selectedProjectSet.value === null || selectedProjectSet.value.has(projName)
    const matchesKey = matchesKeyword(session, keyword.value)
    return matchesProv && matchesProj && matchesKey
  })
)

const includedSessions = computed(() =>
  filteredSessions.value.filter(session => includedSet.value.has(sessionSelectionKey(session)))
)

const excludedSessions = computed(() =>
  filteredSessions.value.filter(session => !includedSet.value.has(sessionSelectionKey(session)))
)

const listSessions = computed(() =>
  sessionView.value === 'included'
    ? includedSessions.value
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

watch(selectedProjects, val => {
  try {
    localStorage.setItem(SELECTED_PROJECTS_KEY, JSON.stringify({ version: 2, projects: val }))
  } catch {
    // Ignore storage errors
  }
}, { deep: true })

watch(
  () => [dateRange.value?.[0], dateRange.value?.[1], selectedProvider.value],
  () => {
    clearTimeout(loadTimer)
    ++loadSequence
    if (!dateRange.value) {
      loading.value = false
      sessions.value = []
      includedIds.value = []
      if (workRecord.value === lastGeneratedRecord) {
        workRecord.value = ''
        lastGeneratedRecord = ''
      }
      return
    }
    loading.value = true
    loadTimer = setTimeout(() => { void loadSessions() }, 150)
  }
)

watch(autoExcludeInvalid, exclude => {
  const included = new Set(includedIds.value)
  for (const session of sessions.value) {
    if (hasWorkContent(session)) continue
    const key = sessionSelectionKey(session)
    if (exclude) included.delete(key)
    else included.add(key)
  }
  includedIds.value = [...included]
})

// Keep automatic output in sync without overwriting user-edited Markdown.
watch(includedSessions, () => {
  if (loading.value) return
  if (workRecord.value === lastGeneratedRecord) generateWorkRecord()
  else if (!staleRecordWarningShown) {
    staleRecordWarningShown = true
    message.warning(t('codexReport.filtersChangedAfterEdit'))
  }
}, { flush: 'post' })

onMounted(() => {
  void Promise.allSettled([
    fetchProjects(),
    loadSessions(),
  ])
})

function reloadReport() {
  void fetchProjects()
  void loadSessions()
}

async function fetchProjects() {
  projectsLoading.value = true
  try {
    availableProjects.value = await loadCodexProjects(warnings => { projectWarnings.value = warnings })
  } catch (error) {
    console.error('Failed to load projects:', error)
  } finally {
    projectsLoading.value = false
  }
}

onUnmounted(() => { clearTimeout(loadTimer); ++loadSequence })

async function loadSessions() {
  clearTimeout(loadTimer)
  if (!dateRange.value) {
    message.warning(t('codexReport.selectDateRange'))
    return
  }
  const request = ++loadSequence
  loading.value = true
  try {
    const providers = selectedProvider.value === 'all'
      ? ['codex', 'claude', 'antigravity', 'opencode']
      : [selectedProvider.value]

    const startDate = new Date(dateRange.value[0])
    const endDate = new Date(dateRange.value[1])
    if (
      endDate.getHours() === 0 &&
      endDate.getMinutes() === 0 &&
      endDate.getSeconds() === 0 &&
      endDate.getMilliseconds() === 0
    ) {
      endDate.setHours(23, 59, 59, 999)
    }

    const loadedSessions = await loadCodexReportSessions({
      from: startDate.toISOString(),
      to: endDate.toISOString(),
      providers,
    }, warnings => { if (request === loadSequence) sessionWarnings.value = warnings })
    if (request !== loadSequence) return
    sessions.value = loadedSessions
    includedIds.value = sessions.value
      .filter(session => !autoExcludeInvalid.value || hasWorkContent(session))
      .map(sessionSelectionKey)
    sessionView.value = 'all'
    if (workRecord.value === lastGeneratedRecord) generateWorkRecord()
  } catch (error) {
    if (request !== loadSequence) return
    sessions.value = []
    includedIds.value = []
    message.error(error instanceof Error ? error.message : String(error))
  } finally {
    if (request === loadSequence) loading.value = false
  }
}

function generateWorkRecord() {
  if (dateRange.value) {
    lastGeneratedRecord = renderWorkRecord(includedSessions.value, dateRange.value[0], dateRange.value[1])
    workRecord.value = lastGeneratedRecord
    staleRecordWarningShown = false
  }
}

function readSavedProjects(): string[] | null {
  try {
    const raw = localStorage.getItem(SELECTED_PROJECTS_KEY)
    if (!raw) return null
    const parsed = JSON.parse(raw)
    if (parsed?.version === 2) {
      return Array.isArray(parsed.projects) ? parsed.projects.filter((value: unknown): value is string => typeof value === 'string') : null
    }
    // Legacy [] meant all projects.
    return Array.isArray(parsed) && parsed.length ? parsed.filter((value): value is string => typeof value === 'string') : null
  } catch {
    return null
  }
}
</script>

<style scoped lang="scss">
.report-warning {
  color: var(--lumina-text-secondary);
  font-size: 12px;
  max-height: 100px;
  overflow: auto;
  padding: 8px 12px;
}

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

.workspace-shell {
  display: flex;
  flex: 1;
  gap: 10px;
  min-height: 0;
  overflow: hidden;
}
</style>
