<template>
  <div class="topbar-quota-pill-host" @mousedown.stop @dblclick.stop>
    <NPopover
      v-model:show="popoverOpen"
      trigger="click"
      placement="bottom-end"
      :show-arrow="false"
      raw
      class="topbar-quota-popover"
    >
      <template #trigger>
        <button
          type="button"
          class="topbar-quota-pill"
          :class="{
            active: popoverOpen,
            'has-warning': quotaStore.statusTone === 'warning',
            'has-danger': quotaStore.statusTone === 'danger',
          }"
          :title="statusTooltip"
        >
          <span class="pill-dot" :class="quotaStore.statusDotClass"></span>
          <Icon icon="solar:wallet-money-linear" class="pill-icon" />
          <span class="pill-text">{{ pillLabel }}</span>
        </button>
      </template>

      <!-- Popover Content Card -->
      <section class="quota-popover-card" role="dialog" :aria-label="t('workbench.aiQuota')">
        <header class="quota-popover-header">
          <div class="header-title">
            <Icon icon="solar:wallet-money-linear" />
            <strong>{{ t('workbench.aiQuota') }}</strong>
          </div>
          <div class="header-tools">
            <div class="quota-legend" :title="t('aiQuota.legendTooltip', '数值格式：5小时限额剩余 / 每周限额剩余')">
              <span class="legend-pill">
                <span class="legend-text">5h</span>
                <span class="legend-slash">/</span>
                <span class="legend-text">周</span>
              </span>
            </div>
            <button
              type="button"
              class="refresh-btn"
              :class="{ spinning: refreshing }"
              :disabled="refreshing"
              :title="t('aiQuota.refresh')"
              @click="handleRefresh"
            >
              <Icon icon="solar:refresh-linear" />
            </button>
          </div>
        </header>

        <!-- Provider Mini Grid (支持 10+ 厂商平滑滚动) -->
        <div class="quota-provider-grid">
          <div v-if="loading && !quotas.length" class="quota-mini-loading">
            <span>{{ t('common.loading') }}</span>
          </div>
          <div
            v-for="provider in displayedQuotas"
            :key="provider.id"
            class="provider-pill-cell"
            :class="[getPercentTone(provider), { unhealthy: !provider.isHealthy, 'span-full': displayedQuotas.length === 1 }]"
            :title="getProviderTooltip(provider)"
          >
            <!-- 顶部金额 / 计划 -->
            <div class="cell-top-val">
              {{ getProviderAmount(provider) }}
            </div>
            <!-- 底下是 Logo + 限额数值（5h 与 周 一一对应，通过顶部图例说明） -->
            <div class="cell-bottom-row">
              <span class="provider-logo-tag" :data-provider="provider.providerType">
                <ProviderBrandLogo :provider="provider.providerType" :size="15" />
              </span>
              <div v-if="getRatePairs(provider).length > 0" class="rate-pairs-group">
                <span
                  v-for="(pair, idx) in getRatePairs(provider)"
                  :key="idx"
                  class="rate-pair-badge"
                  :title="pair.tooltip"
                >
                  <span class="pair-val pair-5h" :class="pair.fiveHourTone">{{ pair.fiveHourPercent || '-' }}</span>
                  <span class="pair-slash">/</span>
                  <span class="pair-val pair-week" :class="pair.weeklyTone">{{ pair.weeklyPercent || '-' }}</span>
                </span>
              </div>
            </div>
          </div>
        </div>

        <!-- Footer Link with Dynamic Platform Shortcut -->
        <footer class="quota-popover-footer">
          <button type="button" class="full-view-link" @click="goToFullQuotaView">
            <span>{{ t('aiQuota.viewFullCenter') }}</span>
            <kbd>{{ isMac ? '⌘5' : 'Alt+5' }}</kbd>
          </button>
        </footer>
      </section>
    </NPopover>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { NPopover } from 'naive-ui'
import { Icon } from '@iconify/vue'
import { useI18n } from 'vue-i18n'
import ProviderBrandLogo from './ProviderBrandLogo.vue'
import { useQuotaStore } from '@/stores/quota'
import type { ProviderQuota, QuotaKind } from '@/services/quota/quota-service'
import { isMacPlatform } from '@/utils/platform-shortcuts'

const router = useRouter()
const { t } = useI18n({ useScope: 'global' })
const isMac = isMacPlatform
const quotaStore = useQuotaStore()

const popoverOpen = ref(false)
const quotas = computed(() => quotaStore.quotas)
const loading = computed(() => quotaStore.loading)
const refreshing = computed(() => quotaStore.refreshing)

// 展示所有已配置/已发现的厂商，支持 10+ 厂商在容器内平滑滚动
const displayedQuotas = computed(() => quotas.value)

const statusTooltip = computed(() => {
  if (quotaStore.statusTone === 'danger') {
    return `${t('workbench.aiQuota')} (${t('quota.statusDanger', '额度即将耗尽')})`
  }
  if (quotaStore.statusTone === 'warning') {
    return `${t('workbench.aiQuota')} (${t('quota.statusWarning', '额度偏紧')})`
  }
  return `${t('workbench.aiQuota')} (${t('quota.statusHealthy', '运行正常')})`
})

const pillLabel = computed(() => {
  return t('workbench.aiQuota')
})

function getProviderAmount(provider: ProviderQuota): string {
  if (!provider.isHealthy) return t('aiQuota.statusUnhealthy')
  const balanceQuota = provider.quotas.find(q => q.type === 'balance')
  if (balanceQuota && balanceQuota.type === 'balance') {
    const sym = balanceQuota.currency === 'USD' ? '$' : '¥'
    return `${sym}${balanceQuota.totalRemaining.toFixed(2)}`
  }
  const creditsQuotas = provider.quotas.filter((q): q is Extract<QuotaKind, { type: 'credits' }> => q.type === 'credits')
  if (creditsQuotas.length > 0) {
    const firstUnit = creditsQuotas[0].unit?.trim() || t('quota.creditsUnit')
    const sameUnit = creditsQuotas.every(q => (q.unit?.trim() || t('quota.creditsUnit')) === firstUnit)
    if (sameUnit) {
      const totalRemaining = creditsQuotas.reduce((acc, q) => acc + q.remaining, 0)
      const rounded = Math.round(totalRemaining * 100) / 100
      return `${rounded.toLocaleString()} ${firstUnit}`
    }
    const first = Math.round(creditsQuotas[0].remaining * 100) / 100
    return `${first.toLocaleString()} ${firstUnit} +${creditsQuotas.length - 1}`
  }
  if (provider.plan) {
    return provider.plan.replace(/ChatGPT\s*/i, '').trim() || provider.plan
  }
  return 'Active'
}

interface RatePair {
  key: string
  label?: string
  fiveHourPercent?: string
  fiveHourTone?: 'healthy' | 'warning' | 'danger'
  weeklyPercent?: string
  weeklyTone?: 'healthy' | 'warning' | 'danger'
  tooltip: string
}

function getRatePairs(provider: ProviderQuota): RatePair[] {
  if (!provider.isHealthy) return []

  const rateLimits = provider.quotas.filter(
    (q): q is Extract<QuotaKind, { type: 'rateLimit' }> => q.type === 'rateLimit'
  )

  // 纯余额服务商（如 DeepSeek、OpenRouter 等）不展示百分比
  if (rateLimits.length === 0) {
    return []
  }

  // 按模型/子组聚合，保证同一组的 5h 与 周 一一对应
  const groups = new Map<
    string,
    {
      label: string
      fiveHour?: Extract<QuotaKind, { type: 'rateLimit' }>
      weekly?: Extract<QuotaKind, { type: 'rateLimit' }>
      other?: Extract<QuotaKind, { type: 'rateLimit' }>
    }
  >()

  for (const rl of rateLimits) {
    const rawLabel = rl.periodLabel || ''
    // 剥离时间窗口后缀，提取组名（如 "Gemini 模型", "Claude 和 GPT 模型"）
    const groupName =
      rawLabel
        .replace(/\((5\s*小时限额|每周限额|5h|weekly|周期限额)\)/gi, '')
        .replace(/5\s*小时限额|每周限额|5h|weekly|周期限额/gi, '')
        .trim() || 'default'

    if (!groups.has(groupName)) {
      groups.set(groupName, { label: groupName === 'default' ? '' : groupName })
    }
    const g = groups.get(groupName)!

    const pl = rawLabel.toLowerCase()
    if (pl.includes('5') && (pl.includes('h') || pl.includes('小时') || pl.includes('hour'))) {
      g.fiveHour = rl
    } else if (pl.includes('周') || pl.includes('week')) {
      g.weekly = rl
    } else {
      g.other = rl
    }
  }

  const pairs: RatePair[] = []

  const calcTone = (used: number): 'healthy' | 'warning' | 'danger' => {
    const rem = Math.max(0, 100 - used)
    if (rem <= 10) return 'danger'
    if (rem <= 30) return 'warning'
    return 'healthy'
  }

  for (const [groupKey, g] of groups.entries()) {
    const fiveHourPercent = g.fiveHour
      ? `${Math.round(Math.max(0, 100 - g.fiveHour.usedPercent))}%`
      : g.other
        ? `${Math.round(Math.max(0, 100 - g.other.usedPercent))}%`
        : undefined
    const fiveHourTone = g.fiveHour
      ? calcTone(g.fiveHour.usedPercent)
      : g.other
        ? calcTone(g.other.usedPercent)
        : undefined

    const weeklyPercent = g.weekly
      ? `${Math.round(Math.max(0, 100 - g.weekly.usedPercent))}%`
      : undefined
    const weeklyTone = g.weekly ? calcTone(g.weekly.usedPercent) : undefined

    if (!fiveHourPercent && !weeklyPercent) continue

    const tooltipParts: string[] = []
    if (g.label) {
      tooltipParts.push(g.label)
    }
    if (g.fiveHour) {
      const rem = Math.round(Math.max(0, 100 - g.fiveHour.usedPercent))
      tooltipParts.push(`5h: 剩余 ${rem}%`)
    } else if (g.other) {
      const rem = Math.round(Math.max(0, 100 - g.other.usedPercent))
      tooltipParts.push(`${g.other.periodLabel}: 剩余 ${rem}%`)
    }
    if (g.weekly) {
      const rem = Math.round(Math.max(0, 100 - g.weekly.usedPercent))
      tooltipParts.push(`周: 剩余 ${rem}%`)
    }

    pairs.push({
      key: groupKey,
      label: g.label,
      fiveHourPercent,
      fiveHourTone,
      weeklyPercent,
      weeklyTone,
      tooltip: tooltipParts.join(' · '),
    })
  }

  return pairs
}

function getPercentTone(provider: ProviderQuota): 'healthy' | 'warning' | 'danger' {
  if (!provider.isHealthy) return 'danger'
  if (provider.pace?.level === 'overPace') return 'danger'
  if (provider.pace?.level === 'tight') return 'warning'

  const rateLimits = provider.quotas.filter(
    (q): q is Extract<QuotaKind, { type: 'rateLimit' }> => q.type === 'rateLimit'
  )
  for (const rl of rateLimits) {
    const rem = Math.max(0, 100 - rl.usedPercent)
    if (rem <= 10) return 'danger'
    if (rem <= 30) return 'warning'
  }
  return 'healthy'
}

function getProviderTooltip(provider: ProviderQuota): string {
  const parts = [provider.name]
  if (provider.pace?.message) {
    parts.push(provider.pace.message)
  }
  return parts.join(' · ')
}

watch(popoverOpen, (open) => {
  if (open) {
    void quotaStore.ensureFresh(10000)
  }
})

async function handleRefresh() {
  try {
    await quotaStore.fetchAll(true)
  } catch (err) {
    console.warn('Failed to refresh quotas from popover:', err)
  }
}

function goToFullQuotaView() {
  popoverOpen.value = false
  void router.push({ name: 'ai-quota' })
}

onMounted(() => {
  void quotaStore.ensureFresh(30000)
})
</script>

<style scoped lang="scss">
.topbar-quota-pill-host {
  align-items: center;
  display: flex;
}

.topbar-quota-pill {
  align-items: center;
  background: var(--lumina-control-bg);
  border: 0.5px solid var(--lumina-separator);
  border-radius: 13px;
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: flex;
  font-size: 11.5px;
  gap: 6px;
  height: 26px;
  padding: 0 9px;
  transition: all var(--lumina-duration-fast) var(--lumina-ease-out);

  &:hover,
  &.active {
    background: var(--lumina-control-hover);
    color: var(--lumina-text);
  }

  &.has-warning {
    border-color: color-mix(in srgb, var(--lumina-warning) 40%, var(--lumina-separator));
  }

  &.has-danger {
    border-color: color-mix(in srgb, var(--lumina-danger) 40%, var(--lumina-separator));
  }
}

.pill-dot {
  border-radius: 50%;
  flex: 0 0 6px;
  height: 6px;
  width: 6px;

  &.healthy {
    background: #34c759;
    box-shadow: 0 0 6px rgba(52, 199, 89, 0.4);
  }

  &.warning {
    background: #ff9500;
    box-shadow: 0 0 6px rgba(255, 149, 0, 0.4);
  }

  &.danger {
    background: #ff3b30;
    box-shadow: 0 0 6px rgba(255, 59, 48, 0.4);
  }

  &.idle {
    background: var(--lumina-text-tertiary);
  }
}

.pill-icon {
  font-size: 13px;
}

.pill-text {
  font-family: var(--lumina-font-mono, monospace);
  font-weight: 550;
  max-width: 90px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* Popover Card */
.quota-popover-card {
  background: var(--lumina-surface-elevated);
  border: 0.5px solid var(--lumina-separator-strong);
  border-radius: var(--lumina-radius-lg);
  box-shadow: var(--lumina-shadow-lg);
  color: var(--lumina-text);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  padding: 12px 14px;
  width: 340px;
  backdrop-filter: var(--lumina-vibrancy);
}

.quota-popover-header {
  align-items: center;
  border-bottom: 0.5px solid var(--lumina-separator);
  display: flex;
  justify-content: space-between;
  padding-bottom: 8px;
}

.header-title {
  align-items: center;
  color: var(--lumina-text);
  display: flex;
  font-size: 13px;
  gap: 6px;

  strong {
    font-weight: 600;
  }
}

.header-tools {
  align-items: center;
  display: flex;
  gap: 7px;
}

.quota-legend {
  align-items: center;
  cursor: help;
  display: flex;
}

.legend-pill {
  align-items: center;
  background: var(--lumina-control-bg);
  border: 0.5px solid var(--lumina-separator);
  border-radius: 4px;
  display: inline-flex;
  font-family: var(--lumina-font-mono, monospace);
  font-size: 10px;
  gap: 2px;
  line-height: 1;
  padding: 2.5px 5.5px;

  .legend-text {
    color: var(--lumina-text-secondary);
    font-weight: 550;
  }

  .legend-slash {
    color: var(--lumina-text-tertiary);
    font-size: 9px;
    opacity: 0.7;
  }
}

.refresh-btn {
  align-items: center;
  background: transparent;
  border: 0;
  border-radius: var(--lumina-radius-sm);
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: flex;
  font-size: 13px;
  height: 22px;
  justify-content: center;
  transition: all 0.15s ease;
  width: 22px;

  &:hover {
    background: var(--lumina-control-hover);
    color: var(--lumina-text);
  }

  &.spinning svg {
    animation: spin 0.8s linear infinite;
  }
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

/* Provider Grid (支持 10+ 厂商在 310px 内部平滑滚动) */
.quota-provider-grid {
  display: grid;
  gap: 8px;
  grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
  margin: 10px 0;
  max-height: 310px;
  overflow-y: auto;
  overflow-x: hidden;
  padding-right: 3px;

  /* Custom slim scrollbar */
  scrollbar-width: thin;
  scrollbar-color: var(--lumina-separator-strong) transparent;

  &::-webkit-scrollbar {
    width: 4px;
  }

  &::-webkit-scrollbar-thumb {
    background: var(--lumina-separator-strong);
    border-radius: 4px;
  }
}

.quota-mini-loading {
  color: var(--lumina-text-tertiary);
  font-size: 11px;
  grid-column: 1 / -1;
  padding: 14px;
  text-align: center;
}

.provider-pill-cell {
  align-items: stretch;
  background: var(--lumina-surface-2);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-sm);
  cursor: default;
  display: flex;
  flex-direction: column;
  gap: 6px;
  justify-content: space-between;
  padding: 7px 8px;
  transition: all 0.15s ease;

  &:hover {
    background: var(--lumina-control-hover);
    border-color: var(--lumina-separator-strong);
    transform: translateY(-1px);
  }

  &.unhealthy {
    border-color: color-mix(in srgb, var(--lumina-danger) 40%, var(--lumina-separator));
  }

  &.span-full {
    grid-column: 1 / -1;
  }
}

.cell-top-val {
  color: var(--lumina-text);
  font-family: var(--lumina-font-mono, monospace);
  font-size: 12.5px;
  font-weight: 650;
  line-height: 1.2;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.cell-bottom-row {
  align-items: center;
  display: flex;
  gap: 6px;
  justify-content: space-between;
}

.provider-logo-tag {
  align-items: center;
  border-radius: 4px;
  display: flex;
  height: 20px;
  justify-content: center;
  width: 20px;

  :deep(.brand-svg),
  svg {
    height: 14px;
    width: 14px;
  }

  &[data-provider='deepseek'] {
    background: rgba(77, 107, 254, 0.12);
  }

  &[data-provider='codex'],
  &[data-provider='openai'] {
    background: rgba(16, 163, 127, 0.12);
  }

  &[data-provider='openrouter'] {
    background: rgba(100, 102, 233, 0.12);
  }

  &[data-provider='gemini'] {
    background: rgba(49, 134, 255, 0.12);
  }

  &[data-provider='claude'] {
    background: rgba(217, 119, 87, 0.12);
  }

  &[data-provider='opencode'] {
    background: rgba(14, 165, 233, 0.12);
  }

  &[data-provider='workbuddy'] {
    background: rgba(0, 102, 255, 0.12);
  }

  &[data-provider='siliconflow'] {
    background: rgba(124, 58, 237, 0.12);
  }

  &[data-provider='moonshot'] {
    background: rgba(99, 102, 241, 0.12);
  }

  &[data-provider='zhipu'] {
    background: rgba(48, 98, 249, 0.12);
  }

  &[data-provider='qwen'] {
    background: rgba(97, 92, 237, 0.12);
  }

  &[data-provider='minimax'] {
    background: rgba(255, 75, 75, 0.12);
  }
}

.rate-pairs-group {
  align-items: center;
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  justify-content: flex-end;
}

.rate-pair-badge {
  align-items: center;
  background: var(--lumina-control-bg);
  border: 0.5px solid var(--lumina-separator);
  border-radius: 3.5px;
  cursor: default;
  display: inline-flex;
  font-family: var(--lumina-font-mono, monospace);
  font-size: 9.5px;
  line-height: 1.2;
  padding: 1.5px 4px;
  white-space: nowrap;

  .pair-val {
    font-weight: 650;

    &.healthy {
      color: #10b981;
    }
    &.warning {
      color: #f59e0b;
    }
    &.danger {
      color: #ef4444;
    }
    &.empty {
      color: var(--lumina-text-tertiary);
      font-weight: 400;
    }
  }

  .pair-slash {
    color: var(--lumina-text-tertiary);
    font-size: 8.5px;
    margin: 0 1.5px;
    opacity: 0.7;
  }
}

.quota-popover-footer {
  border-top: 0.5px solid var(--lumina-separator);
  padding-top: 8px;
}

.full-view-link {
  align-items: center;
  background: transparent;
  border: 0;
  border-radius: var(--lumina-radius-sm);
  color: var(--lumina-primary);
  cursor: pointer;
  display: flex;
  font-size: 11.5px;
  font-weight: 550;
  justify-content: space-between;
  padding: 4px 6px;
  width: 100%;

  &:hover {
    background: var(--lumina-primary-soft);
  }

  kbd {
    background: color-mix(in srgb, var(--lumina-primary) 12%, transparent);
    border: 0.5px solid color-mix(in srgb, var(--lumina-primary) 25%, transparent);
    border-radius: 3px;
    color: var(--lumina-primary);
    font-family: inherit;
    font-size: 10px;
    padding: 0 4px;
  }
}
</style>
