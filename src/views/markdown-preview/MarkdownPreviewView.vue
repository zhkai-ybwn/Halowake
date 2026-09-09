<template>
  <div class="md-preview-page">
    <!-- Topbar Header -->
    <header class="md-topbar" data-tauri-drag-region>
      <div class="md-topbar-leading">
        <div class="md-file-badge">
          <Icon icon="solar:document-text-linear" class="md-file-badge-icon" />
          <div class="md-file-info" :title="currentFilePath">
            <strong class="md-file-name">{{ currentFileName || t('markdownPreview.title') }}</strong>
            <span v-if="currentFilePath" class="md-file-path">{{ currentFilePath }}</span>
            <span v-else class="md-file-hint">{{ t('markdownPreview.noFileSelected') }}</span>
          </div>
        </div>
      </div>

      <!-- Center / Stats -->
      <div v-if="currentFileContent" class="md-topbar-center">
        <span class="md-stat-tag" :title="t('markdownPreview.wordCountTooltip')">
          <Icon icon="solar:text-square-linear" />
          <span>{{ stats.wordCount }} {{ t('markdownPreview.words') }}</span>
        </span>
        <span class="md-stat-tag" :title="t('markdownPreview.readingTimeTooltip')">
          <Icon icon="solar:clock-circle-linear" />
          <span>~{{ stats.readingTimeMinutes }} {{ t('markdownPreview.minutes') }}</span>
        </span>
      </div>

      <!-- Actions -->
      <div class="md-topbar-actions">
        <!-- Open File -->
        <WorkbenchButton size="small" :title="t('markdownPreview.openFileHint')" @click="selectAndOpenFile">
          <Icon icon="solar:folder-open-linear" />
          <span>{{ t('markdownPreview.openFile') }}</span>
        </WorkbenchButton>

        <!-- Recent Files Dropdown -->
        <NDropdown
          v-if="historyStore.recentFiles.length"
          trigger="click"
          :options="recentDropdownOptions"
          @select="handleRecentSelect"
        >
          <WorkbenchIconButton
            icon="solar:history-linear"
            :label="t('markdownPreview.recentFiles')"
            :active="false"
          />
        </NDropdown>

        <span v-if="currentFileContent" class="action-divider"></span>

        <!-- Search in doc -->
        <WorkbenchIconButton
          v-if="currentFileContent"
          icon="solar:magnifer-linear"
          :label="t('markdownPreview.searchShortcut')"
          @click="triggerSearch"
        />

        <!-- Reading Width Toggle -->
        <WorkbenchIconButton
          v-if="currentFileContent"
          :icon="historyStore.readingWidth === 'full' ? 'solar:minimize-square-minimalistic-linear' : 'solar:maximize-square-minimalistic-linear'"
          :label="historyStore.readingWidth === 'full' ? t('markdownPreview.widthCentered') : t('markdownPreview.widthFull')"
          :active="historyStore.readingWidth === 'full'"
          @click="historyStore.toggleReadingWidth"
        />

        <!-- Font Size Toggle -->
        <WorkbenchIconButton
          v-if="currentFileContent"
          icon="solar:text-size-linear"
          :label="t('markdownPreview.fontSizeCycle')"
          @click="historyStore.cycleFontSize"
        />

        <!-- TOC Toggle -->
        <WorkbenchIconButton
          v-if="currentFileContent"
          icon="solar:list-linear"
          :label="t('markdownPreview.toggleOutline')"
          shortcut="Ctrl+Shift+O"
          :active="historyStore.tocOpen"
          @click="historyStore.toggleToc"
        />

        <!-- Reload -->
        <WorkbenchIconButton
          v-if="currentFileContent"
          icon="solar:refresh-linear"
          :label="t('markdownPreview.reload')"
          :disabled="loading"
          @click="reloadCurrentFile"
        />

        <!-- Open in Standalone Popout Window -->
        <WorkbenchIconButton
          v-if="currentFileContent"
          icon="solar:square-top-down-linear"
          :label="t('markdownPreview.openInWindow')"
          @click="openInStandaloneWindow"
        />

        <!-- Open in External Editor (VS Code etc.) -->
        <WorkbenchIconButton
          v-if="currentFileContent"
          icon="solar:pen-linear"
          :label="t('markdownPreview.openInEditor')"
          @click="openInExternalEditor"
        />

        <!-- Copy Raw Markdown -->
        <WorkbenchIconButton
          v-if="currentFileContent"
          icon="solar:copy-linear"
          :label="t('markdownPreview.copyMarkdown')"
          @click="copyMarkdownText"
        />

        <!-- Close current file -->
        <WorkbenchIconButton
          v-if="currentFileContent"
          icon="solar:close-circle-linear"
          :label="t('markdownPreview.closeFile')"
          @click="closeCurrentFile"
        />
      </div>
    </header>

    <!-- Main View Content -->
    <div class="md-body-container">
      <!-- Loading State -->
      <div v-if="loading" class="md-loading-state">
        <WorkbenchEmptyState icon="solar:refresh-circle-linear" :title="t('markdownPreview.loading')" />
      </div>

      <!-- Rendered Document Viewer -->
      <MarkdownDocumentViewer
        v-else-if="currentFileContent"
        ref="docViewer"
        :content="currentFileContent"
        :file-path="currentFilePath"
        @file-dropped="handleViewerFileOpen"
        @stats-updated="updateStats"
      />

      <!-- Empty / Welcome Dropzone State -->
      <div
        v-else
        class="md-welcome-area"
        :class="{ 'is-dragging': isDraggingWelcome }"
        @dragover.prevent="isDraggingWelcome = true"
        @dragleave.prevent="isDraggingWelcome = false"
        @drop.prevent="handleWelcomeDrop"
      >
        <div class="md-welcome-card">
          <div class="welcome-icon-box">
            <Icon icon="solar:document-text-linear" />
          </div>
          <h2>{{ t('markdownPreview.welcomeTitle') }}</h2>
          <p class="welcome-desc">{{ t('markdownPreview.welcomeDesc') }}</p>

          <div class="welcome-actions">
            <WorkbenchButton size="large" variant="primary" @click="selectAndOpenFile">
              <Icon icon="solar:folder-open-linear" />
              <span>{{ t('markdownPreview.selectFile') }}</span>
            </WorkbenchButton>
          </div>

          <div class="welcome-hint">
            <Icon icon="solar:info-circle-linear" />
            <span>{{ t('markdownPreview.dragHint') }}</span>
          </div>
        </div>

        <!-- Recent Documents Panel -->
        <section v-if="historyStore.recentFiles.length" class="md-recent-section">
          <div class="recent-header">
            <h3>{{ t('markdownPreview.recentFiles') }}</h3>
            <button type="button" class="clear-history-btn" @click="historyStore.clearRecentFiles">
              {{ t('common.clear') }}
            </button>
          </div>
          <div class="recent-grid">
            <button
              v-for="item in historyStore.recentFiles"
              :key="item.path"
              type="button"
              class="recent-card"
              @click="loadFile(item.path)"
            >
              <div class="recent-card-icon">
                <Icon icon="solar:document-text-linear" />
              </div>
              <div class="recent-card-text">
                <strong class="recent-file-name">{{ item.name }}</strong>
                <span class="recent-file-path" :title="item.path">{{ item.path }}</span>
              </div>
              <span
                class="recent-card-remove"
                :title="t('common.delete')"
                @click.stop="historyStore.removeRecentFile(item.path)"
              >
                ×
              </span>
            </button>
          </div>
        </section>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { open } from '@tauri-apps/plugin-dialog'
import { NDropdown, useMessage } from 'naive-ui'
import { Icon } from '@iconify/vue'
import { useI18n } from 'vue-i18n'
import WorkbenchButton from '@/components/workbench/WorkbenchButton.vue'
import WorkbenchIconButton from '@/components/workbench/WorkbenchIconButton.vue'
import WorkbenchEmptyState from '@/components/workbench/WorkbenchEmptyState.vue'
import MarkdownDocumentViewer from './components/MarkdownDocumentViewer.vue'
import {
  getMarkdownMetadata,
  openMarkdownInEditor,
  openMarkdownStandaloneWindow,
  readMarkdownFile,
} from '@/services/markdown/markdown-service'
import { useMarkdownHistoryStore } from '@/stores/markdown-history'

const route = useRoute()
const router = useRouter()
const { t } = useI18n({ useScope: 'global' })
const message = useMessage()
const historyStore = useMarkdownHistoryStore()

const docViewer = ref<InstanceType<typeof MarkdownDocumentViewer> | null>(null)
const currentFilePath = ref('')
const currentFileName = ref('')
const currentFileContent = ref('')
const lastModifiedAt = ref<number | null>(null)
const loading = ref(false)
const isDraggingWelcome = ref(false)

const stats = ref({
  wordCount: 0,
  readingTimeMinutes: 1,
})

let pollingTimer: ReturnType<typeof setInterval> | null = null

const recentDropdownOptions = computed(() => {
  return historyStore.recentFiles.map(file => ({
    label: file.name,
    key: file.path,
    description: file.path,
  }))
})

watch(
  () => route.query.file,
  queryFile => {
    if (typeof queryFile === 'string' && queryFile.trim()) {
      void loadFile(queryFile)
    }
  },
  { immediate: true }
)

async function loadFile(filePath: string) {
  if (!filePath) return
  loading.value = true
  try {
    const fileInfo = await readMarkdownFile(filePath)
    currentFilePath.value = fileInfo.path
    currentFileName.value = fileInfo.name
    currentFileContent.value = fileInfo.content
    lastModifiedAt.value = fileInfo.modifiedAt

    historyStore.addRecentFile(fileInfo.path, fileInfo.name)
    startPolling(fileInfo.path)
  } catch (err) {
    message.error(err instanceof Error ? err.message : String(err))
  } finally {
    loading.value = false
  }
}

async function handleViewerFileOpen(filePath: string) {
  await loadFile(filePath)
  void router.replace({ query: { ...route.query, file: filePath } })
}

async function selectAndOpenFile() {
  try {
    const selected = await open({
      multiple: false,
      directory: false,
      filters: [
        {
          name: 'Markdown',
          extensions: ['md', 'markdown', 'mdown', 'mkd', 'txt'],
        },
      ],
    })

    if (selected && typeof selected === 'string') {
      await loadFile(selected)
      void router.replace({ query: { ...route.query, file: selected } })
    }
  } catch (err) {
    message.error(err instanceof Error ? err.message : String(err))
  }
}

function handleRecentSelect(key: string | number) {
  const path = String(key)
  void loadFile(path)
  void router.replace({ query: { ...route.query, file: path } })
}

async function reloadCurrentFile() {
  if (!currentFilePath.value) return
  await loadFile(currentFilePath.value)
  message.success(t('markdownPreview.reloaded'))
}

function closeCurrentFile() {
  stopPolling()
  currentFilePath.value = ''
  currentFileName.value = ''
  currentFileContent.value = ''
  lastModifiedAt.value = null
  void router.replace({ query: {} })
}

async function openInStandaloneWindow() {
  if (!currentFilePath.value) return
  await openMarkdownStandaloneWindow(currentFilePath.value)
}

async function openInExternalEditor() {
  if (!currentFilePath.value) return
  try {
    await openMarkdownInEditor(currentFilePath.value)
  } catch (err) {
    message.error(err instanceof Error ? err.message : String(err))
  }
}

async function copyMarkdownText() {
  if (!currentFileContent.value) return
  try {
    await navigator.clipboard.writeText(currentFileContent.value)
    message.success(t('markdownPreview.copySuccess'))
  } catch {
    message.error(t('markdownPreview.copyFailed'))
  }
}

function triggerSearch() {
  docViewer.value?.openSearch()
}

function updateStats(newStats: { wordCount: number; readingTimeMinutes: number }) {
  stats.value = newStats
}

function handleWelcomeDrop(event: DragEvent) {
  isDraggingWelcome.value = false
  const files = event.dataTransfer?.files
  if (!files || !files.length) return
  const file = files[0]
  const filePath = (file as unknown as { path?: string }).path
  if (filePath && /\.(md|markdown|mdown|mkd|txt)$/i.test(filePath)) {
    void loadFile(filePath)
    void router.replace({ query: { ...route.query, file: filePath } })
  }
}

function startPolling(path: string) {
  stopPolling()
  pollingTimer = setInterval(async () => {
    if (!currentFilePath.value || currentFilePath.value !== path) return
    try {
      const meta = await getMarkdownMetadata(path)
      if (
        meta.modifiedAt &&
        lastModifiedAt.value &&
        meta.modifiedAt > lastModifiedAt.value
      ) {
        lastModifiedAt.value = meta.modifiedAt
        const updated = await readMarkdownFile(path)
        currentFileContent.value = updated.content
      }
    } catch {
      // Ignore polling errors
    }
  }, 3000)
}

function stopPolling() {
  if (pollingTimer) {
    clearInterval(pollingTimer)
    pollingTimer = null
  }
}

function handleGlobalKeydown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'o') {
    e.preventDefault()
    void selectAndOpenFile()
  }
}

onMounted(() => {
  window.addEventListener('keydown', handleGlobalKeydown)
})

onUnmounted(() => {
  stopPolling()
  window.removeEventListener('keydown', handleGlobalKeydown)
})
</script>

<style scoped lang="scss">
.md-preview-page {
  background: var(--lumina-content-bg);
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  min-width: 0;
  overflow: hidden;
  position: relative;
  width: 100%;
}

/* Topbar Header */
.md-topbar {
  align-items: center;
  background: var(--lumina-toolbar-bg);
  border-bottom: 0.5px solid var(--lumina-separator);
  display: flex;
  flex: 0 0 var(--lumina-titlebar-height, 46px);
  gap: 12px;
  height: var(--lumina-titlebar-height, 46px);
  justify-content: space-between;
  padding: 0 14px;
  user-select: none;
  backdrop-filter: var(--lumina-vibrancy);
}

.md-topbar-leading {
  align-items: center;
  display: flex;
  min-width: 0;
}

.md-file-badge {
  align-items: center;
  display: flex;
  gap: 8px;
  min-width: 0;
}

.md-file-badge-icon {
  color: var(--lumina-primary);
  flex: 0 0 auto;
  font-size: 18px;
}

.md-file-info {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.md-file-name {
  color: var(--lumina-text);
  font-size: 13px;
  font-weight: 600;
  line-height: 1.3;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.md-file-path,
.md-file-hint {
  color: var(--lumina-text-secondary);
  font-size: 10.5px;
  line-height: 1.2;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.md-topbar-center {
  align-items: center;
  display: flex;
  gap: 8px;
}

.md-stat-tag {
  align-items: center;
  background: var(--lumina-surface-2);
  border-radius: var(--lumina-radius-sm);
  color: var(--lumina-text-secondary);
  display: inline-flex;
  font-size: 11px;
  gap: 4px;
  padding: 2px 7px;

  svg {
    font-size: 12px;
  }
}

.md-topbar-actions {
  align-items: center;
  display: flex;
  gap: 6px;
  min-width: 0;
}

.action-divider {
  background: var(--lumina-separator);
  height: 18px;
  margin: 0 2px;
  width: 1px;
}

/* Main Body Area */
.md-body-container {
  display: flex;
  flex: 1;
  height: 100%;
  min-height: 0;
  min-width: 0;
  overflow: hidden;
  position: relative;
  width: 100%;
}

.md-loading-state {
  align-items: center;
  display: flex;
  flex: 1;
  justify-content: center;
}

/* Welcome Dropzone Area */
.md-welcome-area {
  align-items: center;
  box-sizing: border-box;
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 32px;
  justify-content: center;
  overflow-y: auto;
  padding: 40px 24px;
  position: relative;

  &.is-dragging {
    background: color-mix(in srgb, var(--lumina-surface-2) 80%, var(--lumina-primary) 20%);
    border: 2px dashed var(--lumina-primary);
  }
}

.md-welcome-card {
  align-items: center;
  background: var(--lumina-surface-elevated);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-xl);
  box-shadow: var(--lumina-shadow-md);
  display: flex;
  flex-direction: column;
  max-width: 480px;
  padding: 36px 32px 28px;
  text-align: center;
  width: 100%;
}

.welcome-icon-box {
  align-items: center;
  background: var(--lumina-primary-soft);
  border-radius: 50%;
  color: var(--lumina-primary);
  display: flex;
  font-size: 32px;
  height: 64px;
  justify-content: center;
  margin-bottom: 16px;
  width: 64px;
}

.md-welcome-card h2 {
  color: var(--lumina-text);
  font-size: 19px;
  font-weight: 650;
  margin: 0 0 8px;
}

.welcome-desc {
  color: var(--lumina-text-secondary);
  font-size: 13px;
  line-height: 1.6;
  margin: 0 0 24px;
}

.welcome-actions {
  display: flex;
  gap: 12px;
  margin-bottom: 20px;
}

.welcome-hint {
  align-items: center;
  color: var(--lumina-text-tertiary);
  display: flex;
  font-size: 11.5px;
  gap: 5px;
}

/* Recent Documents Section */
.md-recent-section {
  max-width: 640px;
  width: 100%;
}

.recent-header {
  align-items: center;
  display: flex;
  justify-content: space-between;
  margin-bottom: 10px;
  padding: 0 4px;

  h3 {
    color: var(--lumina-text-secondary);
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.04em;
    margin: 0;
    text-transform: uppercase;
  }
}

.clear-history-btn {
  background: transparent;
  border: 0;
  color: var(--lumina-text-tertiary);
  cursor: pointer;
  font-size: 11px;
  padding: 2px 6px;

  &:hover {
    color: var(--lumina-danger);
  }
}

.recent-grid {
  display: grid;
  gap: 8px;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
}

.recent-card {
  align-items: center;
  background: var(--lumina-surface-elevated);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-md);
  cursor: pointer;
  display: flex;
  gap: 10px;
  padding: 8px 12px;
  position: relative;
  text-align: left;
  transition: all 0.15s ease;

  &:hover {
    background: var(--lumina-control-hover);
    border-color: var(--lumina-separator-strong);
    transform: translateY(-1px);

    .recent-card-remove {
      opacity: 1;
    }
  }
}

.recent-card-icon {
  color: var(--lumina-primary);
  flex: 0 0 auto;
  font-size: 16px;
}

.recent-card-text {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-width: 0;
}

.recent-file-name {
  color: var(--lumina-text);
  font-size: 12.5px;
  font-weight: 550;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.recent-file-path {
  color: var(--lumina-text-tertiary);
  font-size: 10.5px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.recent-card-remove {
  align-items: center;
  border-radius: 4px;
  color: var(--lumina-text-tertiary);
  display: flex;
  font-size: 14px;
  height: 18px;
  justify-content: center;
  opacity: 0;
  padding: 0 4px;
  transition: opacity 0.15s ease;
  width: 18px;

  &:hover {
    background: var(--lumina-surface-3);
    color: var(--lumina-danger);
  }
}
</style>
