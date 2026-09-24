<template>
  <NModal
    v-model:show="quotaStore.modalVisible"
    :auto-focus="false"
    :trap-focus="false"
    :mask-closable="true"
    :close-on-esc="true"
    transform-origin="center"
  >
    <section class="macos-quota-window" :class="{ maximized }" role="dialog" aria-modal="true" :aria-label="t('quota.viewTitle')">
      <!-- 1. Window-Level Chrome Header: Strictly Window Identity & Window Actions -->
      <header class="quota-titlebar" @dblclick="handleTitlebarDoubleClick">
        <div class="title-identity">
          <span class="app-icon">
            <Icon icon="solar:wallet-money-linear" />
          </span>
          <div>
            <h2 id="quota-window-title">{{ t('quota.viewTitle') }}</h2>
            <p>{{ t('quota.viewSubtitle', '多平台 Token 与余额资产概览 · 自动化配额跟踪与告警') }}</p>
          </div>
        </div>

        <div class="window-actions">
          <button
            class="maximize-button"
            type="button"
            :title="maximized ? t('workbench.restore', '还原窗口') : t('workbench.maximize', '最大化窗口')"
            :aria-label="maximized ? t('workbench.restore', '还原窗口') : t('workbench.maximize', '最大化窗口')"
            @click.stop="toggleMaximized"
          >
            <Icon :icon="maximized ? 'solar:minimize-square-3-linear' : 'solar:maximize-square-3-linear'" />
          </button>
          <WorkbenchCloseButton
            @close="quotaStore.closeModal"
          />
        </div>
      </header>

      <!-- 2. Workspace Subnav: Page-Level Actions (Separated from modal window operations) -->
      <nav class="quota-subnav" aria-label="额度资产操作与状态">
        <div class="subnav-leading">
          <div class="health-summary-pill" :class="{ 'has-issues': attentionCount > 0 }">
            <span class="health-indicator-dot"></span>
            <span class="health-summary-text">
              {{ healthyAccountsCount }}/{{ quotas.length }} {{ t('quota.accountsHealthy') }}
              <template v-if="attentionCount > 0">
                · <span class="highlight-warn">{{ attentionCount }} {{ t('quota.needsAttention') }}</span>
              </template>
            </span>
          </div>
        </div>

        <div class="workspace-actions">
          <!-- 1. 扫描本地客户端凭据 -->
          <button
            type="button"
            class="page-action-btn"
            :disabled="discovering"
            :title="t('quota.discoverLocal')"
            @click="handleDiscoverLocal"
          >
            <Icon :icon="isScanning ? 'solar:radar-bold' : 'solar:radar-linear'" :class="{ 'spin-glyph': isScanning }" />
            <span>{{ isScanning ? t('quota.scanningLocalBadge') : t('quota.discoverLocal') }}</span>
          </button>

          <!-- 2. 添加账号 -->
          <button
            type="button"
            class="page-action-btn page-action-btn--primary"
            :title="t('quota.addAccount')"
            @click="openAddModal"
          >
            <Icon icon="solar:add-circle-linear" />
            <span>{{ t('quota.addAccount') }}</span>
          </button>

          <!-- 3. 管理账号 -->
          <button
            type="button"
            class="page-action-btn"
            :title="t('quota.manageAccounts')"
            @click="manageModalOpen = true"
          >
            <Icon icon="solar:settings-linear" />
            <span>{{ t('quota.manageAccounts') }}</span>
          </button>

          <!-- 4. 刷新全部 -->
          <button
            type="button"
            class="page-action-btn"
            :disabled="loading"
            :title="t('quota.refreshAll')"
            @click="handleRefreshAll(false)"
          >
            <Icon icon="solar:restart-linear" :class="{ 'spin-glyph': loading }" />
            <span>{{ loading ? t('quota.refreshing') : t('quota.refreshAll') }}</span>
          </button>
        </div>
      </nav>

      <!-- macOS 风格仪表盘统计栏 (5 个独立精致 Metric Tiles) -->
      <section class="macos-metric-strip" aria-label="额度资产总览">
        <!-- 1. 账号健康度 Tile -->
        <div
          class="metric-tile metric-tile--status"
          :class="{ 'has-issues': attentionCount > 0, active: activeCategory === 'issues' }"
          role="button"
          tabindex="0"
          :title="attentionCount > 0 ? '点击快速筛选需处理账号' : '全部账号运行健康'"
          @click="activeCategory = activeCategory === 'issues' ? 'all' : 'issues'"
        >
          <div class="tile-icon-box tile-icon-box--status">
            <Icon :icon="attentionCount > 0 ? 'solar:danger-triangle-bold' : 'solar:shield-check-bold'" />
          </div>
          <div class="tile-content">
            <div class="tile-main-row">
              <span class="tile-value">{{ healthyAccountsCount }}<span class="tile-total">/{{ quotas.length }}</span></span>
              <span v-if="attentionCount > 0" class="tile-badge-warn">{{ attentionCount }} 需关注</span>
              <span v-else class="tile-badge-ok">正常</span>
            </div>
            <span class="tile-label">{{ attentionCount > 0 ? t('quota.needsAttention') : t('quota.statusHealthy') }}</span>
          </div>
        </div>

        <!-- 2. 美元余额 Tile (USD) -->
        <div class="metric-tile metric-tile--usd">
          <div class="tile-icon-box tile-icon-box--usd">
            <span>$</span>
          </div>
          <div class="tile-content">
            <span class="tile-value">${{ summary.totalUsdBalance.toFixed(2) }}</span>
            <span class="tile-label">{{ t('quota.totalUsdBalance') }}</span>
          </div>
        </div>

        <!-- 3. 人民币余额 Tile (CNY) -->
        <div class="metric-tile metric-tile--cny">
          <div class="tile-icon-box tile-icon-box--cny">
            <span>¥</span>
          </div>
          <div class="tile-content">
            <span class="tile-value">¥{{ summary.totalCnyBalance.toFixed(2) }}</span>
            <span class="tile-label">{{ t('quota.totalCnyBalance') }}</span>
          </div>
        </div>

        <!-- 4. 可用积分 Tile (Credits) -->
        <div class="metric-tile metric-tile--credits">
          <div class="tile-icon-box tile-icon-box--credits">
            <Icon icon="solar:stars-minimalistic-bold" />
          </div>
          <div class="tile-content">
            <span class="tile-value">{{ (summary.totalCredits || 0).toLocaleString() }}</span>
            <span class="tile-label">{{ t('quota.totalCredits') }} ({{ t('quota.creditsUnit') }})</span>
          </div>
        </div>

        <!-- 5. 自动刷新配置 Tile (直接挂在 Tile 上，不会遮挡任何内容) -->
        <NDropdown :options="autoRefreshDropdownOptions" trigger="click" @select="handleRefreshIntervalSelect">
          <div class="metric-tile metric-tile--refresh" role="button" tabindex="0" :title="t('quota.autoRefreshSettings')">
            <div class="tile-icon-box tile-icon-box--refresh">
              <Icon icon="solar:history-bold" />
            </div>
            <div class="tile-content">
              <div class="tile-main-row">
                <span class="tile-value tile-value--sm">{{ autoRefreshLabel }}</span>
                <Icon icon="solar:alt-arrow-down-linear" class="tile-chevron" />
              </div>
              <span class="tile-label">{{ t('quota.autoRefreshSettings') }}</span>
            </div>
          </div>
        </NDropdown>
      </section>

      <!-- macOS Toolbar: Search & Segmented Filter Bar -->
      <div class="macos-window-toolbar">
        <div class="macos-search-box" @click="focusSearch">
          <Icon icon="solar:magnifer-linear" class="search-glyph" />
          <input
            ref="searchInputRef"
            v-model="searchQuery"
            type="text"
            class="macos-search-field"
            :placeholder="t('quota.searchPlaceholder')"
            autocomplete="off"
            spellcheck="false"
            @click.stop
            @mousedown.stop
            @keydown.stop="handleInputKeydown"
          />
          <kbd v-if="!searchQuery" class="search-kbd">{{ isMac ? '⌘F' : 'Ctrl+F' }}</kbd>
          <button
            v-else
            type="button"
            class="search-clear-glyph"
            :title="t('common.clear', '清空')"
            @click.stop="clearSearch"
          >
            ×
          </button>
        </div>

        <!-- macOS Segmented Control (严格保持 flex-shrink: 0，不再受遮挡) -->
        <div class="macos-segmented-bar" role="tablist">
          <button
            v-for="tab in categoryTabs"
            :key="tab.id"
            type="button"
            role="tab"
            class="segmented-tab"
            :class="{
              active: activeCategory === tab.id,
              'segmented-tab--warning': tab.id === 'issues' && tab.count > 0,
            }"
            @click="activeCategory = tab.id"
          >
            <span v-if="tab.id === 'issues' && tab.count > 0" class="dot-indicator"></span>
            {{ tab.label }}
            <span
              class="tab-badge"
              :class="{ 'tab-badge--warn': tab.id === 'issues' && tab.count > 0 }"
            >
              {{ tab.count }}
            </span>
          </button>
        </div>
      </div>

      <!-- 扫描进行中状态提示横幅 -->
      <transition name="fade">
        <div v-if="discovering" class="scan-progress-banner">
          <div class="scan-banner-left">
            <Icon :icon="isScanning ? 'solar:radar-bold' : 'solar:restart-linear'" class="scan-radar-icon spin-glyph" />
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

      <!-- Content Area: Cards Grid Viewport -->
      <main class="macos-content-viewport">
        <div v-if="loading && quotas.length === 0" class="macos-empty-state">
          <Icon icon="solar:restart-linear" class="spin-glyph" />
          <p>{{ t('quota.loadingQuotas') }}</p>
        </div>

        <div v-else-if="quotas.length === 0" class="macos-empty-state">
          <div class="empty-illustration">
            <Icon icon="solar:box-minimalistic-linear" />
          </div>
          <h3>{{ t('quota.noAccountsConfigured') }}</h3>
          <p>{{ t('quota.emptyGuide') }}</p>
          <div class="empty-actions">
            <button type="button" class="macos-push-btn" :disabled="discovering" @click="handleDiscoverLocal">
              <Icon icon="solar:radar-linear" />
              <span>{{ t('quota.discoverLocal') }}</span>
            </button>
            <button type="button" class="macos-push-btn macos-push-btn--primary" @click="openAddModal">
              <Icon icon="solar:add-circle-linear" />
              <span>{{ t('quota.addAccount') }}</span>
            </button>
          </div>
        </div>

        <div v-else-if="filteredQuotas.length === 0" class="macos-empty-state">
          <Icon icon="solar:filter-linear" class="empty-glyph" />
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

        <div class="scroll-bottom-spacer" aria-hidden="true" />
      </main>
    </section>
  </NModal>

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
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { NDropdown, NModal, useMessage, type DropdownOption } from 'naive-ui'
import { Icon } from '@iconify/vue'
import { useI18n } from 'vue-i18n'
import QuotaCard from './QuotaCard.vue'
import AccountEditModal from './AccountEditModal.vue'
import AccountManageModal from './AccountManageModal.vue'
import WorkbenchCloseButton from '@/components/workbench/WorkbenchCloseButton.vue'
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
import { hasPrimaryModifier, isMacPlatform } from '@/utils/platform-shortcuts'

const { t } = useI18n({ useScope: 'global' })
const message = useMessage()
const quotaStore = useQuotaStore()
const isMac = isMacPlatform

const quotas = computed(() => quotaStore.quotas)
const summary = computed(() => quotaStore.summary)
const loading = computed(() => quotaStore.loading)

const maximized = ref(false)

function toggleMaximized() {
  maximized.value = !maximized.value
}

function handleTitlebarDoubleClick(event: MouseEvent) {
  if (!(event.target instanceof Element) || !event.target.closest('button')) {
    toggleMaximized()
  }
}

type DiscoveryStage = 'idle' | 'scanning' | 'syncing'
const discoveryStage = ref<DiscoveryStage>('idle')
const discovering = computed(() => discoveryStage.value !== 'idle')
const isScanning = computed(() => discoveryStage.value === 'scanning')
const refreshingId = ref<string | null>(null)
const launchingId = ref<string | null>(null)

type CategoryType = 'all' | 'issues' | 'local' | 'api'
const activeCategory = ref<CategoryType>('all')
const searchQuery = ref('')
const searchInputRef = ref<HTMLInputElement | null>(null)

const LOCAL_CLIENT_PROVIDERS: ProviderType[] = ['codex', 'cursor', 'qcode', 'trae', 'zcode', 'workbuddy', 'mimo']

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
  let list = quotas.value

  if (activeCategory.value === 'issues') {
    list = list.filter(needsAttention)
  } else if (activeCategory.value === 'local') {
    list = list.filter(isLocalClientQuota)
  } else if (activeCategory.value === 'api') {
    list = list.filter(quota => !isLocalClientQuota(quota))
  }

  const query = searchQuery.value.trim().toLowerCase()
  if (!query) return list

  return list.filter(item => {
    const name = item.name.toLowerCase()
    const provider = item.providerType.toLowerCase()
    const plan = (item.plan || '').toLowerCase()
    const err = (item.errorMessage || '').toLowerCase()
    return name.includes(query) || provider.includes(query) || plan.includes(query) || err.includes(query)
  })
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

const autoRefreshDropdownOptions = computed<DropdownOption[]>(() =>
  autoRefreshOptions.value.map(option => ({
    label: `${option.value === autoRefreshInterval.value ? '✓ ' : '   '}${option.label}`,
    key: option.value,
  }))
)

function handleRefreshIntervalSelect(key: string | number) {
  const seconds = Number(key)
  if (Number.isFinite(seconds)) {
    autoRefreshInterval.value = seconds
  }
}

function focusSearch() {
  searchInputRef.value?.focus()
}

function clearSearch() {
  searchQuery.value = ''
  focusSearch()
}

function handleInputKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    if (searchQuery.value) {
      searchQuery.value = ''
    } else {
      quotaStore.closeModal()
    }
    event.stopPropagation()
    return
  }
  event.stopPropagation()
}

function handleFindShortcut(event: KeyboardEvent) {
  if (!quotaStore.modalVisible) return
  if (!hasPrimaryModifier(event) || event.key.toLowerCase() !== 'f') return
  event.preventDefault()
  event.stopPropagation()
  searchInputRef.value?.focus()
  searchInputRef.value?.select()
}

onMounted(() => {
  window.addEventListener('keydown', handleFindShortcut, true)
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleFindShortcut, true)
  clearAutoRefresh()
})

watch(
  () => quotaStore.modalVisible,
  visible => {
    if (visible) {
      void quotaStore.ensureFresh(5000)
      setupAutoRefresh()
      void nextTick(() => {
        searchInputRef.value?.focus()
      })
    } else {
      clearAutoRefresh()
    }
  }
)

watch(autoRefreshInterval, () => {
  if (quotaStore.modalVisible) {
    setupAutoRefresh()
  }
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

    if (activeCategory.value !== 'all') {
      activeCategory.value = 'all'
    }

    if (newAccounts.length > 0) {
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

      const refreshed = await handleRefreshAll(true)
      if (refreshed) {
        message.success(`成功发现并添加 ${newAccounts.length} 个本地客户端账号，已同步最新额度`)
      } else {
        quotaStore.quotas = previousQuotas
        message.warning(`已添加 ${newAccounts.length} 个本地客户端账号，但额度刷新失败，请稍后重试`)
      }
    } else {
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
.macos-quota-window {
  backdrop-filter: var(--lumina-vibrancy, blur(28px));
  background: color-mix(in srgb, var(--lumina-surface-elevated) 96%, transparent);
  border: 0.5px solid var(--lumina-separator-strong);
  border-radius: var(--lumina-radius-lg, 12px);
  box-shadow: var(--lumina-shadow-lg, 0 24px 64px rgba(0, 0, 0, 0.28));
  color: var(--lumina-text);
  display: flex;
  flex-direction: column;
  font-family: var(--lumina-font-sans);
  height: min(840px, calc(100vh - 40px));
  max-height: 840px;
  max-width: 1120px;
  overflow: hidden;
  transition: border-radius var(--lumina-duration-normal) var(--lumina-ease-out),
    height var(--lumina-duration-normal) var(--lumina-ease-out),
    width var(--lumina-duration-normal) var(--lumina-ease-out);
  user-select: none;
  width: min(1120px, calc(100vw - 40px));

  &.maximized {
    border-radius: 8px;
    height: calc(100vh - 12px);
    max-height: calc(100vh - 12px);
    max-width: calc(100vw - 12px);
    width: calc(100vw - 12px);
  }
}

/* macOS Window Chrome Header: Window Identity & Window Actions strictly separated */
.quota-titlebar {
  align-items: center;
  backdrop-filter: saturate(180%) blur(24px);
  background: var(--lumina-toolbar-bg);
  border-bottom: 0.5px solid var(--lumina-separator);
  display: flex;
  justify-content: space-between;
  padding: 7px 12px 7px 14px;
  user-select: none;
}

.title-identity {
  align-items: center;
  display: flex;
  gap: 10px;

  h2 {
    font-size: 14px;
    font-weight: 600;
    margin: 0;
  }

  p {
    color: var(--lumina-text-secondary);
    font-size: 11.5px;
    margin: 1px 0 0;
  }
}

.app-icon {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-primary) 12%, var(--lumina-surface-2));
  border-radius: 8px;
  color: var(--lumina-primary);
  display: flex;
  height: 32px;
  justify-content: center;
  width: 32px;

  svg {
    height: 17px;
    width: 17px;
  }
}

.window-actions {
  align-items: center;
  display: flex;
  gap: 2px;
}

.maximize-button {
  align-items: center;
  background: transparent;
  border: 0;
  border-radius: 6px;
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: flex;
  height: 28px;
  justify-content: center;
  padding: 0;
  width: 28px;

  svg {
    height: 15px;
    width: 15px;
  }

  &:hover {
    background: var(--lumina-control-hover);
    color: var(--lumina-text);
  }

  &:focus-visible {
    box-shadow: 0 0 0 3px var(--lumina-accent-ring);
    outline: none;
  }
}

/* Page-level Action Subnav: Separated from window operations */
.quota-subnav {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-surface-elevated) 88%, transparent);
  border-bottom: 0.5px solid var(--lumina-separator);
  display: flex;
  flex-shrink: 0;
  height: 48px;
  justify-content: space-between;
  padding: 0 16px;
  user-select: none;
}

.subnav-leading {
  align-items: center;
  display: flex;
  gap: 10px;
}

.health-summary-pill {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-fill-quaternary) 70%, transparent);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-md, 7px);
  display: inline-flex;
  gap: 8px;
  height: 28px;
  padding: 0 10px;

  .health-indicator-dot {
    background: var(--lumina-success, #27853f);
    border-radius: 50%;
    height: 6px;
    width: 6px;
  }

  &.has-issues .health-indicator-dot {
    background: var(--lumina-warning, #b76500);
  }

  .health-summary-text {
    color: var(--lumina-text);
    font-size: 11.5px;
    font-weight: 500;
  }

  .highlight-warn {
    color: var(--lumina-warning, #b76500);
    font-weight: 600;
  }
}

.workspace-actions {
  align-items: center;
  display: flex;
  gap: 8px;
}

.page-action-btn {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-fill-quaternary) 80%, transparent);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-sm, 6px);
  color: var(--lumina-text);
  cursor: pointer;
  display: inline-flex;
  font-size: 11.5px;
  font-weight: 500;
  gap: 5px;
  height: 28px;
  padding: 0 10px;
  transition: all 120ms ease;
  white-space: nowrap;

  &:hover:not(:disabled) {
    background: color-mix(in srgb, var(--lumina-fill-secondary) 85%, transparent);
  }

  &:disabled {
    cursor: not-allowed;
    opacity: 0.45;
  }

  &--primary {
    background: var(--lumina-primary, #39786f);
    border-color: transparent;
    color: #ffffff;
    font-weight: 600;

    &:hover:not(:disabled) {
      background: var(--lumina-primary, #39786f);
      filter: brightness(1.08);
    }
  }

  svg {
    font-size: 13px;
  }
}

/* macOS 风格仪表盘统计栏 (5 个独立精致 Metric Tiles) */
.macos-metric-strip {
  background: color-mix(in srgb, var(--lumina-toolbar-bg) 60%, transparent);
  border-bottom: 0.5px solid var(--lumina-separator);
  display: grid;
  flex-shrink: 0;
  gap: 8px;
  grid-template-columns: repeat(5, 1fr);
  padding: 10px 16px;

  @media (max-width: 960px) {
    grid-template-columns: repeat(3, 1fr);
  }
}

.metric-tile {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-surface-elevated) 75%, transparent);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-md, 8px);
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.03);
  cursor: default;
  display: flex;
  gap: 10px;
  padding: 8px 12px;
  transition: all var(--lumina-duration-fast) var(--lumina-ease-out);
  user-select: none;

  &[role="button"] {
    cursor: pointer;

    &:hover {
      background: var(--lumina-control-hover);
      border-color: var(--lumina-separator-strong);
    }
  }

  &.active {
    background: color-mix(in srgb, var(--lumina-primary) 12%, var(--lumina-surface-elevated));
    border-color: var(--lumina-primary);
  }

  &--status {
    &.has-issues {
      background: color-mix(in srgb, var(--lumina-warning) 8%, var(--lumina-surface-elevated));
      border-color: rgba(245, 158, 11, 0.35);
    }
  }
}

.tile-icon-box {
  align-items: center;
  background: var(--lumina-control-bg);
  border-radius: 7px;
  color: var(--lumina-text-secondary);
  display: flex;
  flex-shrink: 0;
  font-size: 15px;
  font-weight: 700;
  height: 30px;
  justify-content: center;
  width: 30px;

  &--status {
    background: color-mix(in srgb, var(--lumina-primary, #39786f) 15%, transparent);
    color: var(--lumina-primary, #39786f);

    .has-issues & {
      background: rgba(245, 158, 11, 0.15);
      color: #f59e0b;
    }
  }

  &--usd {
    background: rgba(14, 165, 233, 0.14);
    color: #0284c7;
    font-family: var(--lumina-font-mono);
    font-size: 14px;
  }

  &--cny {
    background: rgba(244, 63, 94, 0.14);
    color: #e11d48;
    font-family: var(--lumina-font-mono);
    font-size: 14px;
  }

  &--credits {
    background: rgba(168, 85, 247, 0.14);
    color: #9333ea;
    font-size: 16px;
  }

  &--refresh {
    background: rgba(100, 116, 139, 0.14);
    color: var(--lumina-text-secondary);
    font-size: 15px;
  }
}

.tile-content {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 1px;
  min-width: 0;
}

.tile-main-row {
  align-items: baseline;
  display: flex;
  gap: 6px;
}

.tile-value {
  color: var(--lumina-text);
  font-family: var(--lumina-font-mono);
  font-size: 13.5px;
  font-weight: 650;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;

  &--sm {
    font-size: 12px;
    font-weight: 600;
  }

  .tile-total {
    color: var(--lumina-text-tertiary);
    font-size: 11px;
    font-weight: 500;
  }
}

.tile-label {
  color: var(--lumina-text-tertiary);
  font-size: 10px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tile-badge-ok {
  background: color-mix(in srgb, var(--lumina-primary, #39786f) 12%, transparent);
  border-radius: 4px;
  color: var(--lumina-primary, #39786f);
  font-size: 9.5px;
  font-weight: 600;
  padding: 0.5px 5px;
}

.tile-badge-warn {
  background: rgba(245, 158, 11, 0.14);
  border-radius: 4px;
  color: #f59e0b;
  font-size: 9.5px;
  font-weight: 600;
  padding: 0.5px 5px;
}

.tile-chevron {
  color: var(--lumina-text-tertiary);
  font-size: 11px;
  margin-left: auto;
}

/* macOS Window Toolbar: Search + Tabs */
.macos-window-toolbar {
  align-items: center;
  background: var(--lumina-toolbar-bg, var(--lumina-surface-elevated));
  border-bottom: 0.5px solid var(--lumina-separator);
  display: flex;
  flex-shrink: 0;
  gap: 12px;
  height: 44px;
  justify-content: space-between;
  padding: 0 16px;
}

.macos-search-box {
  align-items: center;
  background: var(--lumina-control-bg);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-sm, 5px);
  cursor: text;
  display: flex;
  flex: 0 1 240px;
  height: 28px;
  min-width: 140px;
  padding: 0 8px;
  transition: all var(--lumina-duration-fast) var(--lumina-ease-out);
  user-select: text;

  &:focus-within {
    border-color: var(--lumina-primary);
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--lumina-primary) 22%, transparent);
  }

  .search-glyph {
    color: var(--lumina-text-tertiary);
    flex-shrink: 0;
    font-size: 13px;
    margin-right: 6px;
  }

  .macos-search-field {
    background: transparent;
    border: none;
    color: var(--lumina-text);
    font-family: inherit;
    font-size: 11.5px;
    outline: none;
    user-select: text;
    width: 100%;

    &::placeholder {
      color: var(--lumina-text-tertiary);
    }
  }

  .search-kbd {
    background: color-mix(in srgb, var(--lumina-text) 6%, transparent);
    border-radius: 3px;
    color: var(--lumina-text-tertiary);
    font-family: var(--lumina-font-mono);
    font-size: 9.5px;
    padding: 1px 4px;
    user-select: none;
  }

  .search-clear-glyph {
    background: transparent;
    border: none;
    color: var(--lumina-text-tertiary);
    cursor: pointer;
    font-size: 14px;
    line-height: 1;
    padding: 0 2px;

    &:hover {
      color: var(--lumina-text);
    }
  }
}

.macos-segmented-bar {
  align-items: center;
  background: var(--lumina-control-bg);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-sm, 6px);
  display: inline-flex;
  flex-shrink: 0;
  gap: 2px;
  padding: 2px;

  .segmented-tab {
    align-items: center;
    background: transparent;
    border: none;
    border-radius: 4px;
    color: var(--lumina-text-secondary);
    cursor: pointer;
    display: inline-flex;
    font-size: 11px;
    font-weight: 500;
    gap: 5px;
    height: 22px;
    padding: 0 8px;
    transition: all var(--lumina-duration-fast) var(--lumina-ease-out);

    &:hover {
      color: var(--lumina-text);
    }

    &.active {
      background: var(--lumina-surface-elevated, #ffffff);
      box-shadow: 0 1px 2px rgba(0, 0, 0, 0.1);
      color: var(--lumina-text);
      font-weight: 600;
    }

    .tab-badge {
      background: color-mix(in srgb, var(--lumina-text) 8%, transparent);
      border-radius: 8px;
      color: var(--lumina-text-tertiary);
      font-size: 9.5px;
      font-weight: 600;
      padding: 0 5px;

      &--warn {
        background: color-mix(in srgb, var(--lumina-warning) 16%, transparent);
        color: var(--lumina-warning);
      }
    }

    .dot-indicator {
      background: var(--lumina-warning, #b76500);
      border-radius: 50%;
      height: 5px;
      width: 5px;
    }
  }
}

/* 扫描进行中状态提示横幅 */
.scan-progress-banner {
  align-items: center;
  background: linear-gradient(135deg, rgba(59, 130, 246, 0.08) 0%, rgba(99, 102, 241, 0.08) 100%);
  border-bottom: 1px solid rgba(59, 130, 246, 0.25);
  display: flex;
  flex-shrink: 0;
  gap: 12px;
  justify-content: space-between;
  padding: 10px 16px;

  .scan-banner-left {
    align-items: center;
    display: flex;
    gap: 10px;

    .scan-radar-icon {
      color: var(--lumina-accent, #3b82f6);
      font-size: 18px;
    }

    .scan-banner-info {
      display: flex;
      flex-direction: column;
      gap: 1px;

      .scan-banner-title {
        color: var(--lumina-text);
        font-size: 12px;
        font-weight: 600;
      }

      .scan-banner-sub {
        color: var(--lumina-text-secondary);
        font-size: 10.5px;
      }
    }
  }

  .scan-banner-badge {
    align-items: center;
    background: rgba(59, 130, 246, 0.15);
    border: 0.5px solid rgba(59, 130, 246, 0.3);
    border-radius: 20px;
    color: var(--lumina-accent, #3b82f6);
    display: inline-flex;
    font-size: 10.5px;
    font-weight: 500;
    gap: 6px;
    padding: 2px 8px;

    .pulse-dot {
      animation: pulse-dot-anim 1.5s infinite;
      background: var(--lumina-accent, #3b82f6);
      border-radius: 50%;
      height: 6px;
      width: 6px;
    }
  }
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

/* Content Area: Cards Viewport */
.macos-content-viewport {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 16px;
  width: 100%;
}

.cards-grid {
  align-items: stretch;
  display: grid;
  gap: 14px;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  margin-bottom: 16px;
}

.scroll-bottom-spacer {
  flex-shrink: 0;
  height: 32px;
  pointer-events: none;
  width: 100%;
}

.macos-empty-state {
  align-items: center;
  color: var(--lumina-text-secondary);
  display: flex;
  flex-direction: column;
  gap: 10px;
  justify-content: center;
  min-height: 240px;
  padding: 40px 16px;
  text-align: center;

  .empty-glyph {
    color: var(--lumina-text-tertiary);
    font-size: 32px;
    opacity: 0.6;
  }

  .empty-illustration {
    align-items: center;
    background: var(--lumina-control-bg);
    border-radius: 50%;
    color: var(--lumina-text-tertiary);
    display: flex;
    height: 52px;
    justify-content: center;
    width: 52px;

    svg {
      height: 26px;
      width: 26px;
    }
  }

  h3 {
    color: var(--lumina-text);
    font-size: 14px;
    margin: 0;
  }

  p {
    color: var(--lumina-text-tertiary);
    font-size: 12px;
    margin: 0;
    max-width: 320px;
  }

  .empty-actions {
    display: flex;
    gap: 8px;
    margin-top: 6px;
  }
}

.spin-glyph {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  100% {
    transform: rotate(360deg);
  }
}
</style>
