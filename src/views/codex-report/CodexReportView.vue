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
        @load="loadSessions"
        @reset-projects="selectedProjects = []"
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
import { computed, onMounted, ref, watch } from 'vue'
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

const availableProjects = ref<CodexProjectInfo[]>([])
const projectsLoading = ref(false)
const selectedProjects = ref<string[]>(readSavedProjects())
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

watch(selectedProjects, val => {
  try {
    localStorage.setItem(SELECTED_PROJECTS_KEY, JSON.stringify(val))
  } catch {
    // Ignore storage errors
  }
}, { deep: true })

watch(
  () => [dateRange.value?.[0], dateRange.value?.[1], selectedProvider.value],
  ([start, end, prov], [oldStart, oldEnd, oldProv]) => {
    if (start && end && (start !== oldStart || end !== oldEnd || prov !== oldProv)) {
      void loadSessions()
    }
  }
)

onMounted(() => {
  void Promise.allSettled([
    fetchProjects(),
    loadSessions(),
  ])
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

    sessions.value = await loadCodexReportSessions({
      from: startDate.toISOString(),
      to: endDate.toISOString(),
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

.workspace-shell {
  display: flex;
  flex: 1;
  gap: 10px;
  min-height: 0;
  overflow: hidden;
}
</style>
