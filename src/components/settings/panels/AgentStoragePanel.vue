<template>
  <section class="agent-storage">
    <div class="agent-storage__header">
      <div>
        <h3>{{ t('settings.storage.agentTitle') }}</h3>
        <p>{{ t('settings.storage.agentHint') }}</p>
      </div>
      <NButton secondary :loading="scanning" @click="refresh">
        <template #icon><Icon icon="solar:refresh-linear" /></template>
        {{ overview ? t('settings.storage.agentRescan') : t('settings.storage.agentScan') }}
      </NButton>
    </div>

    <div v-if="!overview" class="agent-storage__empty-state">
      <Icon icon="solar:diskette-linear" />
      <strong>{{ t('settings.storage.agentBeforeScan') }}</strong>
      <span>{{ t('settings.storage.agentTotalHint') }}</span>
      <NButton type="primary" :loading="scanning" @click="refresh">{{ t('settings.storage.agentScan') }}</NButton>
    </div>
    <template v-else>
      <div class="agent-storage__summary">
        <div><span>{{ t('settings.storage.agentTotal') }}</span><strong>{{ formatBytes(overview.totalBytes) }}</strong></div>
        <div v-if="overview.systemDrive"><span>{{ overview.systemDrive }} {{ t('settings.storage.agentDriveLabel') }}</span><strong>{{ formatBytes(overview.systemDriveBytes) }}</strong></div>
        <small>{{ t('settings.storage.agentTotalHint') }}</small>
      </div>
      <div class="agent-storage__tabs" role="tablist">
        <button type="button" role="tab" :aria-selected="activeSection === 'agents'" :class="{ active: activeSection === 'agents' }" @click="activeSection = 'agents'">
          {{ t('settings.storage.agentByTool') }} <span>{{ overview.agents.length }}</span>
        </button>
        <button type="button" role="tab" :aria-selected="activeSection === 'caches'" :class="{ active: activeSection === 'caches' }" @click="activeSection = 'caches'">
          {{ t('settings.storage.agentCacheTitle') }} <span>{{ overview.caches.length }}</span>
        </button>
      </div>

      <div v-if="activeSection === 'agents' && overview.agents.length" class="agent-storage__workspace" role="tabpanel">
        <div class="agent-storage__sidebar" :aria-label="t('settings.storage.agentByTool')">
          <button v-for="agent in overview.agents" :key="agent.id" type="button" :class="{ active: selectedAgent?.id === agent.id }" @click="selectedAgentId = agent.id">
            <span class="agent-storage__sidebar-title"><strong>{{ agent.label }}</strong><small>{{ formatBytes(agent.totalBytes) }}</small></span>
            <span class="agent-storage__meter" aria-hidden="true"><span :style="{ width: `${overview.totalBytes ? agent.totalBytes / overview.totalBytes * 100 : 0}%` }"></span></span>
          </button>
        </div>
        <article v-if="selectedAgent" class="agent-storage__detail">
          <header class="agent-storage__detail-heading">
            <div><small>{{ t('settings.storage.agentSelectedTool') }}</small><h4>{{ selectedAgent.label }}</h4></div>
            <strong>{{ formatBytes(selectedAgent.totalBytes) }}</strong>
          </header>
          <div v-for="location in selectedAgent.locations" :key="location.path" class="agent-storage__location-card">
            <div class="agent-storage__location-heading">
              <span>{{ t(`settings.storage.agentKinds.${location.kind}`) }}</span>
              <strong :title="location.incomplete ? t('settings.storage.agentIncomplete') : undefined">{{ location.incomplete ? '≥ ' : '' }}{{ formatBytes(location.bytes) }}</strong>
            </div>
            <div class="agent-storage__location-path">
              <code :title="location.path">{{ location.path }}</code>
              <NButton size="tiny" quaternary :aria-label="t('settings.storage.agentOpenPath', { path: location.path })" @click="openLocation(location.path)">
                <template #icon><Icon icon="solar:folder-with-files-linear" /></template>
                {{ t('settings.storage.agentOpen') }}
              </NButton>
            </div>
            <details v-if="location.largestChildren.length" class="agent-storage__children">
              <summary>{{ t('settings.storage.agentLargestFolders') }}</summary>
              <div v-for="child in location.largestChildren" :key="child.path" class="agent-storage__child">
                <code :title="child.path">{{ child.name }}</code>
                <span>{{ location.incomplete ? '≥ ' : '' }}{{ formatBytes(child.bytes) }}</span>
              </div>
            </details>
          </div>
        </article>
      </div>
      <div v-else-if="activeSection === 'agents'" class="agent-storage__empty">{{ t('settings.storage.agentEmpty') }}</div>

      <div v-else class="agent-storage__caches" role="tabpanel">
        <p>{{ t('settings.storage.agentCacheHint') }}</p>
        <div v-if="!overview.caches.length" class="agent-storage__empty">{{ t('settings.storage.agentNoCaches') }}</div>
        <div v-for="cache in overview.caches" :key="cache.key" class="agent-storage__cache-row">
          <div class="agent-storage__cache-info">
            <span><strong>{{ cache.agentLabel }}</strong><small>{{ formatBytes(cache.bytes) }}</small></span>
            <code :title="cache.path">{{ cache.path }}</code>
          </div>
          <NButton type="error" secondary size="small" :loading="cleaningKey === cache.key" :disabled="scanning || Boolean(cleaningKey) || cache.bytes === 0" @click="confirmClearCache(cache)">
            {{ t('settings.storage.agentClearCache') }}
          </NButton>
        </div>
      </div>
      <p v-if="hasIncompleteLocations" class="agent-storage__note">{{ t('settings.storage.agentIncomplete') }}</p>
    </template>
  </section>
</template>

<script setup lang="ts">
import { computed, h, ref } from 'vue'
import { Icon } from '@iconify/vue'
import { NButton, useDialog, useMessage } from 'naive-ui'
import { useI18n } from 'vue-i18n'
import { openSkillFolder } from '@/services/skills/skills-service'
import { clearAgentCache, scanAgentStorage } from '@/services/storage-service'
import type { AgentCacheItem, AgentStorageOverview } from '@/types/storage'

const { locale, t } = useI18n({ useScope: 'global' })
const message = useMessage()
const dialog = useDialog()
const overview = ref<AgentStorageOverview | null>(null)
const scanning = ref(false)
const cleaningKey = ref<string | null>(null)
const activeSection = ref<'agents' | 'caches'>('agents')
const selectedAgentId = ref<string | null>(null)
const selectedAgent = computed(() => overview.value?.agents.find(agent => agent.id === selectedAgentId.value) ?? overview.value?.agents[0] ?? null)
const hasIncompleteLocations = computed(() => overview.value?.agents.some(agent => agent.locations.some(location => location.incomplete)) ?? false)

async function refresh() {
  if (scanning.value) return
  scanning.value = true
  try {
    overview.value = await scanAgentStorage()
    if (!overview.value.agents.some(agent => agent.id === selectedAgentId.value)) {
      selectedAgentId.value = overview.value.agents[0]?.id ?? null
    }
  } catch (error) {
    message.error(t('settings.storage.agentScanFailed', { error: error instanceof Error ? error.message : String(error) }))
  } finally {
    scanning.value = false
  }
}

async function openLocation(path: string) {
  try {
    await openSkillFolder(path)
  } catch (error) {
    message.error(t('settings.storage.agentOpenFailed', { error: error instanceof Error ? error.message : String(error) }))
  }
}

function confirmClearCache(cache: AgentCacheItem) {
  dialog.warning({
    title: t('settings.storage.agentClearCache'),
    content: () => h('div', { style: { display: 'grid', gap: '8px' } }, [
      h('p', { style: { margin: 0 } }, t('settings.storage.agentClearConfirm', { agent: cache.agentLabel, size: formatBytes(cache.bytes) })),
      h('code', { style: { overflowWrap: 'anywhere' } }, cache.path),
      h('p', { style: { margin: 0 } }, t('settings.storage.agentClearCloseApp')),
    ]),
    positiveText: t('settings.storage.agentClearCache'),
    negativeText: t('common.cancel'),
    positiveButtonProps: { type: 'error' },
    onPositiveClick: async () => {
      cleaningKey.value = cache.key
      try {
        const result = await clearAgentCache(cache.key)
        await refresh()
        message.success(t('settings.storage.agentClearDone', { size: formatBytes(result.reclaimedBytes), skipped: result.skippedEntries }))
      } catch (error) {
        message.error(t('settings.storage.agentClearFailed', { error: error instanceof Error ? error.message : String(error) }))
        return false
      } finally {
        cleaningKey.value = null
      }
    },
  })
}

function formatBytes(bytes: number) {
  if (!Number.isFinite(bytes) || bytes <= 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const index = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1)
  return `${new Intl.NumberFormat(locale.value, { maximumFractionDigits: index ? 1 : 0 }).format(bytes / 1024 ** index)} ${units[index]}`
}
</script>

<style scoped lang="scss">
.agent-storage {
  background: var(--lumina-surface-elevated);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-lg);
  box-shadow: var(--lumina-shadow-sm);
  display: grid;
  gap: 14px;
  min-width: 0;
  padding: 18px;
}
.agent-storage__header {
  align-items: center;
  display: flex;
  gap: 16px;
  justify-content: space-between;

  h3 { font-size: 15px; margin: 0 0 3px; }
  p { color: var(--lumina-text-secondary); font-size: 11px; line-height: 1.4; margin: 0; }
}
.agent-storage__empty-state {
  align-items: center;
  color: var(--lumina-text-secondary);
  display: grid;
  gap: 8px;
  justify-items: center;
  padding: 44px 20px;
  text-align: center;

  svg { color: var(--lumina-primary); font-size: 30px; }
  strong { color: var(--lumina-text); font-size: 13px; }
  span { font-size: 11px; }
}
.agent-storage__empty, .agent-storage__note {
  color: var(--lumina-text-secondary);
  font-size: 11px;
  margin: 0;
  padding: 8px 2px;
}
.agent-storage__summary {
  align-items: center;
  background: var(--lumina-surface-secondary);
  border: 0.5px solid var(--lumina-separator);
  border-radius: 8px;
  display: flex;
  flex-wrap: wrap;
  gap: 8px 24px;
  padding: 12px 14px;

  > div { align-items: baseline; display: flex; gap: 8px; }
  span, small { color: var(--lumina-text-secondary); font-size: 11px; }
  strong { color: var(--lumina-text); font-size: 18px; font-variant-numeric: tabular-nums; }
  small { margin-left: auto; }
}
.agent-storage__tabs {
  background: var(--lumina-control-bg);
  border: 0.5px solid var(--lumina-separator);
  border-radius: 7px;
  display: inline-flex;
  gap: 2px;
  padding: 2px;
  width: fit-content;

  button { background: transparent; border: 0; border-radius: 5px; color: var(--lumina-text-secondary); cursor: pointer; font-size: 11px; height: 27px; padding: 0 12px; }
  button.active { background: var(--lumina-surface-elevated); box-shadow: var(--lumina-shadow-sm); color: var(--lumina-text); font-weight: 600; }
  span { color: var(--lumina-text-tertiary); font-size: 10px; margin-left: 3px; }
}
.agent-storage__workspace {
  border: 0.5px solid var(--lumina-separator);
  border-radius: 9px;
  display: grid;
  grid-template-columns: minmax(180px, 220px) minmax(0, 1fr);
  min-height: 320px;
  overflow: hidden;
}
.agent-storage__sidebar {
  background: var(--lumina-surface-secondary);
  border-right: 0.5px solid var(--lumina-separator);
  display: grid;
  align-content: start;
  gap: 3px;
  max-height: 470px;
  overflow-y: auto;
  padding: 8px;

  button { background: transparent; border: 0; border-radius: 6px; color: var(--lumina-text); cursor: pointer; display: grid; gap: 7px; padding: 9px 10px; text-align: left; }
  button:hover { background: var(--lumina-control-hover); }
  button.active { background: var(--lumina-primary-soft); }
}
.agent-storage__sidebar-title { align-items: center; display: flex; gap: 8px; justify-content: space-between; min-width: 0; }
.agent-storage__sidebar-title strong { font-size: 11px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.agent-storage__sidebar-title small { color: var(--lumina-text-secondary); font-size: 10px; font-variant-numeric: tabular-nums; white-space: nowrap; }
.agent-storage__meter {
  background: var(--lumina-fill-quaternary);
  border-radius: 999px;
  display: block;
  height: 3px;
  overflow: hidden;

  span { background: var(--lumina-primary); border-radius: inherit; display: block; height: 100%; }
}
.agent-storage__detail {
  display: grid;
  align-content: start;
  gap: 12px;
  max-height: 470px;
  overflow-y: auto;
  padding: 18px;
}
.agent-storage__detail-heading {
  align-items: end;
  display: flex;
  justify-content: space-between;
  margin-bottom: 2px;

  small { color: var(--lumina-text-tertiary); font-size: 10px; }
  h4 { font-size: 16px; margin: 2px 0 0; }
  > strong { font-size: 16px; font-variant-numeric: tabular-nums; }
}
.agent-storage__location-card { border: 0.5px solid var(--lumina-separator); border-radius: 7px; display: grid; gap: 8px; padding: 12px; }
.agent-storage__location-heading { align-items: center; display: flex; justify-content: space-between; }
.agent-storage__location-heading span { color: var(--lumina-text-secondary); font-size: 11px; }
.agent-storage__location-heading strong { font-size: 12px; font-variant-numeric: tabular-nums; }
.agent-storage__location-path { align-items: center; display: flex; gap: 8px; min-width: 0; }
.agent-storage__location-path code { color: var(--lumina-text-secondary); flex: 1; font-size: 10px; min-width: 0; overflow-wrap: anywhere; }
.agent-storage__children {
  color: var(--lumina-text-secondary);
  font-size: 10px;
  padding-top: 4px;

  summary { cursor: pointer; width: fit-content; }
}
.agent-storage__child {
  display: flex;
  gap: 12px;
  justify-content: space-between;
  padding: 5px 0 0 12px;

  code { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  span { font-variant-numeric: tabular-nums; white-space: nowrap; }
}
.agent-storage__caches {
  display: grid;
  align-content: start;
  gap: 0;
  max-height: 470px;
  overflow-y: auto;

  > p { color: var(--lumina-text-secondary); font-size: 11px; margin: 0 0 8px; }
}
.agent-storage__cache-row {
  align-items: center;
  border-bottom: 0.5px solid var(--lumina-separator);
  display: flex;
  gap: 14px;
  min-width: 0;
  padding: 10px 2px;
}
.agent-storage__cache-info { display: grid; flex: 1; gap: 3px; min-width: 0; }
.agent-storage__cache-info span { display: flex; gap: 8px; }
.agent-storage__cache-info strong { font-size: 11px; }
.agent-storage__cache-info small { color: var(--lumina-text-secondary); font-size: 10px; font-variant-numeric: tabular-nums; }
.agent-storage__cache-info code { color: var(--lumina-text-tertiary); font-size: 10px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

@media (max-width: 760px) {
  .agent-storage__header { align-items: stretch; flex-direction: column; }
  .agent-storage__summary small { margin-left: 0; width: 100%; }
  .agent-storage__workspace { grid-template-columns: 1fr; }
  .agent-storage__sidebar { border-bottom: 0.5px solid var(--lumina-separator); border-right: 0; display: flex; max-height: 145px; overflow-x: auto; }
  .agent-storage__sidebar button { flex: 0 0 160px; }
  .agent-storage__detail { max-height: none; }
  .agent-storage__cache-row { flex-wrap: wrap; }
}
</style>
