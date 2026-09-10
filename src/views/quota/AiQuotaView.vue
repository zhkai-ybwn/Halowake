<template>
  <div class="quota-view-container">
    <!-- 顶部概览指标栏 -->
    <header class="overview-header">
      <div class="header-left">
        <div class="view-title-group">
          <h2>{{ t('quota.viewTitle') }}</h2>
          <span class="view-subtitle">{{ t('quota.viewSubtitle') }}</span>
        </div>
      </div>

      <div class="header-actions">
        <WorkbenchButton :disabled="loading" @click="handleRefreshAll">
          <Icon icon="solar:restart-linear" :class="{ 'spin-anim': loading }" />
          {{ loading ? t('quota.refreshing') : t('quota.refreshAll') }}
        </WorkbenchButton>

        <WorkbenchButton variant="primary" @click="openAddModal">
          <Icon icon="solar:add-circle-linear" />
          {{ t('quota.addAccount') }}
        </WorkbenchButton>

        <NDropdown :options="moreActionOptions" trigger="click" @select="handleMoreAction">
          <WorkbenchButton>
            <Icon icon="solar:menu-dots-bold" />
            {{ t('quota.moreActions') }}
          </WorkbenchButton>
        </NDropdown>
      </div>
    </header>

    <!-- 紧凑总览：状态优先，资产次之 -->
    <section class="summary-strip">
      <div class="summary-status">
        <span class="summary-status-icon"><Icon icon="solar:check-circle-linear" /></span>
        <strong>{{ healthyAccountsCount }}/{{ quotas.length }}</strong>
        <span>{{ t('quota.accountsHealthy') }}</span>
      </div>
      <button
        type="button"
        class="summary-issue"
        :class="{ active: attentionCount > 0 }"
        @click="activeCategory = 'issues'"
      >
        <Icon icon="solar:danger-triangle-linear" />
        <strong>{{ attentionCount }}</strong>
        <span>{{ t('quota.needsAttention') }}</span>
      </button>
      <div class="summary-divider" />
      <div class="summary-assets" :aria-label="t('quota.assetsOverview')">
        <span><small>CNY</small> ¥{{ summary.totalCnyBalance.toFixed(2) }}</span>
        <span><small>USD</small> ${{ summary.totalUsdBalance.toFixed(2) }}</span>
        <span><small>{{ t('quota.creditsUnit') }}</small> {{ (summary.totalCredits || 0).toLocaleString() }}</span>
      </div>
      <div class="summary-refresh">
        <Icon icon="solar:history-linear" />
        {{ autoRefreshLabel }}
      </div>
    </section>

    <!-- 扫描进行中状态提示横幅 -->
    <transition name="fade">
      <div v-if="discovering" class="scan-progress-banner">
        <div class="scan-banner-left">
          <Icon :icon="isScanning ? 'solar:radar-bold' : 'solar:restart-linear'" class="scan-radar-icon spin-anim" />
          <div class="scan-banner-info">
            <span class="scan-banner-title">
              {{ isScanning ? t('quota.scanningLocalTitle') : t('quota.syncingDiscoveredTitle') }}
            </span>
            <span class="scan-banner-sub">
              {{ isScanning ? t('quota.scanningLocalSubtitle') : t('quota.syncingDiscoveredSubtitle') }}
            </span>
          </div>
        </div>
        <div class="scan-banner-right">
          <span class="scan-banner-badge">
            <span class="pulse-dot"></span>
            {{ isScanning ? t('quota.scanningLocalBadge') : t('quota.syncingDiscoveredBadge') }}
          </span>
        </div>
      </div>
    </transition>

    <!-- 分类筛选 Tab 栏 -->
    <div v-if="quotas.length > 0" class="category-filter-bar">
      <button
        v-for="tab in categoryTabs"
        :key="tab.id"
        type="button"
        class="category-tab-btn"
        :class="{ active: activeCategory === tab.id }"
        @click="activeCategory = tab.id"
      >
        <span>{{ tab.label }}</span>
        <span class="tab-count">{{ tab.count }}</span>
      </button>
    </div>

    <!-- 额度卡片列表 -->
    <main class="cards-viewport">
      <div v-if="loading && quotas.length === 0" class="loading-state">
        <Icon icon="solar:restart-linear" class="spin-anim" />
        <p>{{ t('quota.loadingQuotas') }}</p>
      </div>

      <div v-else-if="quotas.length === 0" class="empty-state">
        <div class="empty-illustration">
          <Icon icon="solar:box-minimalistic-linear" />
        </div>
        <h3>{{ t('quota.noAccountsConfigured') }}</h3>
        <p>{{ t('quota.emptyGuide') }}</p>
        <div class="empty-actions">
          <WorkbenchButton :disabled="discovering" @click="handleDiscoverLocal">
            <Icon icon="solar:radar-linear" />
            {{ t('quota.discoverLocal') }}
          </WorkbenchButton>
          <WorkbenchButton variant="primary" @click="openAddModal">
            <Icon icon="solar:add-circle-linear" />
            {{ t('quota.addAccount') }}
          </WorkbenchButton>
        </div>
      </div>

      <div v-else-if="filteredQuotas.length === 0" class="empty-filtered-state">
        <Icon icon="solar:filter-linear" />
        <p>{{ t('quota.noMatchingQuotas') }}</p>
      </div>

      <div v-else class="cards-grid">
        <QuotaCard
          v-for="item in filteredQuotas"
          :key="item.id"
          :quota="item"
          :refreshing="refreshingId === item.accountId"
          :launching="launchingId === item.accountId"
          @edit="openEditById"
          @refresh="handleRefreshSingle"
          @launch="handleLaunchClient"
        />
      </div>

      <!-- 底部安全留白区：确保滚动到最底部时与窗口底边保持舒适的呼吸间距 -->
      <div class="scroll-bottom-spacer" aria-hidden="true" />
    </main>

    <!-- 新增/编辑账号弹窗 -->
    <AccountEditModal
      v-model:show="editModalOpen"
      :account="selectedEditAccount"
      @saved="handleRefreshAll"
    />

    <!-- 账号启停与删除管理弹窗 -->
    <AccountManageModal
      v-model:show="manageModalOpen"
      @saved="handleRefreshAll"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, watch, h } from 'vue'
import { useI18n } from 'vue-i18n'
import { NDropdown, useMessage, type DropdownOption } from 'naive-ui'
import { Icon } from '@iconify/vue'
import WorkbenchButton from '@/components/workbench/WorkbenchButton.vue'
import QuotaCard from '@/components/quota/QuotaCard.vue'
import AccountEditModal from '@/components/quota/AccountEditModal.vue'
import AccountManageModal from '@/components/quota/AccountManageModal.vue'
import { useQuotaStore } from '@/stores/quota'
import {
  loadQuotaAccounts,
  saveQuotaAccounts,
  discoverLocalAiAccounts,
  launchAiClient,
  type AccountConfig,
  type ProviderQuota,
  type ProviderType,
} from '@/services/quota/quota-service'

const { t } = useI18n({ useScope: 'global' })
const message = useMessage()
const quotaStore = useQuotaStore()

const quotas = computed(() => quotaStore.quotas)
const summary = computed(() => quotaStore.summary)
const loading = computed(() => quotaStore.loading)
type DiscoveryStage = 'idle' | 'scanning' | 'syncing'
const discoveryStage = ref<DiscoveryStage>('idle')
const discovering = computed(() => discoveryStage.value !== 'idle')
const isScanning = computed(() => discoveryStage.value === 'scanning')
const refreshingId = ref<string | null>(null)
const launchingId = ref<string | null>(null)

type CategoryType = 'all' | 'issues' | 'local' | 'api'
const activeCategory = ref<CategoryType>('all')

const LOCAL_CLIENT_PROVIDERS: ProviderType[] = ['codex', 'cursor', 'qcode', 'trae', 'zcode', 'workbuddy']

function isLocalClientQuota(quota: ProviderQuota): boolean {
  if (LOCAL_CLIENT_PROVIDERS.includes(quota.providerType)) return true
  if (quota.providerType !== 'gemini') return false
  return `${quota.name} ${quota.plan ?? ''}`.toLowerCase().includes('antigravity')
}

function needsAttention(quota: ProviderQuota): boolean {
  return !quota.isHealthy || Boolean(quota.errorMessage) || quota.pace?.level === 'overPace'
}

const healthyAccountsCount = computed(() => quotas.value.filter(quota => !needsAttention(quota)).length)
const attentionCount = computed(() => quotas.value.filter(needsAttention).length)

const categoryTabs = computed(() => {
  const allCount = quotas.value.length
  const localCount = quotas.value.filter(isLocalClientQuota).length

  return [
    { id: 'all' as const, label: t('quota.categoryAll'), count: allCount },
    { id: 'issues' as const, label: t('quota.categoryIssues'), count: attentionCount.value },
    { id: 'local' as const, label: t('quota.categoryLocal'), count: localCount },
    { id: 'api' as const, label: t('quota.categoryApi'), count: allCount - localCount },
  ]
})

const filteredQuotas = computed(() => {
  if (activeCategory.value === 'all') return quotas.value
  if (activeCategory.value === 'issues') return quotas.value.filter(needsAttention)
  if (activeCategory.value === 'local') return quotas.value.filter(isLocalClientQuota)
  if (activeCategory.value === 'api') return quotas.value.filter(quota => !isLocalClientQuota(quota))
  return quotas.value
})

const editModalOpen = ref(false)
const manageModalOpen = ref(false)
const selectedEditAccount = ref<AccountConfig | null>(null)

const autoRefreshInterval = ref<number>(300) // 默认 5 分钟 (300 秒)
let timerId: number | null = null

const autoRefreshOptions = computed(() => [
  { label: t('quota.autoRefreshOff'), value: 0 },
  { label: `5 ${t('quota.minutes')}`, value: 300 },
  { label: `15 ${t('quota.minutes')}`, value: 900 },
  { label: `30 ${t('quota.minutes')}`, value: 1800 },
])

const autoRefreshLabel = computed(() => {
  const selected = autoRefreshOptions.value.find(option => option.value === autoRefreshInterval.value)
  return selected?.label ?? t('quota.autoRefreshOff')
})

const moreActionOptions = computed<DropdownOption[]>(() => [
  { label: t('quota.manageAccounts'), key: 'manage', icon: () => h(Icon, { icon: 'solar:settings-linear' }) },
  { label: t('quota.discoverLocal'), key: 'discover', disabled: discovering.value, icon: () => h(Icon, { icon: 'solar:radar-linear' }) },
  { type: 'divider', key: 'divider' },
  {
    label: t('quota.autoRefreshSettings'),
    key: 'auto-refresh',
    children: autoRefreshOptions.value.map(option => ({
      label: `${option.value === autoRefreshInterval.value ? '✓ ' : ''}${option.label}`,
      key: `refresh-${option.value}`,
    })),
  },
])

onMounted(async () => {
  // 保证进入页面时优先复用或即时刷新最新额度
  await quotaStore.ensureFresh(5000)
  setupAutoRefresh()
})

onUnmounted(() => {
  clearAutoRefresh()
})

watch(autoRefreshInterval, () => {
  setupAutoRefresh()
})

function setupAutoRefresh() {
  clearAutoRefresh()
  if (autoRefreshInterval.value > 0) {
    timerId = window.setInterval(() => {
      void handleRefreshAll(true)
    }, autoRefreshInterval.value * 1000)
  }
}

function clearAutoRefresh() {
  if (timerId !== null) {
    clearInterval(timerId)
    timerId = null
  }
}

function openAddModal() {
  selectedEditAccount.value = null
  editModalOpen.value = true
}

async function openEditById(accountId: string) {
  try {
    const accounts = await loadQuotaAccounts()
    const target = accounts.find((a) => a.id === accountId)
    if (target) {
      selectedEditAccount.value = target
      editModalOpen.value = true
    }
  } catch (err) {
    message.error(String(err))
  }
}

async function handleDiscoverLocal() {
  discoveryStage.value = 'scanning'
  const previousQuotas = [...quotaStore.quotas]
  try {
    const discovered = await discoverLocalAiAccounts()
    discoveryStage.value = 'syncing'
    if (discovered.length === 0) {
      message.info(t('quota.noLocalDiscovered'))
      return
    }

    const currentAccounts = await loadQuotaAccounts()
    const newAccounts: AccountConfig[] = []
    for (const item of discovered) {
      if (!currentAccounts.some((a) => a.id === item.id || a.name === item.name)) {
        newAccounts.push(item)
      }
    }

    // 自动切回“全部”分类，确保扫描出的所有卡片都能被用户第一时间看到
    if (activeCategory.value !== 'all') {
      activeCategory.value = 'all'
    }

    if (newAccounts.length > 0) {
      // 乐观即时反馈：立即向 store 中注入加载占位卡片，用户在 100ms 内就能直观看到新增卡片，消除等待焦虑
      const placeholders = newAccounts.map((acc) => ({
        id: acc.id,
        accountId: acc.id,
        providerType: acc.providerType,
        name: acc.name,
        plan: '正在同步额度...',
        quotas: [],
        pace: undefined,
        resetCredits: undefined,
        lastUpdated: Date.now(),
        isHealthy: true,
        errorMessage: undefined,
        officialDashboardUrl: undefined,
      }))

      quotaStore.quotas = [...quotaStore.quotas, ...placeholders]

      const updatedAccounts = [...currentAccounts, ...newAccounts]
      await saveQuotaAccounts(updatedAccounts)

      // 后台静默刷新真实网络数据并无缝替换
      const refreshed = await handleRefreshAll(true)
      if (refreshed) {
        message.success(`成功发现并添加 ${newAccounts.length} 个本地客户端账号，已同步最新额度`)
      } else {
        quotaStore.quotas = previousQuotas
        message.warning(`已添加 ${newAccounts.length} 个本地客户端账号，但额度刷新失败，请稍后重试`)
      }
    } else {
      // 本地客户端账号此前已全部保存，先执行刷新确保展示为最新额度，再给出明确反馈
      const refreshed = await handleRefreshAll(true)
      if (refreshed) {
        message.info(`已检测到 ${discovered.length} 个本地客户端凭据（均已在看板中），已为您同步至最新额度`)
      } else {
        message.warning(`已检测到 ${discovered.length} 个本地客户端凭据，但额度刷新失败，请稍后重试`)
      }
    }
  } catch (err) {
    quotaStore.quotas = previousQuotas
    message.error(String(err))
  } finally {
    discoveryStage.value = 'idle'
  }
}

async function handleRefreshAll(silent = false): Promise<boolean> {
  try {
    await quotaStore.fetchAll(silent)
    return true
  } catch (err) {
    if (!silent) {
      message.error(t('quota.refreshFailed') + ': ' + String(err))
    }
    return false
  }
}

function handleMoreAction(key: string | number) {
  if (key === 'manage') {
    manageModalOpen.value = true
    return
  }
  if (key === 'discover') {
    void handleDiscoverLocal()
    return
  }
  if (typeof key === 'string' && key.startsWith('refresh-')) {
    const seconds = Number(key.slice('refresh-'.length))
    if (Number.isFinite(seconds)) autoRefreshInterval.value = seconds
  }
}

async function handleRefreshSingle(accountId: string) {
  refreshingId.value = accountId
  try {
    const updated = await quotaStore.refreshSingle(accountId)
    if (!updated) {
      message.warning(t('quota.accountNotFound'))
      return
    }
    message.success(t('quota.refreshSingleSuccess'))
  } catch (err) {
    message.error(String(err))
  } finally {
    refreshingId.value = null
  }
}

async function handleLaunchClient(accountId: string, providerType: ProviderType) {
  launchingId.value = accountId
  try {
    const result = await launchAiClient(providerType)
    message.success(t('quota.clientLaunched', { name: result.appName }))
    await new Promise(resolve => window.setTimeout(resolve, 1500))
    await handleRefreshSingle(accountId)
  } catch (err) {
    message.error(String(err))
  } finally {
    launchingId.value = null
  }
}
</script>

<style scoped lang="scss">
.quota-view-container {
  display: flex;
  flex-direction: column;
  height: 100%;
  padding: 20px 24px 64px;
  overflow-y: auto;
  gap: 18px;
  background: var(--lumina-content-bg);
}

.overview-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  flex-wrap: wrap;
}

.view-title-group {
  display: flex;
  flex-direction: column;
  gap: 2px;

  h2 {
    font-size: 18px;
    font-weight: 600;
    color: var(--lumina-text);
    margin: 0;
  }

  .view-subtitle {
    font-size: 12px;
    color: var(--lumina-text-secondary);
  }
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 10px;
}

.summary-strip {
  display: flex;
  align-items: center;
  gap: 16px;
  min-height: 52px;
  padding: 8px 12px;
  background: var(--lumina-surface-elevated);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-md);
  box-shadow: var(--lumina-shadow-sm);
  overflow-x: auto;
}

.summary-status,
.summary-issue,
.summary-assets,
.summary-refresh {
  display: inline-flex;
  align-items: center;
  white-space: nowrap;
  font-size: 12px;
  color: var(--lumina-text-secondary);

  strong {
    color: var(--lumina-text);
    font-family: var(--lumina-font-mono);
  }
}

.summary-status {
  gap: 6px;
}

.summary-status-icon {
  display: inline-flex;
  color: #10b981;
  svg {
    width: 16px;
    height: 16px;
  }
}

.summary-issue {
  gap: 5px;
  padding: 5px 8px;
  border: 0;
  border-radius: var(--lumina-radius-sm);
  background: transparent;
  cursor: pointer;

  &.active {
    color: #d97706;
    background: rgba(245, 158, 11, 0.1);
  }

  &:hover {
    background: var(--lumina-control-hover);
  }
}

.summary-divider {
  width: 0.5px;
  height: 24px;
  background: var(--lumina-separator);
  flex-shrink: 0;
}

.summary-assets {
  gap: 18px;

  span {
    display: inline-flex;
    align-items: baseline;
    gap: 5px;
    color: var(--lumina-text);
    font-family: var(--lumina-font-mono);
    font-weight: 600;
  }

  small {
    color: var(--lumina-text-tertiary);
    font: 500 9px var(--lumina-font-sans);
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
}

.summary-refresh {
  gap: 5px;
  margin-left: auto;
  color: var(--lumina-text-tertiary);

  svg {
    width: 14px;
    height: 14px;
  }
}

.scan-progress-banner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 16px;
  border-radius: var(--lumina-radius-md);
  background: linear-gradient(135deg, rgba(59, 130, 246, 0.08) 0%, rgba(99, 102, 241, 0.08) 100%);
  border: 1px solid rgba(59, 130, 246, 0.25);
  box-shadow: var(--lumina-shadow-sm);
}

.scan-banner-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.scan-radar-icon {
  font-size: 20px;
  color: var(--lumina-accent);
}

.scan-banner-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.scan-banner-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--lumina-text);
}

.scan-banner-sub {
  font-size: 11px;
  color: var(--lumina-text-secondary);
}

.scan-banner-badge {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  font-weight: 500;
  padding: 3px 10px;
  border-radius: 20px;
  background: rgba(59, 130, 246, 0.15);
  color: var(--lumina-accent);
  border: 0.5px solid rgba(59, 130, 246, 0.3);
}

.pulse-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--lumina-accent);
  animation: pulse-dot-anim 1.5s infinite;
}

@keyframes pulse-dot-anim {
  0%, 100% {
    opacity: 1;
    transform: scale(1);
  }
  50% {
    opacity: 0.3;
    transform: scale(0.75);
  }
}

.category-filter-bar {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px;
  background: var(--lumina-surface-elevated);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-md);
  width: fit-content;
}

.category-tab-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 5px 12px;
  font-size: 12px;
  font-weight: 500;
  color: var(--lumina-text-secondary);
  background: transparent;
  border: none;
  border-radius: var(--lumina-radius-sm);
  cursor: pointer;
  transition: all 0.15s ease;

  &:hover {
    color: var(--lumina-text);
    background: var(--lumina-control-hover);
  }

  &.active {
    color: var(--lumina-text);
    background: var(--lumina-control-bg);
    font-weight: 600;
    box-shadow: var(--lumina-shadow-sm);
  }

  .tab-count {
    font-size: 10px;
    padding: 1px 6px;
    border-radius: 10px;
    background: var(--lumina-separator);
    color: var(--lumina-text-secondary);
  }

  &.active .tab-count {
    background: color-mix(in srgb, var(--lumina-accent) 20%, transparent);
    color: var(--lumina-accent);
  }
}

.cards-viewport {
  flex: 1;
  min-height: 0;
  width: 100%;
}

.empty-filtered-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 48px;
  color: var(--lumina-text-tertiary);
  font-size: 13px;

  svg {
    width: 32px;
    height: 32px;
    opacity: 0.5;
  }
}

.cards-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(330px, 1fr));
  gap: 16px;
  align-items: stretch;
  margin-bottom: 24px;
}

@media (max-width: 900px) {
  .overview-header,
  .header-actions {
    align-items: flex-start;
  }

  .summary-refresh {
    margin-left: 0;
  }
}

.scroll-bottom-spacer {
  height: 64px;
  min-height: 64px;
  flex-shrink: 0;
  width: 100%;
  pointer-events: none;
}

.loading-state,
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 60px 0;
  gap: 12px;
  color: var(--lumina-text-secondary);
}

.empty-illustration {
  width: 56px;
  height: 56px;
  border-radius: 50%;
  background: var(--lumina-control-bg);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--lumina-text-tertiary);

  svg {
    width: 28px;
    height: 28px;
  }
}

.empty-state h3 {
  margin: 0;
  font-size: 16px;
  color: var(--lumina-text);
}

.empty-state p {
  margin: 0;
  font-size: 12px;
  max-width: 360px;
  text-align: center;
  color: var(--lumina-text-tertiary);
}

.empty-actions {
  display: flex;
  gap: 10px;
  margin-top: 6px;
}

.spin-anim {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  100% {
    transform: rotate(360deg);
  }
}
</style>
