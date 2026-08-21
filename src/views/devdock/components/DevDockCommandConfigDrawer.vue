<template>
  <WorkbenchDrawer v-if="show" fixed size="wide" :title="t('devdock.config.title')" :description="projectPath" :close-label="t('common.close')" @close="$emit('close')">
    <div class="config-layout">
      <section class="config-section">
        <h4>{{ t('devdock.config.project') }}</h4>
        <div class="field-grid">
          <label><span>{{ t('devdock.config.name') }}</span><input v-model="config.name" type="text" /></label>
          <label><span>{{ t('devdock.config.pythonInterpreter') }}</span><input v-model="pythonInterpreter" type="text" placeholder="python" /></label>
          <label><span>{{ t('devdock.config.workingDirectory') }}</span><input v-model="config.workingDirectory" type="text" placeholder="." /></label>
          <label><span>{{ t('devdock.config.defaultCommand') }}</span><select v-model="config.defaults.commandId"><option value="">{{ t('devdock.config.noDefaultCommand') }}</option><option v-for="command in defaultCommands" :key="command.id" :value="command.id">{{ command.name || command.id }}</option></select></label>
        </div>
        <div class="array-editor">
          <span>{{ t('devdock.config.environment') }}</span>
          <div v-for="(row, rowIndex) in environmentRows" :key="row.keyId" class="environment-row"><input v-model="row.name" type="text" placeholder="NAME" /><input v-model="row.value" type="password" placeholder="••••••" /><button class="icon-action" type="button" :aria-label="t('common.delete')" @click="environmentRows.splice(rowIndex, 1)"><Icon icon="solar:minus-circle-linear" /></button></div>
          <button class="add-row" type="button" @click="environmentRows.push({ keyId: `env-${nextKey++}`, name: '', value: '' })"><Icon icon="solar:add-circle-linear" />{{ t('devdock.config.addVariable') }}</button>
        </div>
      </section>

      <section class="config-section commands-section">
        <header><h4>{{ t('devdock.config.commands') }}</h4><button type="button" @click="addCommand"><Icon icon="solar:add-circle-linear" />{{ t('devdock.config.addCommand') }}</button></header>
        <div v-if="commands.length" class="command-editor-list">
          <article v-for="(command, index) in commands" :key="command.key" class="command-editor">
            <div class="command-editor-head">
              <input v-model="command.name" type="text" :placeholder="t('devdock.config.commandName')" />
              <button class="icon-action danger" type="button" :aria-label="t('common.delete')" @click="commands.splice(index, 1)"><Icon icon="solar:trash-bin-trash-linear" /></button>
            </div>
            <div class="field-grid compact">
              <label><span>ID</span><input v-model="command.id" type="text" /></label>
              <label><span>{{ t('devdock.config.executor') }}</span><select v-model="command.executor"><option value="python">Python script</option><option value="python-module">Python module</option><option value="cmd">CMD / BAT</option><option value="powershell">PowerShell</option></select></label>
              <label v-if="command.executor === 'python-module'"><span>{{ t('devdock.config.module') }}</span><input v-model="command.module" type="text" placeholder="uvicorn" /></label>
              <label v-else><span>{{ t('devdock.config.script') }}</span><input v-model="command.script" type="text" /></label>
              <label><span>{{ t('devdock.config.workingDirectory') }}</span><input v-model="command.workingDirectory" type="text" placeholder="." /></label>
            </div>
            <div class="array-editor">
              <span>{{ t('devdock.config.arguments') }}</span>
              <div v-for="(_, argumentIndex) in command.args" :key="argumentIndex" class="array-row"><input v-model="command.args[argumentIndex]" type="text" /><button class="icon-action" type="button" :aria-label="t('common.delete')" @click="command.args.splice(argumentIndex, 1)"><Icon icon="solar:minus-circle-linear" /></button></div>
              <button class="add-row" type="button" @click="command.args.push('')"><Icon icon="solar:add-circle-linear" />{{ t('devdock.config.addArgument') }}</button>
            </div>
            <div class="array-editor">
              <span>{{ t('devdock.config.commandEnvironment') }}</span>
              <div v-for="(row, rowIndex) in command.environmentRows" :key="row.keyId" class="environment-row"><input v-model="row.name" type="text" placeholder="NAME" /><input v-model="row.value" type="password" placeholder="••••••" /><button class="icon-action" type="button" :aria-label="t('common.delete')" @click="command.environmentRows.splice(rowIndex, 1)"><Icon icon="solar:minus-circle-linear" /></button></div>
              <button class="add-row" type="button" @click="command.environmentRows.push({ keyId: `command-env-${nextKey++}`, name: '', value: '' })"><Icon icon="solar:add-circle-linear" />{{ t('devdock.config.addVariable') }}</button>
            </div>
          </article>
        </div>
        <div v-else class="config-empty">{{ t('devdock.config.noCommands') }}</div>
      </section>

      <section v-if="packageOverrides.length" class="config-section">
        <h4>{{ t('devdock.config.packageOverrides') }}</h4>
        <div class="override-list">
          <article v-for="item in packageOverrides" :key="item.id" class="override-row">
            <code>{{ item.id }}</code>
            <input v-model="item.name" type="text" />
          </article>
        </div>
      </section>

      <section v-if="candidates.length" class="config-section">
        <h4>{{ t('devdock.config.candidates') }}</h4>
        <div class="candidate-list">
          <article v-for="candidate in candidates" :key="`${candidate.executor}:${candidate.source}`">
            <div><strong>{{ candidate.name }}</strong><span>{{ candidate.reason }} · {{ candidate.source }}</span></div>
            <button type="button" @click="addCandidate(candidate)">{{ t('devdock.config.add') }}</button>
          </article>
        </div>
      </section>

      <p v-if="error" class="config-error">{{ error }}</p>
      <footer><button type="button" @click="$emit('close')">{{ t('common.cancel') }}</button><button class="primary" type="button" :disabled="loading || saving" @click="save">{{ saving ? t('devdock.config.saving') : t('common.save') }}</button></footer>
    </div>
  </WorkbenchDrawer>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import WorkbenchDrawer from '@/components/workbench/WorkbenchDrawer.vue'
import { useLocale } from '@/hooks/useLocale'
import { loadProjectConfig, saveProjectConfig, validateProjectConfig, type LuminaProjectConfig, type ProjectCommand, type ProjectCommandCandidate } from '@/services/project/project-service'

interface EditableEnvironmentRow {
  keyId: string
  name: string
  value: string
}

interface EditableCommand {
  key: string
  id: string
  name: string
  executor: 'python' | 'python-module' | 'cmd' | 'powershell'
  script: string
  module: string
  args: string[]
  workingDirectory: string
  environmentRows: EditableEnvironmentRow[]
}

const props = defineProps<{ candidates: ProjectCommandCandidate[]; projectCommands: ProjectCommand[]; projectPath: string; show: boolean }>()
const emit = defineEmits<{ (e: 'close'): void; (e: 'saved'): void }>()
const { t } = useLocale()
const config = reactive<LuminaProjectConfig>(emptyConfig())
const commands = ref<EditableCommand[]>([])
const pythonInterpreter = ref('python')
const environmentRows = ref<Array<{ keyId: string; name: string; value: string }>>([])
const packageOverrides = ref<Array<{ id: string; name: string }>>([])
const loading = ref(false)
const saving = ref(false)
const error = ref('')
let nextKey = 0
const defaultCommands = computed(() => {
  const items = new Map<string, string>()
  packageOverrides.value.forEach(command => items.set(command.id, command.name || command.id))
  commands.value.forEach(command => {
    if (command.id) items.set(command.id, command.name || command.id)
  })
  return Array.from(items, ([id, name]) => ({ id, name }))
})

watch(() => [props.show, props.projectPath] as const, ([show]) => { if (show) void load() }, { immediate: true })

async function load() {
  loading.value = true
  error.value = ''
  try {
    const loaded = await loadProjectConfig(props.projectPath)
    Object.assign(config, emptyConfig(), loaded)
    pythonInterpreter.value = loaded.runtimes.python?.interpreter || 'python'
    environmentRows.value = Object.entries(loaded.environment).map(([name, value]) => ({ keyId: `env-${nextKey++}`, name, value }))
    commands.value = loaded.commands.map(toEditableCommand)
    packageOverrides.value = props.projectCommands.filter(command => command.source === 'package-json').map(command => ({
      id: command.id,
      name: loaded.commandOverrides[command.id]?.name || command.name,
    }))
  } catch (cause) {
    error.value = cause instanceof Error ? cause.message : String(cause)
  } finally {
    loading.value = false
  }
}

function addCommand() {
  commands.value.push({ key: `new-${nextKey++}`, id: '', name: '', executor: 'python', script: '', module: '', args: [], workingDirectory: '.', environmentRows: [] })
}

function addCandidate(candidate: ProjectCommandCandidate) {
  const draft = candidate.draft
  if (draft.interpreter) pythonInterpreter.value = String(draft.interpreter)
  commands.value.push({
    key: `candidate-${nextKey++}`,
    id: uniqueId(candidate.suggestedId),
    name: candidate.name,
    executor: candidate.executor as EditableCommand['executor'],
    script: String(draft.script || ''),
    module: String(draft.module || ''),
    args: Array.isArray(draft.args) ? draft.args.map(String) : [],
    workingDirectory: '.',
    environmentRows: [],
  })
}

async function save() {
  saving.value = true
  error.value = ''
  try {
    const usesPython = commands.value.some(command => command.executor === 'python' || command.executor === 'python-module') || config.types.includes('python')
    const payload: LuminaProjectConfig = {
      ...config,
      name: config.name?.trim() || null,
      workingDirectory: config.workingDirectory?.trim() || '.',
      environment: Object.fromEntries(environmentRows.value.filter(row => row.name.trim()).map(row => [row.name.trim(), row.value])),
      types: usesPython ? Array.from(new Set([...config.types, 'python'])) : config.types,
      runtimes: usesPython ? { ...config.runtimes, python: { interpreter: pythonInterpreter.value.trim() || 'python' } } : config.runtimes,
      commands: commands.value.map(toConfigCommand),
      schemaVersion: 2,
      commandOverrides: Object.fromEntries(packageOverrides.value.map(item => [item.id, { name: item.name.trim() || undefined }])),
      defaults: { commandId: config.defaults.commandId || null },
    }
    await validateProjectConfig(payload)
    await saveProjectConfig(props.projectPath, payload)
    emit('saved')
  } catch (cause) {
    error.value = cause instanceof Error ? cause.message : String(cause)
  } finally {
    saving.value = false
  }
}

function toEditableCommand(command: Record<string, unknown>): EditableCommand {
  const environment = typeof command.environment === 'object' && command.environment ? command.environment as Record<string, string> : {}
  return { key: `loaded-${nextKey++}`, id: String(command.id || ''), name: String(command.name || ''), executor: command.executor as EditableCommand['executor'], script: String(command.script || ''), module: String(command.module || ''), args: Array.isArray(command.args) ? command.args.map(String) : [], workingDirectory: String(command.workingDirectory || '.'), environmentRows: Object.entries(environment).map(([name, value]) => ({ keyId: `command-env-${nextKey++}`, name, value })) }
}

function toConfigCommand(command: EditableCommand): Record<string, unknown> {
  const environment = Object.fromEntries(command.environmentRows.filter(row => row.name.trim()).map(row => [row.name.trim(), row.value]))
  const result: Record<string, unknown> = { id: command.id.trim(), name: command.name.trim(), executor: command.executor, args: command.args, workingDirectory: command.workingDirectory || '.', environment, runPolicy: 'singleton' }
  if (command.executor === 'python-module') result.module = command.module.trim()
  else result.script = command.script.trim()
  return result
}

function uniqueId(base: string) {
  let id = base || 'command'
  let suffix = 2
  while (commands.value.some(command => command.id === id)) id = `${base}-${suffix++}`
  return id
}

function emptyConfig(): LuminaProjectConfig {
  return { schemaVersion: 2, name: null, types: [], workingDirectory: '.', environment: {}, runtimes: {}, commands: [], commandOverrides: {}, defaults: {} }
}
</script>

<style scoped lang="scss">
.config-layout { display: grid; gap: 16px; padding: 14px; }
.config-section { display: grid; gap: 10px; }
.config-section > h4, .config-section > header h4 { font-size: 13px; margin: 0; }
.config-section > header { align-items: center; display: flex; justify-content: space-between; }
.config-section > header > button { align-items: center; display: inline-flex; gap: 5px; }
footer { display: flex; gap: 6px; }
.field-grid { display: grid; gap: 8px; grid-template-columns: repeat(3, minmax(0, 1fr)); }
.field-grid.compact { grid-template-columns: repeat(2, minmax(0, 1fr)); }
label, .array-editor { display: grid; gap: 5px; }
label span, .array-editor > span { color: var(--lumina-text-secondary); font-size: 10px; }
input, select { background: var(--lumina-input-bg); border: 1px solid var(--lumina-card-border); border-radius: var(--lumina-radius-sm); color: var(--lumina-text); font: inherit; font-size: 12px; height: 30px; min-width: 0; padding: 0 8px; }
button { background: var(--lumina-button-secondary-bg); border: 1px solid var(--lumina-card-border); border-radius: var(--lumina-radius-sm); color: var(--lumina-text); cursor: pointer; font-size: 11px; min-height: 28px; padding: 0 9px; }
button.primary { background: var(--lumina-primary); border-color: var(--lumina-primary); color: var(--lumina-on-accent); }
.command-editor-list, .candidate-list { display: grid; gap: 8px; }
.command-editor { background: color-mix(in srgb, var(--lumina-surface-2) 68%, transparent); border: 0.5px solid var(--lumina-card-border); border-radius: var(--lumina-radius-md); display: grid; gap: 10px; padding: 10px; }
.command-editor-head { display: grid; gap: 7px; grid-template-columns: minmax(0, 1fr) 30px; }
.icon-action { align-items: center; display: inline-flex; justify-content: center; padding: 0; width: 30px; }
.icon-action.danger { color: var(--lumina-danger); }
.array-row { display: grid; gap: 6px; grid-template-columns: minmax(0, 1fr) 30px; }
.environment-row { display: grid; gap: 6px; grid-template-columns: minmax(120px, 0.7fr) minmax(0, 1.3fr) 30px; }
.override-list { display: grid; gap: 6px; }
.override-row { align-items: center; display: grid; gap: 8px; grid-template-columns: minmax(120px, 0.8fr) minmax(0, 1.2fr); }
.override-row code { color: var(--lumina-text-secondary); font-size: 10px; }
.add-row { align-items: center; display: inline-flex; gap: 5px; justify-self: start; }
.candidate-list article { align-items: center; border-bottom: 1px solid var(--lumina-card-border); display: flex; gap: 10px; justify-content: space-between; padding: 7px 0; }
.candidate-list article div { display: grid; gap: 3px; }
.candidate-list span, .config-empty { color: var(--lumina-text-secondary); font-size: 11px; }
.config-error { color: var(--lumina-danger); font-size: 11px; margin: 0; white-space: pre-wrap; }
footer { border-top: 1px solid var(--lumina-card-border); justify-content: flex-end; padding-top: 12px; }
@media (max-width: 900px) { .field-grid, .field-grid.compact { grid-template-columns: 1fr; } }
</style>
