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
        <div class="auto-refresh-selector">
          <Icon icon="solar:history-linear" />
          <NSelect
            v-model:value="autoRefreshInterval"
            size="small"
            style="width: 130px"
            :options="autoRefreshOptions"
          />
        </div>

        <WorkbenchButton :disabled="loading" @click="handleRefreshAll">
          <Icon icon="solar:restart-linear" :class="{ 'spin-anim': loading }" />
          {{ loading ? t('quota.refreshing') : t('quota.refreshAll') }}
        </WorkbenchButton>

        <WorkbenchButton :title="t('quota.copySummaryTitle')" @click="handleCopyShareSummary">
          <Icon icon="solar:copy-linear" />
          {{ t('quota.copySummary') }}
        </WorkbenchButton>

        <WorkbenchButton @click="manageModalOpen = true">
          <Icon icon="solar:settings-linear" />
          {{ t('quota.manageAccounts') }}
        </WorkbenchButton>

        <WorkbenchButton :disabled="discovering" @click="handleDiscoverLocal">
          <Icon icon="solar:radar-linear" :class="{ 'spin-anim': discovering }" />
          {{ t('quota.discoverLocal') }}
        </WorkbenchButton>

        <WorkbenchButton variant="primary" @click="openAddModal">
          <Icon icon="solar:add-circle-linear" />
          {{ t('quota.addAccount') }}
        </WorkbenchButton>
      </div>
    </header>

    <!-- 统计总览卡片 -->
    <section class="summary-metrics-grid">
      <div class="metric-card">
        <div class="metric-icon-wrap cny">
          <Icon icon="solar:wallet-money-linear" />
        </div>
        <div class="metric-content">
          <span class="metric-label">{{ t('quota.totalCnyBalance') }}</span>
          <div class="metric-value-row">
            <span class="metric-symbol">¥</span>
            <strong class="metric-value">{{ summary.totalCnyBalance.toFixed(2) }}</strong>
          </div>
        </div>
      </div>

      <div class="metric-card">
        <div class="metric-icon-wrap usd">
          <Icon icon="solar:dollar-linear" />
        </div>
        <div class="metric-content">
          <span class="metric-label">{{ t('quota.totalUsdBalance') }}</span>
          <div class="metric-value-row">
            <span class="metric-symbol">$</span>
            <strong class="metric-value">{{ summary.totalUsdBalance.toFixed(2) }}</strong>
          </div>
        </div>
      </div>

      <div class="metric-card">
        <div class="metric-icon-wrap active">
          <Icon icon="solar:check-circle-linear" />
        </div>
        <div class="metric-content">
          <span class="metric-label">{{ t('quota.activeAccounts') }}</span>
          <div class="metric-value-row">
            <strong class="metric-value">{{ summary.activeAccountsCount }}</strong>
            <span class="metric-unit">{{ t('quota.accountsCountUnit') }}</span>
          </div>
        </div>
      </div>

      <div class="metric-card" :class="{ 'has-warnings': summary.warningAccountsCount > 0 }">
        <div class="metric-icon-wrap warning">
          <Icon icon="solar:danger-triangle-linear" />
        </div>
        <div class="metric-content">
          <span class="metric-label">{{ t('quota.warningAccounts') }}</span>
          <div class="metric-value-row">
            <strong class="metric-value">{{ summary.warningAccountsCount }}</strong>
            <span class="metric-unit">{{ t('quota.accountsCountUnit') }}</span>
          </div>
        </div>
      </div>
    </section>

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

      <div v-else class="cards-grid">
        <QuotaCard
          v-for="item in quotas"
          :key="item.id"
          :quota="item"
          :refreshing="refreshingId === item.accountId"
          @edit="openEditById"
          @refresh="handleRefreshSingle"
        />
      </div>
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
import { ref, onMounted, onUnmounted, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { NSelect, useMessage } from 'naive-ui'
import WorkbenchButton from '@/components/workbench/WorkbenchButton.vue'
import QuotaCard from '@/components/quota/QuotaCard.vue'
import AccountEditModal from '@/components/quota/AccountEditModal.vue'
import AccountManageModal from '@/components/quota/AccountManageModal.vue'
import {
  loadAllQuotas,
  refreshSingleQuota,
  loadQuotaAccounts,
  saveQuotaAccounts,
  discoverLocalAiAccounts,
  type AccountConfig,
  type ProviderQuota,
  type QuotaSummary,
} from '@/services/quota/quota-service'

const { t } = useI18n({ useScope: 'global' })
const message = useMessage()

const quotas = ref<ProviderQuota[]>([])
const summary = ref<QuotaSummary>({
  totalCnyBalance: 0,
  totalUsdBalance: 0,
  activeAccountsCount: 0,
  warningAccountsCount: 0,
})
const loading = ref(false)
const discovering = ref(false)
const refreshingId = ref<string | null>(null)

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

onMounted(async () => {
  await handleRefreshAll()
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

function handleCopyShareSummary() {
  if (quotas.value.length === 0) {
    message.warning(t('quota.noAccountsConfigured'))
    return
  }

  const lines = [
    '📊 我的 AI 算力与额度看板 (via Lumina)',
    `💰 资产总览: ¥${summary.value.totalCnyBalance.toFixed(2)} CNY | $${summary.value.totalUsdBalance.toFixed(2)} USD`,
  ]

  for (const q of quotas.value) {
    const parts: string[] = []
    for (const item of q.quotas) {
      if (item.type === 'balance') {
        const symbol = item.currency === 'USD' ? '$' : '¥'
        parts.push(`余额 ${symbol}${item.totalRemaining.toFixed(2)}`)
      } else if (item.type === 'rateLimit') {
        const rem = Math.max(0, 100 - item.usedPercent).toFixed(0)
        parts.push(`${item.periodLabel} 剩余 ${rem}%`)
      }
    }
    const paceStr = q.pace ? ` [${q.pace.level === 'onPace' ? '🟢 节奏健康' : q.pace.level === 'overPace' ? '🔴 预计超标' : '🟡 用量偏紧'}]` : ''
    lines.push(`• ${q.name}: ${parts.join(' · ')}${paceStr}`)
  }

  const text = lines.join('\n')
  navigator.clipboard
    .writeText(text)
    .then(() => {
      message.success(t('quota.shareSummaryCopied'))
    })
    .catch(() => {
      message.error('复制失败，请检查剪贴板权限')
    })
}

async function handleDiscoverLocal() {
  discovering.value = true
  try {
    const discovered = await discoverLocalAiAccounts()
    if (discovered.length === 0) {
      message.info(t('quota.noLocalDiscovered'))
      return
    }

    const currentAccounts = await loadQuotaAccounts()
    let addedCount = 0
    for (const item of discovered) {
      if (!currentAccounts.some((a) => a.id === item.id || a.name === item.name)) {
        currentAccounts.push(item)
        addedCount++
      }
    }

    if (addedCount > 0) {
      await saveQuotaAccounts(currentAccounts)
      message.success(`${t('quota.discoveredAccountsSuccess')}: ${addedCount}`)
      await handleRefreshAll()
    } else {
      message.info(t('quota.allLocalAlreadyAdded'))
    }
  } catch (err) {
    message.error(String(err))
  } finally {
    discovering.value = false
  }
}

async function handleRefreshAll(silent = false) {
  if (!silent) loading.value = true
  try {
    const [fetchedQuotas, fetchedSummary] = await loadAllQuotas()
    quotas.value = fetchedQuotas
    summary.value = fetchedSummary
  } catch (err) {
    if (!silent) {
      message.error(t('quota.refreshFailed') + ': ' + String(err))
    }
  } finally {
    if (!silent) loading.value = false
  }
}

async function handleRefreshSingle(accountId: string) {
  refreshingId.value = accountId
  try {
    const accounts = await loadQuotaAccounts()
    const target = accounts.find((a) => a.id === accountId)
    if (!target) {
      message.warning(t('quota.accountNotFound'))
      return
    }

    const updated = await refreshSingleQuota(target)
    const idx = quotas.value.findIndex((q) => q.accountId === accountId)
    if (idx !== -1) {
      quotas.value[idx] = updated
    } else {
      quotas.value.push(updated)
    }
    message.success(t('quota.refreshSingleSuccess'))
  } catch (err) {
    message.error(String(err))
  } finally {
    refreshingId.value = null
  }
}
</script>

<style scoped lang="scss">
.quota-view-container {
  display: flex;
  flex-direction: column;
  height: 100%;
  padding: 20px 24px;
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

.auto-refresh-selector {
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--lumina-text-secondary);

  svg {
    width: 15px;
    height: 15px;
  }
}

.summary-metrics-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 14px;
}

.metric-card {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 14px 16px;
  background: var(--lumina-surface-elevated);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-lg);
  box-shadow: var(--lumina-shadow-sm);

  &.has-warnings {
    border-color: rgba(245, 158, 11, 0.4);
  }
}

.metric-icon-wrap {
  width: 40px;
  height: 40px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;

  svg {
    width: 22px;
    height: 22px;
  }

  &.cny {
    background: rgba(2, 132, 199, 0.1);
    color: #0284c7;
  }

  &.usd {
    background: rgba(16, 185, 129, 0.1);
    color: #10b981;
  }

  &.active {
    background: rgba(99, 102, 241, 0.1);
    color: #6366f1;
  }

  &.warning {
    background: rgba(245, 158, 11, 0.1);
    color: #f59e0b;
  }
}

.metric-content {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.metric-label {
  font-size: 11px;
  color: var(--lumina-text-tertiary);
  text-transform: uppercase;
  letter-spacing: 0.03em;
}

.metric-value-row {
  display: flex;
  align-items: baseline;
  gap: 4px;
}

.metric-symbol {
  font-size: 14px;
  font-weight: 600;
  color: var(--lumina-text-secondary);
}

.metric-value {
  font-size: 22px;
  font-weight: 700;
  font-family: var(--lumina-font-mono);
  color: var(--lumina-text);
}

.metric-unit {
  font-size: 11px;
  color: var(--lumina-text-tertiary);
  margin-left: 2px;
}

.cards-viewport {
  flex: 1;
  min-height: 0;
}

.cards-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: 16px;
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
