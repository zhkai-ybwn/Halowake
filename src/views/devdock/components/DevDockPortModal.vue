<template>
  <NModal
    :show="show"
    class="port-inspector-modal"
    :mask-closable="true"
    :auto-focus="false"
    :trap-focus="false"
    @update:show="handleShowUpdate"
  >
    <WorkbenchSheet
      size="normal"
      icon="solar:server-minimalistic-linear"
      :title="t('devdock.portInspector.title')"
      :description="t('devdock.portInspector.subtitle')"
      :close-label="t('common.close')"
      @close="emit('close')"
    >
      <div class="port-modal-body">
        <!-- Quick Ports Bar -->
        <section class="quick-ports-section">
          <span class="quick-ports-label">{{ t('devdock.portInspector.quickPorts') }}:</span>
          <div class="quick-ports-chips">
            <!-- Active DevDock ports first if any -->
            <button
              v-for="port in activeDevDockPorts"
              :key="`devdock-${port}`"
              type="button"
              class="port-chip is-devdock"
              :class="{ active: portInput === String(port) }"
              :title="`${t('devdock.portInspector.devdockPorts')}: ${port}`"
              @click="selectPort(port)"
            >
              <Icon icon="solar:bolt-circle-bold" class="chip-icon" />
              <span>{{ port }}</span>
            </button>

            <!-- Standard common ports -->
            <button
              v-for="port in commonPorts"
              :key="`common-${port}`"
              type="button"
              class="port-chip"
              :class="{ active: portInput === String(port) }"
              @click="selectPort(port)"
            >
              <span>{{ port }}</span>
            </button>
          </div>
        </section>

        <!-- Search Input Bar -->
        <div class="port-input-row">
          <NInput
            ref="inputRef"
            v-model:value="portInput"
            size="small"
            clearable
            :placeholder="t('devdock.portInspector.portPlaceholder')"
            class="port-search-input"
            @keydown.enter.prevent="handleScan"
          >
            <template #prefix>
              <Icon icon="solar:magnifer-linear" class="search-prefix-icon" />
            </template>
          </NInput>

          <WorkbenchButton
            variant="primary"
            :disabled="scanning || !portInput.trim()"
            @click="handleScan"
          >
            <Icon v-if="scanning" icon="solar:restart-linear" class="spin" />
            <Icon v-else icon="solar:radar-linear" />
            <span>{{ scanning ? t('devdock.portInspector.scanning') : t('devdock.portInspector.scan') }}</span>
          </WorkbenchButton>
        </div>

        <!-- Content Area -->
        <div class="port-results-area">
          <!-- Initial State (Before Any Scan) -->
          <WorkbenchEmptyState
            v-if="scannedPort === null && !scanning"
            icon="solar:tuning-square-linear"
            :title="t('devdock.portInspector.emptyInitial')"
          />

          <!-- Loading State -->
          <WorkbenchEmptyState
            v-else-if="scanning"
            icon="solar:restart-linear"
            :title="t('devdock.portInspector.scanning')"
            class="is-loading-state"
          />

          <!-- Free State (Port is Available) -->
          <WorkbenchEmptyState
            v-else-if="scannedPort !== null && processes.length === 0"
            icon="solar:check-circle-bold"
            :title="t('devdock.portInspector.emptyStateFree', { port: scannedPort })"
            :description="t('devdock.portInspector.emptyStateFreeHint')"
            class="is-free-state"
          />

          <!-- Occupied State (Processes Found) -->
          <div v-else-if="processes.length > 0" class="processes-list-wrapper">
            <div class="results-summary-bar">
              <div class="summary-left">
                <WorkbenchTag
                  :label="t('devdock.portInspector.foundProcesses', { count: processes.length })"
                  :value="`:${scannedPort}`"
                  tone="danger"
                />
              </div>

              <div class="summary-actions">
                <WorkbenchButton
                  :disabled="scanning"
                  @click="handleScan"
                >
                  <Icon icon="solar:restart-linear" :class="{ spin: scanning }" />
                  <span>{{ t('devdock.portInspector.refresh') }}</span>
                </WorkbenchButton>

                <WorkbenchButton
                  v-if="processes.length > 1"
                  variant="danger"
                  :disabled="isKillingAny"
                  @click="handleKillAll"
                >
                  <Icon icon="solar:trash-bin-trash-linear" />
                  <span>{{ t('devdock.portInspector.killAll') }}</span>
                </WorkbenchButton>
              </div>
            </div>

            <div class="process-card-list">
              <article
                v-for="proc in processes"
                :key="proc.pid"
                class="process-card"
                :class="{ 'is-managed': proc.isDevdockManaged }"
              >
                <!-- Card Header: Title, Tags, Action Button -->
                <div class="card-header">
                  <div class="card-headline">
                    <strong class="process-name">{{ proc.name }}</strong>
                    <WorkbenchTag label="PID" :value="proc.pid" />
                    <WorkbenchTag
                      :label="proc.protocol"
                      :value="proc.state"
                      :tone="proc.state === 'LISTENING' ? 'warning' : 'default'"
                    />
                    <WorkbenchTag
                      v-if="proc.isDevdockManaged"
                      label="DevDock"
                      :value="proc.commandName || proc.projectName || ''"
                      tone="primary"
                    />
                  </div>

                  <div class="card-action">
                    <WorkbenchButton
                      variant="danger"
                      :disabled="killingPid === proc.pid || isKillingAny"
                      @click="handleKillProcess(proc)"
                    >
                      <Icon v-if="killingPid === proc.pid" icon="solar:restart-linear" class="spin" />
                      <Icon v-else icon="solar:stop-circle-linear" />
                      <span>{{ t('devdock.portInspector.killProcess') }}</span>
                    </WorkbenchButton>
                  </div>
                </div>

                <!-- Card Meta: Full Width Details Without Squeeze -->
                <div class="card-meta-list">
                  <!-- Managed Project info if available -->
                  <div v-if="proc.isDevdockManaged && proc.projectName" class="meta-row is-managed-meta">
                    <Icon icon="solar:folder-with-files-linear" class="meta-icon" />
                    <span class="meta-label">{{ t('devdock.overview.project') }}:</span>
                    <span class="meta-value">{{ proc.projectName }} / {{ proc.commandName }}</span>
                  </div>

                  <!-- Socket Address info -->
                  <div class="meta-row">
                    <Icon icon="solar:link-minimalistic-2-linear" class="meta-icon" />
                    <span class="meta-label">{{ t('devdock.portInspector.localAddress') }}:</span>
                    <span class="meta-value address-value">{{ proc.localAddress }}</span>
                  </div>

                  <!-- Executable Path if available -->
                  <div v-if="proc.path" class="meta-row" :title="proc.path">
                    <Icon icon="solar:file-code-linear" class="meta-icon" />
                    <span class="meta-label">{{ t('devdock.portInspector.path') }}:</span>
                    <span class="meta-value path-value">{{ proc.path }}</span>
                  </div>
                </div>
              </article>
            </div>
          </div>
        </div>
      </div>

      <template #footer>
        <div class="modal-footer-content">
          <span v-if="scannedPort !== null" class="footer-summary">
            <template v-if="processes.length">
              {{ t('devdock.portInspector.foundProcesses', { count: processes.length }) }}
            </template>
            <template v-else>
              {{ t('devdock.portInspector.emptyStateFree', { port: scannedPort }) }}
            </template>
          </span>
          <span v-else></span>
          <WorkbenchButton @click="emit('close')">
            {{ t('common.close') }}
          </WorkbenchButton>
        </div>
      </template>
    </WorkbenchSheet>
  </NModal>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { NInput, NModal, useDialog, useMessage } from 'naive-ui'
import { Icon } from '@iconify/vue'
import WorkbenchSheet from '@/components/workbench/WorkbenchSheet.vue'
import WorkbenchButton from '@/components/workbench/WorkbenchButton.vue'
import WorkbenchTag from '@/components/workbench/WorkbenchTag.vue'
import WorkbenchEmptyState from '@/components/workbench/WorkbenchEmptyState.vue'
import { useLocale } from '@/composables/useLocale'
import {
  findProcessesByPort,
  killProcessByPid,
  type PortProcessInfo,
} from '@/services/project/project-service'

const props = withDefaults(
  defineProps<{
    show: boolean
    initialPort?: number | null
    activePorts?: number[]
  }>(),
  {
    initialPort: null,
    activePorts: () => [],
  },
)

const emit = defineEmits<{
  (e: 'update:show', value: boolean): void
  (e: 'close'): void
  (e: 'processKilled', pid: number): void
}>()

const { t } = useLocale()
const dialog = useDialog()
const message = useMessage()

const portInput = ref('')
const scannedPort = ref<number | null>(null)
const scanning = ref(false)
const processes = ref<PortProcessInfo[]>([])
const killingPid = ref<number | null>(null)
const isKillingAny = ref(false)
const inputRef = ref<InstanceType<typeof NInput> | null>(null)

const defaultCommonPorts = [3000, 5173, 8080, 8000, 4200, 8888, 9000, 3306, 5432]

const activeDevDockPorts = computed(() => {
  const set = new Set<number>()
  for (const p of props.activePorts) {
    if (p > 0 && p <= 65535) {
      set.add(p)
    }
  }
  return Array.from(set)
})

const commonPorts = computed(() => {
  const activeSet = new Set(activeDevDockPorts.value)
  return defaultCommonPorts.filter(p => !activeSet.has(p))
})

watch(
  () => props.show,
  isShown => {
    if (isShown) {
      if (props.initialPort && props.initialPort > 0) {
        portInput.value = String(props.initialPort)
        nextTick(() => {
          handleScan()
        })
      } else {
        scannedPort.value = null
        processes.value = []
        nextTick(() => {
          inputRef.value?.focus()
        })
      }
    }
  },
  { immediate: true },
)

function handleShowUpdate(val: boolean) {
  emit('update:show', val)
  if (!val) {
    emit('close')
  }
}

function selectPort(port: number) {
  portInput.value = String(port)
  handleScan()
}

async function handleScan() {
  const raw = portInput.value.trim()
  const port = parseInt(raw, 10)
  if (!raw || isNaN(port) || port < 1 || port > 65535) {
    message.warning(t('devdock.portInspector.invalidPort'))
    return
  }

  scanning.value = true
  try {
    const list = await findProcessesByPort(port)
    processes.value = list
    scannedPort.value = port
  } catch (err: unknown) {
    const errorText = err instanceof Error ? err.message : String(err)
    message.error(errorText || '扫描端口失败')
  } finally {
    scanning.value = false
  }
}

function handleKillProcess(proc: PortProcessInfo) {
  dialog.warning({
    title: t('devdock.portInspector.killConfirmTitle'),
    content: t('devdock.portInspector.killConfirmContent', {
      name: proc.name,
      pid: proc.pid,
    }),
    positiveText: t('devdock.portInspector.killProcess'),
    negativeText: t('common.cancel'),
    onPositiveClick: async () => {
      killingPid.value = proc.pid
      try {
        await killProcessByPid(proc.pid)
        message.success(t('devdock.portInspector.killSuccess', { pid: proc.pid }))
        emit('processKilled', proc.pid)
        // Refresh port results after kill
        await handleScan()
      } catch (err: unknown) {
        const errorText = err instanceof Error ? err.message : String(err)
        message.error(t('devdock.portInspector.killFailed', { error: errorText }))
      } finally {
        killingPid.value = null
      }
    },
  })
}

function handleKillAll() {
  dialog.warning({
    title: t('devdock.portInspector.killConfirmTitle'),
    content: `即将终止所有占用端口 ${scannedPort.value} 的进程（共 ${processes.value.length} 个），确定继续吗？`,
    positiveText: t('devdock.portInspector.killAll'),
    negativeText: t('common.cancel'),
    onPositiveClick: async () => {
      isKillingAny.value = true
      let successCount = 0
      for (const proc of [...processes.value]) {
        try {
          await killProcessByPid(proc.pid)
          successCount++
          emit('processKilled', proc.pid)
        } catch (err) {
          console.error(`Failed to kill PID ${proc.pid}:`, err)
        }
      }
      isKillingAny.value = false
      message.success(`已处理完成，成功终止 ${successCount} 个进程`)
      await handleScan()
    },
  })
}
</script>

<style scoped lang="scss">
:deep(.workbench-sheet) {
  max-height: min(700px, calc(100vh - 64px));
  width: min(720px, calc(100vw - 48px));
}

.port-modal-body {
  display: flex;
  flex-direction: column;
  gap: 14px;
  min-height: 0;
  overflow-x: hidden;
}

/* Quick Ports Bar */
.quick-ports-section {
  align-items: center;
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.quick-ports-label {
  color: var(--lumina-text-secondary);
  font-size: 12px;
}

.quick-ports-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.port-chip {
  align-items: center;
  background: var(--lumina-control-bg);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-xs);
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: inline-flex;
  font-family: var(--lumina-font-mono);
  font-size: 11px;
  gap: 4px;
  height: 24px;
  line-height: 1;
  padding: 0 8px;
  transition: all var(--lumina-duration-fast) var(--lumina-ease-out);

  &:hover {
    background: var(--lumina-control-hover);
    color: var(--lumina-text);
  }

  &.active {
    background: color-mix(in srgb, var(--lumina-primary) 18%, var(--lumina-surface-2));
    border-color: var(--lumina-primary);
    color: var(--lumina-primary);
    font-weight: 600;
  }

  &.is-devdock {
    background: color-mix(in srgb, var(--lumina-primary) 10%, transparent);
    border-color: color-mix(in srgb, var(--lumina-primary) 28%, transparent);
    color: var(--lumina-primary);

    &.active {
      background: color-mix(in srgb, var(--lumina-primary) 24%, var(--lumina-surface-2));
      border-color: var(--lumina-primary);
      color: var(--lumina-primary);
    }

    .chip-icon {
      font-size: 12px;
    }
  }
}

/* Port Input Section */
.port-input-row {
  align-items: center;
  display: flex;
  gap: 10px;
}

.port-search-input {
  flex: 1;

  .search-prefix-icon {
    color: var(--lumina-text-tertiary);
    font-size: 15px;
  }
}

/* Results Content Area */
.port-results-area {
  display: flex;
  flex: 1;
  flex-direction: column;
  max-height: 420px;
  min-height: 240px;
  overflow-x: hidden;
  overflow-y: auto;
  padding: 2px;
}

.is-loading-state {
  :deep(.workbench-empty-icon) {
    animation: icon-spin 1s linear infinite;
    color: var(--lumina-primary);
  }
}

.is-free-state {
  :deep(.workbench-empty-icon) {
    background: color-mix(in srgb, var(--lumina-success) 14%, transparent);
    color: var(--lumina-success);
  }

  :deep(h3) {
    color: var(--lumina-success);
  }
}

/* Occupied State */
.processes-list-wrapper {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.results-summary-bar {
  align-items: center;
  display: flex;
  justify-content: space-between;
  padding-bottom: 2px;
}

.summary-left {
  align-items: center;
  display: flex;
  gap: 8px;
}

.summary-actions {
  display: flex;
  gap: 8px;
}

.process-card-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.process-card {
  background: var(--lumina-surface-2);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-md);
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px 14px;
  transition: all var(--lumina-duration-fast) var(--lumina-ease-out);

  &:hover {
    background: var(--lumina-surface-3);
    border-color: var(--lumina-separator-strong);
  }

  &.is-managed {
    border-left: 3px solid var(--lumina-primary);
  }
}

.card-header {
  align-items: center;
  display: flex;
  gap: 12px;
  justify-content: space-between;
}

.card-headline {
  align-items: center;
  display: flex;
  flex: 1;
  flex-wrap: wrap;
  gap: 8px;
  min-width: 0;
}

.process-name {
  color: var(--lumina-text);
  font-size: 14px;
  font-weight: 600;
  line-height: 1;
}

.card-action {
  flex-shrink: 0;
}

.card-meta-list {
  display: flex;
  flex-direction: column;
  gap: 5px;
  min-width: 0;
}

.meta-row {
  align-items: center;
  color: var(--lumina-text-secondary);
  display: flex;
  font-size: 12px;
  gap: 6px;
  min-width: 0;

  &.is-managed-meta {
    color: var(--lumina-primary);
  }
}

.meta-icon {
  color: var(--lumina-text-tertiary);
  flex-shrink: 0;
  font-size: 14px;
}

.meta-label {
  color: var(--lumina-text-tertiary);
  flex-shrink: 0;
}

.meta-value {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.address-value {
  font-family: var(--lumina-font-mono);
}

.path-value {
  color: var(--lumina-text-tertiary);
  font-family: var(--lumina-font-mono);
}

/* Modal Footer */
.modal-footer-content {
  align-items: center;
  display: flex;
  justify-content: space-between;
  width: 100%;
}

.footer-summary {
  color: var(--lumina-text-secondary);
  font-size: 12px;
}

@keyframes icon-spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}

.spin {
  animation: icon-spin 1s linear infinite;
}
</style>
