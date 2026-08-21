<template>
  <section class="storage-panel">
    <header class="storage-panel__intro">
      <span class="storage-panel__app-icon" aria-hidden="true">
        <Icon icon="solar:database-linear" />
      </span>
      <div class="storage-panel__intro-copy">
        <h2>{{ t('settings.storage.title') }}</h2>
        <p>{{ t('settings.storage.description') }}</p>
      </div>
    </header>

    <section class="storage-overview">
      <div class="storage-overview__topline">
        <div class="storage-overview__total">
          <span>{{ t('settings.storage.currentUsage') }}</span>
          <strong>{{ formatBytes(storageStore.overview.totalBytes) }}</strong>
        </div>
        <NButton
          quaternary
          circle
          size="small"
          :aria-label="t('settings.storage.refresh')"
          :title="t('settings.storage.refresh')"
          :loading="storageStore.loading"
          @click="refreshUsage"
        >
          <template #icon><Icon icon="solar:refresh-linear" /></template>
        </NButton>
      </div>

      <div class="storage-meter" role="img" :aria-label="t('settings.storage.currentUsage')">
        <span
          v-for="item in storageSegments"
          :key="item.key"
          class="storage-meter__segment"
          :class="`storage-tone--${item.key}`"
          :style="{ width: `${item.percentage}%` }"
          :title="`${item.label}: ${formatBytes(item.bytes)}`"
        ></span>
      </div>

      <div class="storage-legend">
        <article v-for="item in usageItems" :key="item.key" class="storage-legend__item">
          <span class="storage-legend__icon" :class="`storage-tone--${item.key}`" aria-hidden="true">
            <Icon :icon="item.icon" />
          </span>
          <span class="storage-legend__copy">
            <small>{{ item.label }}</small>
            <strong>{{ formatBytes(item.bytes) }}</strong>
          </span>
          <span class="storage-legend__share">{{ formatPercentage(item.percentage) }}</span>
        </article>
      </div>
    </section>

    <section class="preference-group">
      <div class="preference-row">
        <span class="preference-row__icon preference-row__icon--automatic" aria-hidden="true">
          <Icon icon="solar:broom-linear" />
        </span>
        <div class="preference-row__copy">
          <strong>{{ t('settings.storage.autoCleanup') }}</strong>
          <small>{{ t('settings.storage.autoCleanupHint') }}</small>
        </div>
        <NSwitch
          :value="storageStore.settings.autoCleanupEnabled"
          :loading="storageStore.saving"
          @update:value="updateAutoCleanup"
        />
      </div>

      <div class="preference-row">
        <span class="preference-row__icon preference-row__icon--retention" aria-hidden="true">
          <Icon icon="solar:calendar-date-linear" />
        </span>
        <div class="preference-row__copy">
          <strong>{{ t('settings.storage.retention') }}</strong>
          <small>{{ retentionHint }}</small>
        </div>
        <div class="retention-control">
          <NInputNumber
            v-model:value="retentionDraft"
            :disabled="!storageStore.settings.autoCleanupEnabled"
            :min="1"
            :max="3650"
            :precision="0"
            size="small"
            @blur="saveRetentionDays"
            @keyup.enter="saveRetentionDays"
          />
          <span>{{ t('settings.storage.days') }}</span>
        </div>
      </div>
    </section>

    <footer class="cleanup-card">
      <span class="cleanup-card__icon" aria-hidden="true">
        <Icon icon="solar:trash-bin-minimalistic-linear" />
      </span>
      <div class="cleanup-card__copy">
        <strong>{{ t('settings.storage.manualCleanup') }}</strong>
        <small>{{ t('settings.storage.manualCleanupHint') }}</small>
        <small v-if="lastCleanupLabel" class="last-cleanup">
          {{ t('settings.storage.lastCleanup', { time: lastCleanupLabel }) }}
        </small>
      </div>
      <NButton type="error" secondary :loading="storageStore.cleaning" @click="cleanNow">
        <template #icon><Icon icon="solar:trash-bin-minimalistic-linear" /></template>
        {{ t('settings.storage.cleanNow') }}
      </NButton>
    </footer>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { NButton, NInputNumber, NSwitch, useMessage } from 'naive-ui'
import { useI18n } from 'vue-i18n'
import { useStorageStore } from '@/stores/storage'

const { locale, t } = useI18n({ useScope: 'global' })
const message = useMessage()
const storageStore = useStorageStore()
const retentionDraft = ref<number | null>(storageStore.settings.retentionDays)

const usageItems = computed(() => {
  const total = storageStore.overview.totalBytes
  return [
    { key: 'data', icon: 'solar:database-linear', label: t('settings.storage.categories.data'), bytes: storageStore.overview.dataBytes },
    { key: 'cache', icon: 'solar:box-minimalistic-linear', label: t('settings.storage.categories.cache'), bytes: storageStore.overview.cacheBytes },
    { key: 'logs', icon: 'solar:document-text-linear', label: t('settings.storage.categories.logs'), bytes: storageStore.overview.logBytes },
    { key: 'settings', icon: 'solar:settings-minimalistic-linear', label: t('settings.storage.categories.settings'), bytes: storageStore.overview.configurationBytes },
    { key: 'webview', icon: 'solar:window-frame-linear', label: t('settings.storage.categories.webview'), bytes: storageStore.overview.localStorageBytes },
  ].map(item => ({
    ...item,
    percentage: total > 0 ? (item.bytes / total) * 100 : 0,
  }))
})

const storageSegments = computed(() => usageItems.value.filter(item => item.bytes > 0))

const retentionHint = computed(() => storageStore.settings.autoCleanupEnabled
  ? t('settings.storage.retentionHint', { days: retentionDraft.value ?? 90 })
  : t('settings.storage.retentionDisabledHint'))

const lastCleanupLabel = computed(() => {
  if (!storageStore.settings.lastCleanupAt) return ''
  return new Intl.DateTimeFormat(locale.value, {
    dateStyle: 'medium',
    timeStyle: 'short',
  }).format(storageStore.settings.lastCleanupAt)
})

watch(() => storageStore.settings.retentionDays, value => {
  retentionDraft.value = value
})

onMounted(async () => {
  await storageStore.initStorage()
  retentionDraft.value = storageStore.settings.retentionDays
})

async function updateAutoCleanup(enabled: boolean) {
  try {
    await storageStore.updateSettings({ autoCleanupEnabled: enabled })
  } catch (error) {
    message.error(t('settings.storage.saveFailed', { error: errorMessage(error) }))
  }
}

async function saveRetentionDays() {
  const days = Math.round(retentionDraft.value ?? storageStore.settings.retentionDays)
  const normalized = Math.min(3650, Math.max(1, days))
  retentionDraft.value = normalized
  if (normalized === storageStore.settings.retentionDays) return
  try {
    await storageStore.updateSettings({ retentionDays: normalized })
    message.success(t('settings.storage.saved'))
  } catch (error) {
    retentionDraft.value = storageStore.settings.retentionDays
    message.error(t('settings.storage.saveFailed', { error: errorMessage(error) }))
  }
}

async function refreshUsage() {
  storageStore.loading = true
  try {
    await storageStore.refreshOverview()
  } catch (error) {
    message.error(t('settings.storage.refreshFailed', { error: errorMessage(error) }))
  } finally {
    storageStore.loading = false
  }
}

async function cleanNow() {
  try {
    const result = await storageStore.runCleanup(true)
    message.success(t('settings.storage.cleanupComplete', {
      size: formatBytes(result.totalReclaimedBytes),
    }))
  } catch (error) {
    message.error(t('settings.storage.cleanupFailed', { error: errorMessage(error) }))
  }
}

function formatBytes(bytes: number) {
  if (!Number.isFinite(bytes) || bytes <= 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB']
  const index = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1)
  const value = bytes / 1024 ** index
  return `${new Intl.NumberFormat(locale.value, { maximumFractionDigits: index ? 1 : 0 }).format(value)} ${units[index]}`
}

function formatPercentage(value: number) {
  if (value <= 0) return '—'
  if (value < 1) return '<1%'
  return `${Math.round(value)}%`
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error)
}
</script>

<style scoped lang="scss">
.storage-panel {
  display: grid;
  gap: var(--lumina-gap-lg);
  max-width: 820px;
  padding-bottom: var(--lumina-gap-xl);
}

.storage-panel__intro {
  align-items: center;
  display: flex;
  gap: 12px;
  padding: 2px 2px 0;
}

.storage-panel__app-icon {
  align-items: center;
  background: linear-gradient(145deg, var(--lumina-surface-elevated), var(--lumina-surface-tertiary));
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-lg);
  box-shadow: var(--lumina-shadow-sm);
  color: var(--lumina-primary);
  display: inline-flex;
  flex: 0 0 auto;
  height: 44px;
  justify-content: center;
  width: 44px;

  svg {
    height: 22px;
    width: 22px;
  }
}

.storage-panel__intro-copy {
  min-width: 0;

  h2 {
    font-size: 18px;
    letter-spacing: -0.01em;
    margin: 0 0 3px;
  }

  p {
    color: var(--lumina-text-secondary);
    font-size: 12px;
    line-height: 1.45;
    margin: 0;
  }
}

.storage-overview,
.preference-group,
.cleanup-card {
  background: var(--lumina-surface-elevated);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-lg);
  box-shadow: var(--lumina-shadow-sm);
}

.storage-overview {
  padding: 18px;
}

.storage-overview__topline {
  align-items: center;
  display: flex;
  justify-content: space-between;
}

.storage-overview__total {
  align-items: baseline;
  display: flex;
  gap: 10px;

  span {
    color: var(--lumina-text-secondary);
    font-size: 12px;
  }

  strong {
    font-size: 24px;
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.025em;
    line-height: 1;
  }
}

.storage-meter {
  background: var(--lumina-control-bg);
  border-radius: var(--lumina-radius-pill);
  box-shadow: inset 0 0 0 0.5px var(--lumina-separator);
  display: flex;
  height: 9px;
  margin-top: 18px;
  overflow: hidden;
}

.storage-meter__segment {
  min-width: 2px;
  transition: width var(--lumina-duration-normal) var(--lumina-ease-out);

  & + & {
    box-shadow: -1px 0 0 var(--lumina-surface-elevated);
  }
}

.storage-legend {
  display: grid;
  gap: 8px 12px;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  margin-top: 16px;
}

.storage-legend__item {
  align-items: center;
  border-radius: var(--lumina-radius-sm);
  display: flex;
  gap: 9px;
  min-height: 36px;
  padding: 4px 6px;
  transition: background var(--lumina-duration-fast) var(--lumina-ease-out);

  &:hover {
    background: var(--lumina-control-bg);
  }
}

.storage-legend__icon,
.preference-row__icon,
.cleanup-card__icon {
  align-items: center;
  border-radius: var(--lumina-radius-sm);
  color: var(--lumina-on-accent);
  display: inline-flex;
  flex: 0 0 auto;
  height: 28px;
  justify-content: center;
  width: 28px;

  svg {
    height: 15px;
    width: 15px;
  }
}

.storage-legend__copy {
  display: grid;
  gap: 1px;
  min-width: 0;

  small {
    color: var(--lumina-text-secondary);
    font-size: 10px;
  }

  strong {
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    font-weight: 600;
  }
}

.storage-legend__share {
  color: var(--lumina-text-tertiary);
  font-size: 10px;
  font-variant-numeric: tabular-nums;
  margin-left: auto;
}

.storage-tone--data { background: var(--lumina-primary); }
.storage-tone--cache { background: var(--lumina-warning); }
.storage-tone--logs { background: var(--lumina-danger); }
.storage-tone--settings { background: var(--lumina-success); }
.storage-tone--webview { background: var(--lumina-text-tertiary); }

.preference-group {
  overflow: hidden;
}

.preference-row {
  align-items: center;
  display: flex;
  gap: 12px;
  min-height: 64px;
  padding: 8px 14px;

  & + & {
    border-top: 1px solid var(--lumina-separator);
  }
}

.preference-row__icon--automatic {
  background: var(--lumina-primary);
}

.preference-row__icon--retention {
  background: var(--lumina-warning);
}

.preference-row__copy,
.cleanup-card__copy {
  display: grid;
  flex: 1;
  gap: 2px;
  min-width: 0;

  strong { font-size: 13px; }

  small {
    color: var(--lumina-text-secondary);
    font-size: 11px;
    line-height: 1.45;
  }
}

.retention-control {
  align-items: center;
  display: flex;
  flex: 0 0 auto;
  gap: 8px;

  :deep(.n-input-number) {
    width: 96px;
  }

  span {
    color: var(--lumina-text-secondary);
    font-size: 12px;
  }
}

.cleanup-card {
  align-items: center;
  display: flex;
  gap: 12px;
  padding: 14px;
}

.cleanup-card__icon {
  background: color-mix(in srgb, var(--lumina-danger) 14%, var(--lumina-surface-elevated));
  color: var(--lumina-danger);
}

.last-cleanup {
  color: var(--lumina-text-tertiary) !important;
  margin-top: 2px;
}

@media (max-width: 680px) {
  .storage-legend { grid-template-columns: 1fr; }
  .storage-overview__total { align-items: flex-start; flex-direction: column; gap: 4px; }
  .preference-row { align-items: flex-start; flex-wrap: wrap; padding-block: 12px; }
  .preference-row__copy { min-width: calc(100% - 44px); }
  .retention-control { margin-left: 40px; }
  .cleanup-card { align-items: flex-start; flex-wrap: wrap; }
  .cleanup-card__copy { min-width: calc(100% - 44px); }
  .cleanup-card :deep(.n-button) { margin-left: 40px; }
}
</style>
