<template>
  <div class="md-window-page">
    <!-- Standalone Window Header -->
    <header class="md-window-header" data-tauri-drag-region>
      <div class="header-leading" data-tauri-drag-region>
        <div class="file-badge" data-tauri-drag-region>
          <Icon icon="solar:document-text-linear" class="file-icon" />
          <div class="file-details" data-tauri-drag-region>
            <strong class="file-title" data-tauri-drag-region>{{ currentFileName || t('markdownPreview.title') }}</strong>
            <span class="file-path" data-tauri-drag-region :title="currentFilePath">{{ currentFilePath }}</span>
          </div>
        </div>
      </div>

      <!-- Center Reading Stats -->
      <div v-if="currentFileContent" class="header-center" data-tauri-drag-region>
        <span class="header-tag">
          <Icon icon="solar:text-square-linear" />
          <span>{{ stats.wordCount }} {{ t('markdownPreview.words') }}</span>
        </span>
        <span class="header-tag">
          <Icon icon="solar:clock-circle-linear" />
          <span>~{{ stats.readingTimeMinutes }} {{ t('markdownPreview.minutes') }}</span>
        </span>
      </div>

      <!-- Header Actions -->
      <div class="header-actions">
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

        <!-- Open in External Editor -->
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
      </div>
    </header>

    <!-- Content Area -->
    <main class="md-window-content">
      <div v-if="loading" class="window-state">
        <WorkbenchEmptyState icon="solar:refresh-circle-linear" :title="t('markdownPreview.loading')" />
      </div>
      <div v-else-if="error" class="window-state window-state--error">
        <WorkbenchEmptyState icon="solar:danger-triangle-linear" :title="error" />
      </div>
      <MarkdownDocumentViewer
        v-else-if="currentFileContent"
        ref="docViewer"
        :content="currentFileContent"
        :file-path="currentFilePath"
        @file-dropped="loadFile"
        @stats-updated="updateStats"
      />
      <div v-else class="window-state">
        <WorkbenchEmptyState icon="solar:document-text-linear" :title="t('markdownPreview.noFileSelected')" />
      </div>
    </main>
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { Icon } from '@iconify/vue'
import { useMessage } from 'naive-ui'
import { useI18n } from 'vue-i18n'
import WorkbenchIconButton from '@/components/workbench/WorkbenchIconButton.vue'
import WorkbenchEmptyState from '@/components/workbench/WorkbenchEmptyState.vue'
import MarkdownDocumentViewer from './components/MarkdownDocumentViewer.vue'
import {
  getMarkdownMetadata,
  openMarkdownInEditor,
  readMarkdownFile,
} from '@/services/markdown/markdown-service'
import { useMarkdownHistoryStore } from '@/stores/markdown-history'

const route = useRoute()
const { t } = useI18n({ useScope: 'global' })
const message = useMessage()
const historyStore = useMarkdownHistoryStore()
const appWindow = getCurrentWindow()

const docViewer = ref<InstanceType<typeof MarkdownDocumentViewer> | null>(null)
const currentFilePath = ref('')
const currentFileName = ref('')
const currentFileContent = ref('')
const lastModifiedAt = ref<number | null>(null)
const loading = ref(false)
const error = ref('')

const stats = ref({
  wordCount: 0,
  readingTimeMinutes: 1,
})

let pollingTimer: ReturnType<typeof setInterval> | null = null
let unlistenOpenEvent: UnlistenFn | null = null

watch(
  () => route.query.file,
  newFile => {
    if (typeof newFile === 'string' && newFile.trim()) {
      void loadFile(newFile)
    }
  },
  { immediate: true }
)

async function loadFile(filePath: string) {
  if (!filePath) return
  loading.value = true
  error.value = ''
  try {
    const fileInfo = await readMarkdownFile(filePath)
    currentFilePath.value = fileInfo.path
    currentFileName.value = fileInfo.name
    currentFileContent.value = fileInfo.content
    lastModifiedAt.value = fileInfo.modifiedAt

    try {
      await appWindow.setTitle(`Halowake - ${fileInfo.name}`)
    } catch (titleErr) {
      console.warn('Failed to set native window title:', titleErr)
    }
    if (typeof document !== 'undefined') {
      document.title = `Halowake - ${fileInfo.name}`
    }
    historyStore.addRecentFile(fileInfo.path, fileInfo.name)
    startPolling(fileInfo.path)
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
  } finally {
    loading.value = false
  }
}

async function reloadCurrentFile() {
  if (!currentFilePath.value) return
  await loadFile(currentFilePath.value)
  message.success(t('markdownPreview.reloaded'))
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

onMounted(async () => {
  unlistenOpenEvent = await listen<{ filePath: string }>('markdown-preview-open', event => {
    if (event.payload?.filePath) {
      void loadFile(event.payload.filePath)
    }
  })
})

onUnmounted(() => {
  stopPolling()
  unlistenOpenEvent?.()
})
</script>

<style scoped lang="scss">
.md-window-page {
  background: var(--lumina-window-bg);
  color: var(--lumina-text);
  display: flex;
  flex-direction: column;
  height: 100vh;
  inset: 0;
  overflow: hidden;
  position: fixed;
  width: 100%;
}

.md-window-header {
  align-items: center;
  background: var(--lumina-toolbar-bg);
  border-bottom: 0.5px solid var(--lumina-separator);
  display: flex;
  flex: 0 0 50px;
  height: 50px;
  justify-content: space-between;
  padding: 0 16px;
  user-select: none;
  backdrop-filter: var(--lumina-vibrancy);
}

.header-leading {
  align-items: center;
  display: flex;
  min-width: 0;
}

.file-badge {
  align-items: center;
  display: flex;
  gap: 10px;
  min-width: 0;
}

.file-icon {
  color: var(--lumina-primary);
  flex: 0 0 auto;
  font-size: 20px;
}

.file-details {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.file-title {
  color: var(--lumina-text);
  font-size: 13.5px;
  font-weight: 650;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.file-path {
  color: var(--lumina-text-secondary);
  font-size: 11px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.header-center {
  align-items: center;
  display: flex;
  gap: 8px;
}

.header-tag {
  align-items: center;
  background: var(--lumina-surface-2);
  border-radius: var(--lumina-radius-sm);
  color: var(--lumina-text-secondary);
  display: inline-flex;
  font-size: 11.5px;
  gap: 5px;
  padding: 3px 8px;

  svg {
    font-size: 13px;
  }
}

.header-actions {
  align-items: center;
  display: flex;
  gap: 6px;
}

.md-window-content {
  background: var(--lumina-content-bg);
  display: flex;
  flex: 1;
  height: calc(100vh - 50px);
  min-height: 0;
  min-width: 0;
  overflow: hidden;
  position: relative;
  width: 100%;
}

.window-state {
  align-items: center;
  display: flex;
  flex: 1;
  justify-content: center;
}

.window-state--error {
  color: var(--lumina-danger);
}
</style>
