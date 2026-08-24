<template>
  <NModal
    :show="updaterStore.modalVisible"
    :mask-closable="!updaterStore.isDownloading"
    :close-on-esc="!updaterStore.isDownloading"
    preset="card"
    class="update-modal"
    style="width: 540px; max-width: calc(100vw - 32px)"
    @update:show="handleCloseModal"
  >
    <template #header>
      <div class="update-header">
        <div class="update-icon-wrapper">
          <Icon icon="solar:cloud-download-bold-duotone" class="update-icon" />
        </div>
        <div class="update-header-text">
          <h3>{{ updaterStore.isReadyToRelaunch ? t('updater.readyTitle') : t('updater.newVersionFound') }}</h3>
          <p class="update-version-diff">
            <span class="version-current">v{{ updaterStore.currentVersion }}</span>
            <Icon icon="solar:arrow-right-linear" class="arrow-icon" />
            <span class="version-new">v{{ updaterStore.newVersion }}</span>
            <span v-if="updaterStore.releaseDate" class="release-date">{{ formatDate(updaterStore.releaseDate) }}</span>
          </p>
        </div>
      </div>
    </template>

    <div class="update-body">
      <!-- Release Notes section -->
      <div v-if="renderedReleaseNotes" class="release-notes-container">
        <div class="release-notes-label">{{ t('updater.releaseNotes') }}</div>
        <!-- eslint-disable-next-line vue/no-v-html -->
        <div class="release-notes-content" v-html="renderedReleaseNotes" />
      </div>

      <!-- Downloading Progress -->
      <div v-if="updaterStore.isDownloading || updaterStore.isReadyToRelaunch" class="download-progress-section">
        <div class="progress-info">
          <span>{{ updaterStore.isReadyToRelaunch ? t('updater.downloadComplete') : t('updater.downloading') }}</span>
          <span v-if="updaterStore.totalBytes > 0" class="bytes-counter">
            {{ formatBytes(updaterStore.downloadedBytes) }} / {{ formatBytes(updaterStore.totalBytes) }}
          </span>
        </div>
        <NProgress
          type="line"
          :percentage="updaterStore.downloadProgress"
          :status="updaterStore.isReadyToRelaunch ? 'success' : 'default'"
          :show-indicator="true"
          :height="8"
          border-radius="4px"
        />
      </div>

      <!-- Error message if any -->
      <div v-if="updaterStore.errorMessage" class="update-error-banner">
        <Icon icon="solar:danger-triangle-linear" />
        <span>{{ updaterStore.errorMessage }}</span>
      </div>
    </div>

    <template #footer>
      <div class="modal-actions">
        <WorkbenchButton
          v-if="!updaterStore.isDownloading && !updaterStore.isReadyToRelaunch"
          variant="secondary"
          @click="updaterStore.closeModal()"
        >
          {{ t('updater.remindLater') }}
        </WorkbenchButton>

        <WorkbenchButton
          v-if="!updaterStore.isDownloading && !updaterStore.isReadyToRelaunch"
          variant="primary"
          @click="handleStartDownload"
        >
          <Icon icon="solar:download-square-linear" />
          {{ t('updater.updateNow') }}
        </WorkbenchButton>

        <WorkbenchButton
          v-if="updaterStore.isDownloading"
          variant="primary"
          disabled
        >
          <Icon icon="solar:spinner-linear" class="spinning" />
          {{ t('updater.downloading') }} ({{ updaterStore.downloadProgress }}%)
        </WorkbenchButton>

        <WorkbenchButton
          v-if="updaterStore.isReadyToRelaunch"
          variant="primary"
          @click="handleRelaunch"
        >
          <Icon icon="solar:restart-linear" />
          {{ t('updater.restartNow') }}
        </WorkbenchButton>
      </div>
    </template>
  </NModal>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { NModal, NProgress, useMessage } from 'naive-ui'
import { Icon } from '@iconify/vue'
import { useI18n } from 'vue-i18n'
import { marked } from 'marked'
import { useUpdaterStore } from '@/stores/updater'
import WorkbenchButton from '@/components/workbench/WorkbenchButton.vue'

const { t } = useI18n({ useScope: 'global' })
const message = useMessage()
const updaterStore = useUpdaterStore()

const renderedReleaseNotes = computed(() => {
  if (!updaterStore.releaseNotes) return ''
  try {
    return marked.parse(updaterStore.releaseNotes)
  } catch {
    return updaterStore.releaseNotes
  }
})

function formatDate(dateStr: string): string {
  try {
    const d = new Date(dateStr)
    if (isNaN(d.getTime())) return dateStr
    return d.toLocaleDateString()
  } catch {
    return dateStr
  }
}

function formatBytes(bytes: number): string {
  if (bytes <= 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`
}

function handleCloseModal(show: boolean) {
  if (!show && !updaterStore.isDownloading) {
    updaterStore.closeModal()
  }
}

async function handleStartDownload() {
  try {
    await updaterStore.startDownloadAndInstall()
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : String(err)
    message.error(t('updater.downloadFailed', { error: msg }))
  }
}

async function handleRelaunch() {
  try {
    await updaterStore.relaunchApplication()
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : String(err)
    message.error(t('updater.relaunchFailed', { error: msg }))
  }
}
</script>

<style scoped lang="scss">
.update-modal {
  :deep(.n-card__content) {
    padding-top: 10px;
  }
}

.update-header {
  align-items: center;
  display: flex;
  gap: 14px;

  .update-icon-wrapper {
    align-items: center;
    background: color-mix(in srgb, var(--lumina-accent, #39786f) 15%, transparent);
    border-radius: var(--lumina-radius-md, 8px);
    color: var(--lumina-accent, #39786f);
    display: flex;
    font-size: 26px;
    height: 48px;
    justify-content: center;
    width: 48px;
  }

  .update-header-text {
    display: flex;
    flex-direction: column;
    gap: 4px;

    h3 {
      font-size: 16px;
      font-weight: 600;
      margin: 0;
    }

    .update-version-diff {
      align-items: center;
      color: var(--lumina-text-secondary);
      display: flex;
      font-size: 12px;
      gap: 6px;
      margin: 0;

      .version-current {
        font-family: var(--lumina-font-mono, monospace);
        opacity: 0.8;
      }

      .arrow-icon {
        font-size: 11px;
        opacity: 0.5;
      }

      .version-new {
        color: var(--lumina-accent, #39786f);
        font-family: var(--lumina-font-mono, monospace);
        font-weight: 600;
      }

      .release-date {
        margin-left: 6px;
        opacity: 0.6;
      }
    }
  }
}

.update-body {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.release-notes-container {
  display: flex;
  flex-direction: column;
  gap: 6px;

  .release-notes-label {
    color: var(--lumina-text-secondary);
    font-size: 12px;
    font-weight: 500;
  }

  .release-notes-content {
    background: var(--lumina-surface-secondary);
    border: 1px solid var(--lumina-separator);
    border-radius: var(--lumina-radius-sm, 6px);
    font-size: 13px;
    line-height: 1.6;
    max-height: 220px;
    overflow-y: auto;
    padding: 12px 14px;

    :deep(ul), :deep(ol) {
      margin: 4px 0 4px 18px;
      padding: 0;
    }

    :deep(li) {
      margin-bottom: 2px;
    }

    :deep(p) {
      margin: 0 0 6px;

      &:last-child {
        margin-bottom: 0;
      }
    }

    :deep(code) {
      background: var(--lumina-surface-elevated, rgb(0 0 0 / 6%));
      border-radius: 3px;
      font-family: var(--lumina-font-mono, monospace);
      font-size: 0.9em;
      padding: 1px 4px;
    }
  }
}

.download-progress-section {
  background: var(--lumina-surface-secondary);
  border: 1px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-sm, 6px);
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px 14px;

  .progress-info {
    display: flex;
    font-size: 12px;
    font-weight: 500;
    justify-content: space-between;

    .bytes-counter {
      color: var(--lumina-text-secondary);
      font-family: var(--lumina-font-mono, monospace);
      font-weight: normal;
    }
  }
}

.update-error-banner {
  align-items: center;
  background: color-mix(in srgb, #e05252 10%, transparent);
  border: 1px solid color-mix(in srgb, #e05252 25%, transparent);
  border-radius: var(--lumina-radius-sm, 6px);
  color: #e05252;
  display: flex;
  font-size: 12px;
  gap: 8px;
  padding: 10px 12px;
}

.modal-actions {
  display: flex;
  gap: 10px;
  justify-content: flex-end;
  width: 100%;
}

.spinning {
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
</style>
