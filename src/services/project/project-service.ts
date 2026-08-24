import { invoke } from '@tauri-apps/api/core'

export interface ProjectManifest {
  projectPath: string
  packageJsonPath?: string | null
  name?: string | null
  version?: string | null
  packageManager: string
  scripts: ProjectScript[]
  commands: ProjectCommand[]
  candidates: ProjectCommandCandidate[]
  detectedTypes: string[]
  configState: 'default' | 'configured' | 'invalid'
  configError?: string | null
  defaultCommandId?: string | null
  dependenciesCount: number
  devDependenciesCount: number
  detectedStack: string[]
}

export interface ProjectScript {
  name: string
  command: string
}

export type ProjectCommandExecutor = 'package-script' | 'python' | 'python-module' | 'cmd' | 'powershell'

export interface ProjectCommand {
  id: string
  name: string
  executor: ProjectCommandExecutor
  source: 'config' | 'package-json'
  sourceLabel: string
  commandPreview: string
  workingDirectory: string
  runPolicy: 'singleton'
  configRevision: string
  environmentKeys: string[]
}

export interface ProjectCommandCandidate {
  suggestedId: string
  name: string
  executor: ProjectCommandExecutor
  confidence: 'high' | 'medium' | 'low'
  reason: string
  source: string
  draft: Record<string, unknown>
}

export async function discoverProjectCommands(projectPath: string): Promise<ProjectCommandCandidate[]> {
  return await invoke<ProjectCommandCandidate[]>('discover_project_commands', { projectPath })
}

export interface ProjectProcessStatus {
  state: 'starting' | 'running' | 'succeeded' | 'failed' | 'exited' | 'stopped' | 'unknown'
  exitCode?: number | null
  exitedAt?: number | null
}

export interface ProjectProcessSnapshot {
  id: string
  projectPath: string
  projectName: string
  scriptName: string
  command: string
  packageManager: string
  pid: number
  status: ProjectProcessStatus
  startedAt: number
  exitedAt?: number | null
  exitCode?: number | null
  ports: number[]
  urls: string[]
  logCount: number
  lastLogLine?: string | null
  commandId?: string | null
  commandName?: string | null
  executor?: ProjectCommandExecutor | null
  commandPreview?: string | null
  workingDirectory?: string | null
  configRevision?: string | null
  warning?: string | null
}

export interface ProjectProcessLogLine {
  stream: 'stdout' | 'stderr' | 'system'
  text: string
  timestamp: number
}

export interface ProjectProcessLogs {
  process: ProjectProcessSnapshot
  lines: ProjectProcessLogLine[]
}

export async function loadProjectManifest(projectPath: string): Promise<ProjectManifest> {
  return await invoke<ProjectManifest>('load_project_manifest', { projectPath })
}

export async function startProjectProcess(payload: {
  projectPath: string
  projectName?: string
  scriptName: string
  packageManager: string
}): Promise<ProjectProcessSnapshot> {
  return await invoke<ProjectProcessSnapshot>('start_project_process', { payload })
}

export async function startProjectCommand(payload: {
  projectPath: string
  commandId: string
}): Promise<ProjectProcessSnapshot> {
  return await invoke<ProjectProcessSnapshot>('start_project_command', { payload })
}

export interface LuminaProjectConfig {
  schemaVersion: number
  name?: string | null
  types: string[]
  workingDirectory?: string | null
  environment: Record<string, string>
  runtimes: { python?: { interpreter: string } | null }
  commands: Array<Record<string, unknown>>
  commandOverrides: Record<string, { name?: string }>
  defaults: { commandId?: string | null }
}

export async function loadProjectConfig(projectPath: string): Promise<LuminaProjectConfig> {
  return await invoke<LuminaProjectConfig>('load_project_config', { projectPath })
}

export async function validateProjectConfig(config: LuminaProjectConfig): Promise<void> {
  await invoke('validate_project_config', { config })
}

export async function saveProjectConfig(projectPath: string, config: LuminaProjectConfig): Promise<void> {
  await invoke('save_project_config_command', { projectPath, config })
}

export async function listProjectProcesses(): Promise<ProjectProcessSnapshot[]> {
  return await invoke<ProjectProcessSnapshot[]>('list_project_processes')
}

export async function stopProjectProcess(processId: string): Promise<ProjectProcessSnapshot> {
  return await invoke<ProjectProcessSnapshot>('stop_project_process', { processId })
}

export async function restartProjectProcess(processId: string): Promise<ProjectProcessSnapshot> {
  return await invoke<ProjectProcessSnapshot>('restart_project_process', { processId })
}

export async function loadProjectProcessLogs(processId: string): Promise<ProjectProcessLogs> {
  return await invoke<ProjectProcessLogs>('load_project_process_logs', { processId })
}

export async function openProjectUrl(url: string): Promise<void> {
  await invoke('open_project_url', { url })
}

export async function stopAllProjectProcesses(): Promise<ProjectProcessSnapshot[]> {
  return await invoke<ProjectProcessSnapshot[]>('stop_all_project_processes')
}

export interface DevDockProjectRecord {
  path: string
  name: string
  isPinned: boolean
  sortOrder: number
  createdAt: number
  openedAt: number
}

export async function loadDevDockProjects(): Promise<DevDockProjectRecord[]> {
  return await invoke<DevDockProjectRecord[]>('load_devdock_projects')
}

export async function saveDevDockProject(project: DevDockProjectRecord): Promise<void> {
  await invoke('save_devdock_project', { project })
}

export async function removeDevDockProject(path: string): Promise<void> {
  await invoke('remove_devdock_project', { path })
}

export interface DevDockRunHistoryRecord {
  id: string
  projectPath: string
  projectName: string
  commandId: string
  commandName: string
  executor: string
  commandPreview?: string | null
  exitCode?: number | null
  status: string
  startedAt: number
  durationMs: number
  lastLogLine?: string | null
  expiresAt?: number | null
}

export async function loadDevDockRunHistory(
  projectPath?: string,
  limit?: number,
): Promise<DevDockRunHistoryRecord[]> {
  return await invoke<DevDockRunHistoryRecord[]>('load_devdock_run_history', {
    projectPath: projectPath || null,
    limit: limit || null,
  })
}

export async function clearDevDockRunHistory(
  projectPath?: string,
): Promise<void> {
  await invoke('clear_devdock_run_history', {
    projectPath: projectPath || null,
  })
}
