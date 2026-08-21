<template>
  <div class="devdock-page">
    <WorkbenchTopbar>
      <WorkbenchIdentity :label="t('devdock.overview.eyebrow')" :value="t('devdock.overview.title')" />
      <div class="toolbar-tags" :aria-label="t('devdock.projects.summary', { count: projects.length, scanned: scannedCount })">
        <WorkbenchTag :label="t('devdock.projects.totalLabel')" :value="projects.length" />
        <WorkbenchTag v-if="scannedCount !== projects.length" :label="t('devdock.projects.scannedLabel')" :value="scannedCount" tone="primary" />
        <WorkbenchTag v-if="runningProcessCount" :label="t('devdock.processes.running')" :value="runningProcessCount" tone="success" />
      </div>

      <template #actions>
        <WorkbenchButton
          :aria-pressed="processInspectorOpen"
          @click="processInspectorOpen = !processInspectorOpen"
        >
          {{ processInspectorOpen ? t('devdock.actions.hideProcesses') : t('devdock.actions.showProcesses') }}
        </WorkbenchButton>
        <WorkbenchButton :disabled="!projects.length || loadingAll" @click="scanAllProjects">
          {{ loadingAll ? t('devdock.actions.scanning') : t('devdock.actions.scanAll') }}
        </WorkbenchButton>
        <WorkbenchButton variant="primary" @click="handleAddProject">
          {{ t('devdock.actions.addProject') }}
        </WorkbenchButton>
      </template>
    </WorkbenchTopbar>

    <section class="devdock-shell" :class="{ 'inspector-open': processInspectorOpen }">
      <DevDockProjectList
        class="source-list"
        v-model:pin-editing="pinEditing"
        v-model:script-search="scriptSearch"
        :displayed-scripts="displayedScripts"
        :editing-alias-path="editingAliasPath"
        :filtered-scripts="filteredScripts"
        :has-projects="Boolean(projects.length)"
        :hidden-script-count="hiddenScriptCount"
        :is-project-commands-expanded="isProjectCommandsExpanded"
        :is-script-pinned="isScriptPinned"
        :is-script-running="isScriptRunning"
        :is-script-starting="isScriptStarting"
        :projects="visibleProjects"
        :recent-count="recentCommands.length"
        :script-action-label="scriptActionLabel"
        :script-sort="scriptSort"
        :set-alias-input-ref="setAliasInputRef"
        @add-project="handleAddProject"
        @cancel-edit-alias="cancelEditAlias"
        @configure-commands="openCommandConfig"
        @dismiss-project-error="dismissProjectError"
        @finish-edit-alias="finishEditAlias"
        @open-recent="recentDrawerOpen = true"
        @remove-project="removeProject"
        @rename-project="renameProject"
        @scan-project="project => scanProject(project, { touch: true })"
        @start-edit-alias="startEditAlias"
        @toggle-pinned-script="togglePinnedScript"
        @toggle-project-commands="toggleProjectCommands"
        @toggle-script="toggleScript"
        @update:script-sort="setScriptSort"
      />

      <DevDockProcessPanel
        v-if="processInspectorOpen"
        :is-busy="isProcessBusy"
        :process-status-label="processStatusLabel"
        :process-url="processUrl"
        :processes="processes"
        @copy-url="copyProcessUrl"
        @open-logs="openProcessLogs"
        @open-url="openProcessUrl"
        @restart="restartProcess"
        @stop="stopProcess"
        @close="processInspectorOpen = false"
      />
    </section>

    <DevDockRecentDrawer
      :commands="recentCommands"
      :is-running="isRecentCommandRunning"
      :show="recentDrawerOpen"
      @close="recentDrawerOpen = false"
      @start-command="startRecentCommand"
    />

    <DevDockCommandConfigDrawer
      :candidates="configProject?.manifest?.candidates ?? []"
      :project-commands="configProject?.manifest?.commands ?? []"
      :project-path="configProject?.path ?? ''"
      :show="Boolean(configProject)"
      @close="configProject = null"
      @saved="handleConfigSaved"
    />

    <DevDockLogModal :logs="processLogs" :show="logModalOpen" @after-leave="stopLogPolling" @close="closeProcessLogs" />
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onActivated, onDeactivated, onMounted, onUnmounted, reactive, ref } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { useMessage } from 'naive-ui'
import { useLocale } from '@/hooks/useLocale'
import { GIT_RECENT_REPOS_STORAGE_KEY, GIT_REPO_STORAGE_KEY } from '@/constants/git'
import WorkbenchButton from '@/components/workbench/WorkbenchButton.vue'
import WorkbenchIdentity from '@/components/workbench/WorkbenchIdentity.vue'
import WorkbenchTag from '@/components/workbench/WorkbenchTag.vue'
import WorkbenchTopbar from '@/components/workbench/WorkbenchTopbar.vue'
import { reportError } from '@/services/app-error-service'
import {
  listProjectProcesses,
  loadProjectManifest,
  loadProjectProcessLogs,
  openProjectUrl,
  restartProjectProcess,
  startProjectCommand as invokeStartProjectCommand,
  stopProjectProcess,
  type ProjectProcessLogs,
  type ProjectProcessSnapshot,
  type ProjectCommand,
} from '@/services/project/project-service'
import DevDockLogModal from './components/DevDockLogModal.vue'
import DevDockCommandConfigDrawer from './components/DevDockCommandConfigDrawer.vue'
import DevDockProcessPanel from './components/DevDockProcessPanel.vue'
import DevDockProjectList from './components/DevDockProjectList.vue'
import DevDockRecentDrawer from './components/DevDockRecentDrawer.vue'
import type { DevDockProject, RecentCommand, ScriptSort, StoredProject } from './types'

const DEVDOC_PINNED_SCRIPTS_STORAGE_KEY = 'lumina.devdock.pinnedCommands.v1'
const DEVDOC_RECENT_COMMANDS_STORAGE_KEY = 'lumina.devdock.recentCommands.v1'
const DEVDOC_SCRIPT_SORT_STORAGE_KEY = 'lumina.devdock.commandSort.v2'
const LEGACY_PINNED_SCRIPTS_STORAGE_KEY = 'lumina.devdock.pinnedScripts'
const LEGACY_RECENT_COMMANDS_STORAGE_KEY = 'lumina.devdock.recentCommands'
const SCRIPT_PRIORITY = ['dev', 'serve', 'start', 'tauri:dev', 'preview', 'build', 'test', 'lint']

interface ProjectScriptView {
  displayed: ProjectCommand[]
  filtered: ProjectCommand[]
  hiddenCount: number
}

const { t } = useLocale()
const message = useMessage()
const projects = ref<DevDockProject[]>([])
const pinnedScripts = ref(new Set<string>())
const expandedCommandProjects = reactive(new Set<string>())
const recentCommands = ref<RecentCommand[]>([])
const recentDrawerOpen = ref(false)
const processes = ref<ProjectProcessSnapshot[]>([])
const processBusy = reactive(new Set<string>())
const startingScripts = reactive(new Set<string>())
const processLogs = ref<ProjectProcessLogs | null>(null)
const logModalOpen = ref(false)
const editingAliasPath = ref<string | null>(null)
const aliasInputRefs = new Map<string, HTMLInputElement>()
const scriptSearch = ref('')
const scriptSort = ref<ScriptSort>(loadScriptSort())
const pinEditing = ref(false)
const processInspectorOpen = ref(false)
const configProject = ref<DevDockProject | null>(null)
let logPollTimer: ReturnType<typeof window.setInterval> | undefined
let processPollTimer: ReturnType<typeof window.setInterval> | undefined
const loadingAll = computed(() => projects.value.some(project => project.loading))
const scannedCount = computed(() => projects.value.filter(project => project.manifest).length)
const runningProcessCount = computed(() => processes.value.filter(process => process.status.state === 'running').length)
const sortedProjects = computed(() => [...projects.value].sort((left, right) => right.openedAt - left.openedAt))
const scriptViews = computed(() => {
  const views = new Map<string, ProjectScriptView>()
  for (const project of projects.value) {
    const filtered = getSortedProjectScripts(project)
    const displayed = getDisplayedProjectScripts(project, filtered)
    views.set(normalizePath(project.path), {
      displayed,
      filtered,
      hiddenCount: Math.max(0, filtered.length - displayed.length),
    })
  }
  return views
})
const visibleProjects = computed(() => {
  if (!scriptSearch.value) return sortedProjects.value
  return sortedProjects.value.filter(project => filteredScripts(project).length > 0)
})

onMounted(() => {
  pinnedScripts.value = loadPinnedScripts()
  recentCommands.value = loadRecentCommands()
  projects.value = loadStoredProjects()
  void scanAllProjects()
  void refreshProcesses()
  startProcessPolling()
})

onActivated(() => {
  void refreshProcesses()
  startProcessPolling()
})

onDeactivated(() => {
  stopProcessPolling()
})

onUnmounted(() => {
  stopLogPolling()
  stopProcessPolling()
})

async function handleAddProject() {
  const selected = await open({
    directory: true,
    multiple: false,
    title: t('devdock.actions.addProject'),
  })

  if (typeof selected !== 'string') return
  const project = rememberProject(selected)
  await scanProject(project, { touch: true })
}

function openCommandConfig(project: DevDockProject) {
  configProject.value = project
}

async function handleConfigSaved() {
  const project = configProject.value
  if (!project) return
  await scanProject(project, { touch: true })
  configProject.value = null
}

async function scanAllProjects() {
  await runWithConcurrency(projects.value, 3, project => scanProject(project))
}

async function scanProject(project: DevDockProject, options: { touch?: boolean } = {}) {
  if (options.touch) {
    touchProject(project)
  }
  project.loading = true
  project.error = ''
  try {
    project.manifest = await loadProjectManifest(project.path)
    project.error = project.manifest.configError || ''
  } catch (err) {
    project.manifest = null
    project.error = err instanceof Error ? err.message : String(err)
  } finally {
    project.loading = false
  }
}

function renameProject(path: string, name: string) {
  const normalized = normalizePath(path)
  const project = projects.value.find(project => normalizePath(project.path) === normalized)
  if (project) {
    project.name = name
  }
}

function normalizeProjectAlias(project: DevDockProject) {
  project.name = project.name.trim() || project.manifest?.name || getProjectDisplayName(project.path)
  persistProjects()
}

function setAliasInputRef(el: HTMLInputElement | null, path: string) {
  if (el) {
    aliasInputRefs.set(path, el)
  } else {
    aliasInputRefs.delete(path)
  }
}

function startEditAlias(path: string) {
  editingAliasPath.value = path
  nextTick(() => {
    const input = aliasInputRefs.get(path)
    if (input) {
      input.focus()
      input.select()
    }
  })
}

function finishEditAlias(project: DevDockProject) {
  normalizeProjectAlias(project)
  editingAliasPath.value = null
}

function cancelEditAlias() {
  editingAliasPath.value = null
}

function dismissProjectError(project: DevDockProject) {
  project.error = ''
}

function removeProject(path: string) {
  const normalized = normalizePath(path)
  projects.value = projects.value.filter(project => normalizePath(project.path) !== normalized)
  persistProjects()
}

function rememberProject(path: string) {
  const normalized = normalizePath(path)
  const existing = projects.value.find(project => normalizePath(project.path) === normalized)
  if (existing) return existing

  const project: DevDockProject = {
    path,
    name: getProjectDisplayName(path),
    loading: false,
    error: '',
    manifest: null,
    openedAt: Date.now(),
  }
  projects.value = [project, ...projects.value]
  localStorage.setItem(GIT_REPO_STORAGE_KEY, path)
  persistProjects()
  return project
}

function loadStoredProjects() {
  const stored = readRecentProjects()
  const current = localStorage.getItem(GIT_REPO_STORAGE_KEY)
  if (current) {
    stored.unshift({ path: current, name: getProjectDisplayName(current) })
  }

  const seen = new Set<string>()
  return stored
    .filter(project => typeof project.path === 'string' && project.path.trim())
    .filter(project => {
      const normalized = normalizePath(project.path || '')
      if (seen.has(normalized)) return false
      seen.add(normalized)
      return true
    })
    .map(project => ({
      path: project.path as string,
      name: project.name || getProjectDisplayName(project.path as string),
      loading: false,
      error: '',
      manifest: null,
      openedAt: typeof project.openedAt === 'number' ? project.openedAt : 0,
    }))
    .sort((left, right) => right.openedAt - left.openedAt)
}

function readRecentProjects(): StoredProject[] {
  try {
    const raw = localStorage.getItem(GIT_RECENT_REPOS_STORAGE_KEY)
    return raw ? (JSON.parse(raw) as StoredProject[]) : []
  } catch (err) {
    reportError('devdock.load-recent-projects', err)
    return []
  }
}

function persistProjects() {
  const stored = projects.value.map(project => ({
    path: project.path,
    name: project.name || getProjectDisplayName(project.path),
    openedAt: project.openedAt,
  }))
  localStorage.setItem(GIT_RECENT_REPOS_STORAGE_KEY, JSON.stringify(stored))
}

function touchProject(project: DevDockProject) {
  project.openedAt = Date.now()
  localStorage.setItem(GIT_REPO_STORAGE_KEY, project.path)
  persistProjects()
}

function filteredScripts(project: DevDockProject) {
  return getProjectScriptView(project).filtered
}

function displayedScripts(project: DevDockProject) {
  return getProjectScriptView(project).displayed
}

function hiddenScriptCount(project: DevDockProject) {
  return getProjectScriptView(project).hiddenCount
}

function getProjectScriptView(project: DevDockProject): ProjectScriptView {
  return scriptViews.value.get(normalizePath(project.path)) ?? {
    displayed: [],
    filtered: [],
    hiddenCount: 0,
  }
}

function getSortedProjectScripts(project: DevDockProject) {
  const search = scriptSearch.value.toLocaleLowerCase()
  const scripts = (project.manifest?.commands ?? []).filter(command => !search || command.name.toLocaleLowerCase().includes(search))
  return [...scripts].sort((left, right) => {
    if (scriptSort.value === 'recent') {
      const recentDifference = getScriptLastUsed(project.path, right.id) - getScriptLastUsed(project.path, left.id)
      if (recentDifference) return recentDifference
    }

    const defaultId = project.manifest?.defaultCommandId
    const leftDefault = left.id === defaultId
    const rightDefault = right.id === defaultId
    if (leftDefault !== rightDefault) return leftDefault ? -1 : 1

    const leftPinned = isScriptPinned(project.path, left.id)
    const rightPinned = isScriptPinned(project.path, right.id)
    if (leftPinned !== rightPinned) return leftPinned ? -1 : 1

    if (scriptSort.value === 'priority') {
      const leftPriority = getScriptPriority(left.name)
      const rightPriority = getScriptPriority(right.name)
      if (leftPriority !== rightPriority) return leftPriority - rightPriority
    }

    return left.name.localeCompare(right.name)
  })
}

function getDisplayedProjectScripts(project: DevDockProject, scripts: ProjectCommand[]) {
  if (scriptSearch.value || expandedCommandProjects.has(project.path)) return scripts

  const pinned = scripts.filter(command => isScriptPinned(project.path, command.id))
  const suggested = scripts.filter(command => !isScriptPinned(project.path, command.id)).slice(0, 4)
  return [...pinned, ...suggested]
}

function toggleProjectCommands(path: string) {
  if (expandedCommandProjects.has(path)) {
    expandedCommandProjects.delete(path)
  } else {
    expandedCommandProjects.add(path)
  }
}

function isProjectCommandsExpanded(path: string) {
  return expandedCommandProjects.has(path)
}

function loadScriptSort(): ScriptSort {
  const saved = localStorage.getItem(DEVDOC_SCRIPT_SORT_STORAGE_KEY)
  return saved === 'name' || saved === 'priority' ? saved : 'recent'
}

function setScriptSort(value: ScriptSort) {
  scriptSort.value = value
  persistScriptSort()
}

function persistScriptSort() {
  localStorage.setItem(DEVDOC_SCRIPT_SORT_STORAGE_KEY, scriptSort.value)
}

function getScriptLastUsed(projectPath: string, commandId: string) {
  return recentCommands.value.find(command => normalizePath(command.projectPath) === normalizePath(projectPath) && command.commandId === commandId)?.usedAt ?? 0
}

function getScriptPriority(name: string) {
  const index = SCRIPT_PRIORITY.indexOf(name)
  return index === -1 ? SCRIPT_PRIORITY.length : index
}

function getScriptKey(projectPath: string, scriptName: string) {
  return `${normalizePath(projectPath)}::${scriptName}`
}

function isScriptPinned(projectPath: string, scriptName: string) {
  return pinnedScripts.value.has(getScriptKey(projectPath, scriptName))
}

function togglePinnedScript(projectPath: string, scriptName: string) {
  const next = new Set(pinnedScripts.value)
  const key = getScriptKey(projectPath, scriptName)
  if (next.has(key)) {
    next.delete(key)
  } else {
    next.add(key)
  }
  pinnedScripts.value = next
  localStorage.setItem(DEVDOC_PINNED_SCRIPTS_STORAGE_KEY, JSON.stringify([...next]))
}

function loadPinnedScripts() {
  try {
    const raw = localStorage.getItem(DEVDOC_PINNED_SCRIPTS_STORAGE_KEY)
    if (raw) return new Set<string>(JSON.parse(raw))
    const legacyRaw = localStorage.getItem(LEGACY_PINNED_SCRIPTS_STORAGE_KEY)
    const migrated = (legacyRaw ? (JSON.parse(legacyRaw) as string[]) : []).map(key => {
      const separator = key.lastIndexOf('::')
      return separator < 0 ? key : `${key.slice(0, separator + 2)}package:${key.slice(separator + 2)}`
    })
    if (migrated.length) localStorage.setItem(DEVDOC_PINNED_SCRIPTS_STORAGE_KEY, JSON.stringify(migrated))
    return new Set<string>(migrated)
  } catch (err) {
    reportError('devdock.load-pinned-scripts', err)
    return new Set<string>()
  }
}

async function startScript(project: DevDockProject, command: ProjectCommand) {
  recordRecentCommand(project, command)
  await startProjectCommand({
    projectPath: project.path,
    commandId: command.id,
  })
}

async function toggleScript(project: DevDockProject, command: ProjectCommand) {
  const process = findRunningScript(project.path, command.id)
  if (process) {
    await stopProcess(process.id)
    return
  }
  await startScript(project, command)
}

async function startRecentCommand(command: RecentCommand) {
  await startProjectCommand(command)
  recentDrawerOpen.value = false
}

async function startProjectCommand(command: {
  projectPath: string
  commandId: string
}) {
  const key = getScriptKey(command.projectPath, command.commandId)
  startingScripts.add(key)
  try {
    const process = await invokeStartProjectCommand(command)
    updateProcess(process)
    processInspectorOpen.value = true
  } catch (err) {
    message.error(reportError('devdock.start', err), { duration: 8000 })
  } finally {
    startingScripts.delete(key)
    await refreshProcesses()
  }
}

async function refreshProcesses() {
  try {
    processes.value = await listProjectProcesses()
  } catch (err) {
    reportError('devdock.refresh-processes', err)
  }
}

function startProcessPolling() {
  stopProcessPolling()
  processPollTimer = window.setInterval(() => void refreshProcesses(), 5000)
}

function stopProcessPolling() {
  if (processPollTimer) {
    window.clearInterval(processPollTimer)
    processPollTimer = undefined
  }
}

async function stopProcess(processId: string) {
  processBusy.add(processId)
  try {
    await stopProjectProcess(processId)
    processes.value = processes.value.filter(process => process.id !== processId)
  } catch (err) {
    message.error(reportError('devdock.stop', err), { duration: 8000 })
  } finally {
    processBusy.delete(processId)
    await refreshProcesses()
  }
}

async function restartProcess(processId: string) {
  processBusy.add(processId)
  try {
    updateProcess(await restartProjectProcess(processId))
  } catch (err) {
    message.error(reportError('devdock.restart', err), { duration: 8000 })
  } finally {
    processBusy.delete(processId)
    await refreshProcesses()
  }
}

async function openProcessLogs(processId: string) {
  try {
    processLogs.value = await loadProjectProcessLogs(processId)
    logModalOpen.value = true
    startLogPolling(processId)
  } catch (err) {
    message.error(reportError('devdock.open-logs', err))
  }
}

function closeProcessLogs() {
  logModalOpen.value = false
  stopLogPolling()
}

function startLogPolling(processId: string) {
  stopLogPolling()
  logPollTimer = window.setInterval(() => {
    if (!logModalOpen.value) {
      stopLogPolling()
      return
    }
    void refreshProcessLogs(processId)
  }, 1000)
}

function stopLogPolling() {
  if (logPollTimer) {
    window.clearInterval(logPollTimer)
    logPollTimer = undefined
  }
}

async function refreshProcessLogs(processId: string) {
  try {
    processLogs.value = await loadProjectProcessLogs(processId)
    updateProcess(processLogs.value.process)
  } catch (err) {
    reportError('devdock.poll-logs', err)
    stopLogPolling()
  }
}

function updateProcess(process: ProjectProcessSnapshot) {
  const commandId = process.commandId || process.scriptName
  processes.value = [
    process,
    ...processes.value.filter(item =>
      item.id !== process.id && (
        normalizePath(item.projectPath) !== normalizePath(process.projectPath) ||
        (item.commandId || item.scriptName) !== commandId
      )
    ),
  ].sort((left, right) => right.startedAt - left.startedAt)
}

function recordRecentCommand(project: DevDockProject, projectCommand: ProjectCommand) {
  touchProject(project)
  const command: RecentCommand = {
    projectPath: project.path,
    projectName: project.manifest?.name || project.name,
    commandId: projectCommand.id,
    commandName: projectCommand.name,
    commandPreview: projectCommand.commandPreview,
    executor: projectCommand.executor,
    source: projectCommand.source,
    usedAt: Date.now(),
  }
  recentCommands.value = [
    command,
    ...recentCommands.value.filter(
      item => normalizePath(item.projectPath) !== normalizePath(command.projectPath) || item.commandId !== command.commandId,
    ),
  ].slice(0, 12)
  localStorage.setItem(DEVDOC_RECENT_COMMANDS_STORAGE_KEY, JSON.stringify(recentCommands.value))
}

function loadRecentCommands() {
  try {
    const raw = localStorage.getItem(DEVDOC_RECENT_COMMANDS_STORAGE_KEY)
    if (raw) return (JSON.parse(raw) as RecentCommand[])
      .filter(command => command.projectPath && command.commandId)
      .sort((left, right) => right.usedAt - left.usedAt)
      .slice(0, 12)
    const legacyRaw = localStorage.getItem(LEGACY_RECENT_COMMANDS_STORAGE_KEY)
    const legacy = legacyRaw ? (JSON.parse(legacyRaw) as Array<Record<string, unknown>>) : []
    const parsed = legacy.map(item => ({
      projectPath: String(item.projectPath || ''),
      projectName: String(item.projectName || ''),
      commandId: `package:${String(item.scriptName || '')}`,
      commandName: String(item.scriptName || ''),
      commandPreview: String(item.command || ''),
      executor: 'package-script',
      source: 'package-json',
      usedAt: Number(item.usedAt || 0),
    }))
    if (parsed.length) localStorage.setItem(DEVDOC_RECENT_COMMANDS_STORAGE_KEY, JSON.stringify(parsed))
    return parsed
      .filter(command => command.projectPath && command.commandId)
      .sort((left, right) => right.usedAt - left.usedAt)
      .slice(0, 12)
  } catch (err) {
    reportError('devdock.load-recent-commands', err)
    return []
  }
}

function isScriptStarting(projectPath: string, scriptName: string) {
  return startingScripts.has(getScriptKey(projectPath, scriptName))
}

function isScriptRunning(projectPath: string, scriptName: string) {
  return Boolean(findRunningScript(projectPath, scriptName))
}

function findRunningScript(projectPath: string, scriptName: string) {
  const normalized = normalizePath(projectPath)
  return processes.value.find(
    process => normalizePath(process.projectPath) === normalized && (process.commandId || process.scriptName) === scriptName && process.status.state === 'running',
  )
}

function isRecentCommandRunning(command: RecentCommand) {
  return isScriptRunning(command.projectPath, command.commandId)
}

function scriptActionLabel(projectPath: string, scriptName: string) {
  if (isScriptStarting(projectPath, scriptName)) return t('devdock.actions.starting')
  if (isScriptRunning(projectPath, scriptName)) return t('devdock.actions.stop')
  const hasCompletedRun = processes.value.some(process => normalizePath(process.projectPath) === normalizePath(projectPath) && process.commandId === scriptName)
  return hasCompletedRun ? t('devdock.actions.rerunTask') : t('devdock.actions.run')
}

function isProcessBusy(processId: string) {
  return processBusy.has(processId)
}

function processStatusLabel(process: ProjectProcessSnapshot) {
  if (process.status.state === 'running') return t('devdock.processes.running')
  if (process.status.state === 'succeeded') return t('devdock.processes.succeeded')
  if (process.status.state === 'failed') return t('devdock.processes.failed', { code: process.status.exitCode ?? '--' })
  if (process.status.state === 'stopped') return t('devdock.processes.stopped')
  if (process.status.state === 'exited') return t('devdock.processes.exited', { code: process.status.exitCode ?? '--' })
  return t('devdock.processes.unknown')
}

function processUrl(process: ProjectProcessSnapshot) {
  const networkUrl = process.urls.find(url => !/\/\/(localhost|127\.0\.0\.1|0\.0\.0\.0)(?=[:/])/i.test(url))
  return networkUrl || process.urls[0] || ''
}

async function openProcessUrl(process: ProjectProcessSnapshot) {
  const url = processUrl(process)
  if (url) {
    await openProjectUrl(url)
  }
}

async function copyProcessUrl(process: ProjectProcessSnapshot) {
  const url = processUrl(process)
  if (!url) return
  await navigator.clipboard.writeText(url)
}

function getProjectDisplayName(path: string) {
  const normalized = path.replace(/\\/g, '/')
  const parts = normalized.split('/').filter(Boolean)
  return parts[parts.length - 1] || path
}

function normalizePath(path: string) {
  return path.replace(/\\/g, '/').toLowerCase()
}

async function runWithConcurrency<T>(items: T[], limit: number, worker: (item: T) => Promise<void>) {
  const pending = [...items]
  const workerCount = Math.min(limit, pending.length)
  await Promise.all(
    Array.from({ length: workerCount }, async () => {
      while (pending.length) {
        const item = pending.shift()
        if (item) {
          await worker(item)
        }
      }
    }),
  )
}
</script>

<style scoped lang="scss">
.devdock-page {
  background: var(--lumina-content-bg);
  color: var(--lumina-text);
  display: flex;
  flex-direction: column;
  gap: 0;
  height: 100%;
  min-height: 0;
  min-width: 0;
  overflow: hidden;
  padding: 0;
}

.toolbar-tags {
  align-items: center;
  display: flex;
  flex: 1;
  gap: 8px;
  min-width: 0;
  overflow: hidden;
}

.devdock-shell {
  display: grid;
  flex: 1;
  grid-template-columns: minmax(0, 1fr);
  min-height: 0;

  &.inspector-open {
    grid-template-columns: minmax(0, 1fr) minmax(320px, 360px);
  }
}

.source-list {
  border-block: 0;
  border-left: 0;
  border-radius: 0;
  box-shadow: none;
}

@media (max-width: 980px) {
  .devdock-shell.inspector-open {
    grid-template-columns: minmax(0, 1fr) minmax(280px, 320px);
  }
}
</style>
