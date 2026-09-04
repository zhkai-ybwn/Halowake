<template>
  <div class="quota-card" :class="{ 'is-unhealthy': !quota.isHealthy || quota.errorMessage }">
    <!-- 卡片头部：图标 + 标题 + Plan 标签（纯净通透，无操作按钮挤压） -->
    <header class="card-header">
      <div class="header-main">
        <div class="provider-badge" :data-provider="quota.providerType">
          <Icon :icon="providerIcon" />
        </div>
        <div class="provider-info">
          <div class="title-row">
            <h3 class="account-name" :title="quota.name">{{ quota.name }}</h3>
            <span v-if="quota.plan" class="plan-tag">{{ quota.plan }}</span>
            <span
              v-if="quota.resetCredits && quota.resetCredits.availableCount > 0"
              class="reset-credits-tag"
              :title="formatResetCreditsTooltip(quota.resetCredits)"
            >
              <Icon icon="solar:restart-circle-linear" />
              {{ quota.resetCredits.availableCount }} {{ t('quota.resetCreditsUnit') }}{{ t('quota.resetCreditsShort') }}
            </span>
          </div>
          <span class="provider-type-label">{{ providerDisplayName }}</span>
        </div>
      </div>
    </header>

    <!-- 卡片主体：上下自适应对齐 -->
    <main class="card-body">
      <div class="card-content-top">
        <!-- 错误/离线警告状态 -->
        <div v-if="!quota.isHealthy || quota.errorMessage" class="error-banner">
          <Icon icon="solar:danger-triangle-linear" />
          <span>{{ quota.errorMessage || t('quota.connectionFailed') }}</span>
        </div>

        <!-- 1. 余额类型展示 (DeepSeek / OpenRouter / OpenAI) -->
        <div v-for="(item, idx) in balanceQuotas" :key="'bal-' + idx" class="balance-section">
          <div class="balance-main">
            <span class="currency-symbol">{{ item.currency === 'USD' ? '$' : '¥' }}</span>
            <span class="balance-value">{{ item.totalRemaining.toFixed(2) }}</span>
            <span class="currency-code">{{ item.currency }}</span>
          </div>

          <div v-if="item.toppedUp > 0 || item.granted > 0" class="balance-breakdown">
            <div v-if="item.toppedUp > 0" class="breakdown-item">
              <span class="breakdown-label">{{ t('quota.toppedUp') }}</span>
              <span class="breakdown-val">{{ item.toppedUp.toFixed(2) }}</span>
            </div>
            <div v-if="item.granted > 0" class="breakdown-item">
              <span class="breakdown-label">{{ t('quota.granted') }}</span>
              <span class="breakdown-val">{{ item.granted.toFixed(2) }}</span>
            </div>
          </div>
        </div>

        <!-- 2. 周期限额类型展示 (Codex 5h/Weekly, Gemini 等) -->
        <div v-if="rateLimitQuotas.length > 0" class="ratelimit-section">
          <div v-for="(item, idx) in rateLimitQuotas" :key="'rate-' + idx" class="ratelimit-row">
            <div class="ratelimit-header">
              <span class="rate-label">{{ item.periodLabel }}</span>
              <div class="rate-percent-group">
                <span class="rate-remaining" :class="getRateColorClass(item.usedPercent)">
                  {{ t('quota.remaining') }} {{ Math.max(0, 100 - item.usedPercent).toFixed(0) }}%
                </span>
                <span class="rate-used-sub">
                  ({{ item.usedPercent.toFixed(0) }}% {{ t('quota.used') }})
                </span>
              </div>
            </div>

            <div class="progress-bar-bg">
              <div
                class="progress-bar-fill"
                :class="getRateColorClass(item.usedPercent)"
                :style="{ width: Math.max(0, 100 - item.usedPercent) + '%' }"
              ></div>
            </div>

            <div v-if="item.resetsAt || item.resetsInSeconds" class="reset-time-hint">
              <Icon icon="solar:clock-circle-linear" />
              <span>{{ formatResetDetailed(item.resetsAt, item.resetsInSeconds) }}</span>
            </div>
          </div>

          <!-- 限额重置次数卡片 (Codex rate_limit_reset_credits) -->
          <div v-if="quota.resetCredits" class="reset-credits-panel">
            <div class="reset-credits-left">
              <div class="reset-icon-pill">
                <Icon icon="solar:restart-square-linear" />
              </div>
              <div class="reset-info">
                <div class="reset-title-row">
                  <span class="reset-label">{{ t('quota.resetCredits') }}</span>
                  <span
                    v-if="quota.resetCredits.nearestExpiresAt || quota.resetCredits.nearestExpiresInSeconds"
                    class="reset-expire-badge"
                  >
                    <Icon icon="solar:calendar-date-linear" />
                    {{ formatExpiryTag(quota.resetCredits.nearestExpiresAt, quota.resetCredits.nearestExpiresInSeconds) }}
                  </span>
                </div>
                <span class="reset-desc">
                  {{ formatResetCreditsSubtitle(quota.resetCredits) }}
                </span>
              </div>
            </div>
            <div class="reset-credits-count">
              <strong class="reset-number">{{ quota.resetCredits.availableCount }}</strong>
              <span class="reset-unit">{{ t('quota.resetCreditsUnit') }}</span>
            </div>
          </div>
        </div>

        <!-- 无 RateLimit 时的独立重置次数展示兜底 -->
        <div v-else-if="quota.resetCredits" class="reset-credits-panel">
          <div class="reset-credits-left">
            <div class="reset-icon-pill">
              <Icon icon="solar:restart-square-linear" />
            </div>
            <div class="reset-info">
              <div class="reset-title-row">
                <span class="reset-label">{{ t('quota.resetCredits') }}</span>
                <span
                  v-if="quota.resetCredits.nearestExpiresAt || quota.resetCredits.nearestExpiresInSeconds"
                  class="reset-expire-badge"
                >
                  <Icon icon="solar:calendar-date-linear" />
                  {{ formatExpiryTag(quota.resetCredits.nearestExpiresAt, quota.resetCredits.nearestExpiresInSeconds) }}
                </span>
              </div>
              <span class="reset-desc">
                {{ formatResetCreditsSubtitle(quota.resetCredits) }}
              </span>
            </div>
          </div>
          <div class="reset-credits-count">
            <strong class="reset-number">{{ quota.resetCredits.availableCount }}</strong>
            <span class="reset-unit">{{ t('quota.resetCreditsUnit') }}</span>
          </div>
        </div>

        <!-- 3. 点数/积分展示 (Prompt 积分 / Flow 积分 / Credits) -->
        <div v-if="creditsQuotas.length > 0" class="credits-container">
          <div v-for="(item, idx) in creditsQuotas" :key="'cred-' + idx" class="credit-pill">
            <span class="credit-pill-label">{{ item.label || t('quota.credits') }}</span>
            <div class="credit-pill-num-group">
              <span class="credit-pill-val">{{ item.remaining.toLocaleString() }}</span>
              <span v-if="item.total" class="credit-pill-total">/ {{ item.total.toLocaleString() }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- 4. Pace Projection (配速健康预测，对齐吸附到底部区域) -->
      <div v-if="quota.pace" class="pace-section" :class="'pace-' + quota.pace.level">
        <div class="pace-header">
          <span class="pace-dot"></span>
          <span class="pace-level-text">{{ getPaceLabel(quota.pace.level) }}</span>
          <span v-if="quota.pace.projectedUsagePercent" class="projected-tag">
            {{ t('quota.projected') }} {{ quota.pace.projectedUsagePercent }}%
          </span>
        </div>
        <p class="pace-desc">{{ quota.pace.message }}</p>
      </div>
    </main>

    <!-- 卡片底部：左侧更新时间，右侧操作按钮栏 -->
    <footer class="card-footer">
      <span class="update-time">
        {{ t('quota.lastChecked') }} {{ formatRelativeTime(quota.lastUpdated) }}
      </span>

      <div class="footer-actions">
        <button
          class="action-btn"
          type="button"
          :title="t('quota.editAccount')"
          @click="emit('edit', quota.accountId)"
        >
          <Icon icon="solar:pen-2-linear" />
        </button>
        <button
          v-if="quota.officialDashboardUrl"
          class="action-btn"
          type="button"
          :title="t('quota.openDashboard')"
          @click="handleOpenExternal(quota.officialDashboardUrl)"
        >
          <Icon icon="solar:link-linear" />
        </button>
        <button
          class="action-btn"
          type="button"
          :class="{ 'is-refreshing': refreshing }"
          :title="t('quota.refreshSingle')"
          @click="emit('refresh', quota.accountId)"
        >
          <Icon icon="solar:restart-linear" />
        </button>
      </div>
    </footer>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { openExternalUrl } from '@/services/app-service'
import type { ProviderQuota, QuotaKind, PaceLevel } from '@/services/quota/quota-service'

const props = defineProps<{
  quota: ProviderQuota
  refreshing?: boolean
}>()

const emit = defineEmits<{
  (e: 'refresh', accountId: string): void
  (e: 'edit', accountId: string): void
}>()

const { t } = useI18n({ useScope: 'global' })

const providerDisplayName = computed(() => {
  switch (props.quota.providerType) {
    case 'codex':
      return 'OpenAI / Codex'
    case 'deepseek':
      return 'DeepSeek'
    case 'openrouter':
      return 'OpenRouter'
    case 'gemini':
      return 'Google Gemini / Antigravity'
    default:
      return 'OpenAI-Compatible'
  }
})

const providerIcon = computed(() => {
  switch (props.quota.providerType) {
    case 'codex':
      return 'solar:code-square-linear'
    case 'deepseek':
      return 'solar:bolt-circle-linear'
    case 'openrouter':
      return 'solar:routing-2-linear'
    case 'gemini':
      return 'solar:stars-minimalistic-linear'
    default:
      return 'solar:server-linear'
  }
})

const balanceQuotas = computed(() => {
  return props.quota.quotas.filter((q): q is Extract<QuotaKind, { type: 'balance' }> => q.type === 'balance')
})

const rateLimitQuotas = computed(() => {
  return props.quota.quotas.filter((q): q is Extract<QuotaKind, { type: 'rateLimit' }> => q.type === 'rateLimit')
})

const creditsQuotas = computed(() => {
  return props.quota.quotas.filter((q): q is Extract<QuotaKind, { type: 'credits' }> => q.type === 'credits')
})

function getRateColorClass(usedPercent: number): string {
  const remaining = 100 - usedPercent
  if (remaining <= 10) return 'is-danger'
  if (remaining <= 25) return 'is-warning'
  return 'is-healthy'
}

function getPaceLabel(level: PaceLevel): string {
  switch (level) {
    case 'onPace':
      return t('quota.paceHealthy')
    case 'tight':
      return t('quota.paceTight')
    case 'overPace':
      return t('quota.paceOver')
  }
}

function formatResetDetailed(resetsAt?: number, resetsInSeconds?: number): string {
  if (!resetsAt && !resetsInSeconds) return ''

  let timeDesc = ''
  if (resetsAt) {
    const d = new Date(resetsAt * 1000)
    const month = d.getMonth() + 1
    const date = d.getDate()
    const hours = String(d.getHours()).padStart(2, '0')
    const mins = String(d.getMinutes()).padStart(2, '0')
    timeDesc = `${month}月${date}日 ${hours}:${mins}`
  }

  let countdown = ''
  if (resetsInSeconds && resetsInSeconds > 0) {
    const d = Math.floor(resetsInSeconds / 86400)
    const h = Math.floor((resetsInSeconds % 86400) / 3600)
    const m = Math.floor((resetsInSeconds % 3600) / 60)

    if (d > 0) {
      countdown = ` (${d}天后重置)`
    } else if (h > 0) {
      countdown = ` (${h}h ${m}m 后重置)`
    } else {
      countdown = ` (${m}m 后重置)`
    }
  }

  return `${t('quota.resetAt')}: ${timeDesc}${countdown}`
}

function formatExpiryTag(expiresAt?: number, expiresInSeconds?: number): string {
  if (!expiresAt && !expiresInSeconds) return ''
  let timeStr = ''
  if (expiresAt) {
    const d = new Date(expiresAt * 1000)
    const month = d.getMonth() + 1
    const date = d.getDate()
    timeStr = `${month}月${date}日`
  }
  let remaining = ''
  if (expiresInSeconds && expiresInSeconds > 0) {
    const days = Math.floor(expiresInSeconds / 86400)
    const hours = Math.floor((expiresInSeconds % 86400) / 3600)
    if (days > 0) {
      remaining = `${days}天后`
    } else if (hours > 0) {
      remaining = `${hours}h后`
    } else {
      remaining = '即将到期'
    }
  }
  if (timeStr && remaining) {
    return `${timeStr} · ${remaining}`
  }
  return timeStr || remaining
}

function formatResetCreditsSubtitle(credits: NonNullable<ProviderQuota['resetCredits']>): string {
  if (credits.nearestExpiresAt || credits.nearestExpiresInSeconds) {
    let timeDesc = ''
    if (credits.nearestExpiresAt) {
      const d = new Date(credits.nearestExpiresAt * 1000)
      const month = d.getMonth() + 1
      const date = d.getDate()
      const hours = String(d.getHours()).padStart(2, '0')
      const mins = String(d.getMinutes()).padStart(2, '0')
      timeDesc = `${month}月${date}日 ${hours}:${mins}`
    }

    let countdown = ''
    if (credits.nearestExpiresInSeconds && credits.nearestExpiresInSeconds > 0) {
      const d = Math.floor(credits.nearestExpiresInSeconds / 86400)
      const h = Math.floor((credits.nearestExpiresInSeconds % 86400) / 3600)
      if (d > 0) {
        countdown = ` (${d}天后过期)`
      } else if (h > 0) {
        countdown = ` (${h}小时后过期)`
      } else {
        countdown = ` (即将过期)`
      }
    }

    return `${t('quota.resetCreditsExpiresAt', { time: timeDesc })}${countdown}`
  }

  if (credits.applicableAvailableCount && credits.applicableAvailableCount > 0) {
    return t('quota.resetCreditsApplicableDesc')
  }

  return t('quota.resetCreditsAvailableDesc', { count: credits.availableCount })
}

function formatResetCreditsTooltip(credits: NonNullable<ProviderQuota['resetCredits']>): string {
  const base = credits.applicableAvailableCount
    ? t('quota.resetCreditsApplicable')
    : t('quota.resetCreditsAvailable', { count: credits.availableCount })
  if (credits.nearestExpiresAt) {
    const d = new Date(credits.nearestExpiresAt * 1000)
    const month = d.getMonth() + 1
    const date = d.getDate()
    const hours = String(d.getHours()).padStart(2, '0')
    const mins = String(d.getMinutes()).padStart(2, '0')
    return `${base} (${t('quota.resetCreditsExpiresTag', { time: `${month}月${date}日 ${hours}:${mins}` })})`
  }
  return base
}

function formatRelativeTime(timestamp: number): string {
  const diff = Date.now() - timestamp
  const mins = Math.floor(diff / 60000)
  if (mins < 1) return t('quota.justNow')
  if (mins < 60) return `${mins} ${t('quota.minsAgo')}`
  const hours = Math.floor(mins / 60)
  return `${hours} ${t('quota.hoursAgo')}`
}

async function handleOpenExternal(url: string) {
  try {
    await openExternalUrl(url)
  } catch {
    window.open(url, '_blank')
  }
}
</script>

<style scoped lang="scss">
.quota-card {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 290px;
  background: var(--lumina-surface-elevated);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-lg);
  padding: 16px;
  box-shadow: var(--lumina-shadow-sm);
  transition: all var(--lumina-duration-fast) var(--lumina-ease-out);
  position: relative;

  &:hover {
    border-color: var(--lumina-separator-strong);
    box-shadow: var(--lumina-shadow-md);
  }

  &.is-unhealthy {
    border-color: rgba(220, 38, 38, 0.35);
  }
}

.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-bottom: 12px;
  border-bottom: 0.5px solid var(--lumina-separator);
}

.header-main {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
  flex: 1;
}

.provider-badge {
  width: 32px;
  height: 32px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;

  svg {
    width: 18px;
    height: 18px;
  }

  &[data-provider='deepseek'] {
    background: rgba(2, 132, 199, 0.12);
    color: #0284c7;
  }
  &[data-provider='codex'] {
    background: rgba(16, 185, 129, 0.12);
    color: #10b981;
  }
  &[data-provider='openrouter'] {
    background: rgba(139, 92, 246, 0.12);
    color: #8b5cf6;
  }
  &[data-provider='gemini'] {
    background: rgba(245, 158, 11, 0.12);
    color: #f59e0b;
  }
}

.provider-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
  flex: 1;
}

.title-row {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}

.account-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--lumina-text);
  margin: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.plan-tag {
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 4px;
  background: var(--lumina-control-bg);
  color: var(--lumina-text-secondary);
  border: 0.5px solid var(--lumina-separator);
  white-space: nowrap;
}

.reset-credits-tag {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  font-size: 10px;
  font-weight: 500;
  padding: 1px 6px;
  border-radius: 4px;
  background: rgba(16, 185, 129, 0.1);
  color: #10b981;
  border: 0.5px solid rgba(16, 185, 129, 0.25);
  white-space: nowrap;

  svg {
    width: 11px;
    height: 11px;
  }
}

.provider-type-label {
  font-size: 11px;
  color: var(--lumina-text-tertiary);
}

.card-body {
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  gap: 12px;
  flex: 1;
  padding: 12px 0 6px;
}

.card-content-top {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.error-banner {
  display: flex;
  align-items: flex-start;
  gap: 6px;
  padding: 8px 10px;
  border-radius: 6px;
  background: rgba(220, 38, 38, 0.08);
  border: 0.5px solid rgba(220, 38, 38, 0.2);
  color: var(--lumina-danger);
  font-size: 11px;
  line-height: 1.4;

  svg {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
    margin-top: 1px;
  }
}

.balance-section {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.balance-main {
  display: flex;
  align-items: baseline;
  gap: 4px;
}

.currency-symbol {
  font-size: 15px;
  font-weight: 600;
  color: var(--lumina-text-secondary);
}

.balance-value {
  font-size: 24px;
  font-weight: 700;
  font-family: var(--lumina-font-mono);
  color: var(--lumina-text);
  line-height: 1;
}

.currency-code {
  font-size: 11px;
  font-weight: 600;
  color: var(--lumina-text-tertiary);
  margin-left: 4px;
}

.balance-breakdown {
  display: flex;
  gap: 12px;
  font-size: 11px;
  color: var(--lumina-text-tertiary);
}

.breakdown-item {
  display: flex;
  gap: 4px;
}

.ratelimit-section {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.ratelimit-row {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.ratelimit-header {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  font-size: 11px;
}

.rate-label {
  color: var(--lumina-text-secondary);
  font-weight: 500;
}

.rate-percent-group {
  display: flex;
  align-items: baseline;
  gap: 4px;
}

.rate-remaining {
  font-weight: 600;

  &.is-healthy {
    color: #10b981;
  }
  &.is-warning {
    color: #f59e0b;
  }
  &.is-danger {
    color: #ef4444;
  }
}

.rate-used-sub {
  color: var(--lumina-text-tertiary);
  font-size: 10px;
}

.progress-bar-bg {
  width: 100%;
  height: 6px;
  background: var(--lumina-control-bg);
  border-radius: 3px;
  overflow: hidden;
}

.progress-bar-fill {
  height: 100%;
  border-radius: 3px;
  transition: width var(--lumina-duration-fast);

  &.is-healthy {
    background: #10b981;
  }
  &.is-warning {
    background: #f59e0b;
  }
  &.is-danger {
    background: #ef4444;
  }
}

.reset-time-hint {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 10px;
  color: var(--lumina-text-tertiary);

  svg {
    width: 11px;
    height: 11px;
  }
}

.reset-credits-panel {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 10px;
  border-radius: 6px;
  background: rgba(16, 185, 129, 0.06);
  border: 0.5px solid rgba(16, 185, 129, 0.2);
  margin-top: 2px;
}

.reset-credits-left {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.reset-icon-pill {
  width: 24px;
  height: 24px;
  border-radius: 5px;
  background: rgba(16, 185, 129, 0.12);
  color: #10b981;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;

  svg {
    width: 14px;
    height: 14px;
  }
}

.reset-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.reset-title-row {
  display: flex;
  align-items: center;
  gap: 6px;
}

.reset-label {
  font-size: 11px;
  font-weight: 600;
  color: var(--lumina-text);
  line-height: 1.2;
}

.reset-expire-badge {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  font-size: 9.5px;
  font-weight: 500;
  padding: 0.5px 5px;
  border-radius: 3px;
  background: rgba(16, 185, 129, 0.12);
  color: #059669;
  border: 0.5px solid rgba(16, 185, 129, 0.25);
  white-space: nowrap;

  svg {
    width: 10px;
    height: 10px;
  }
}

.reset-desc {
  font-size: 10px;
  color: var(--lumina-text-tertiary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.reset-credits-count {
  display: flex;
  align-items: baseline;
  gap: 2px;
  flex-shrink: 0;
  margin-left: 8px;
}

.reset-number {
  font-size: 15px;
  font-weight: 700;
  font-family: var(--lumina-font-mono);
  color: #10b981;
  line-height: 1;
}

.reset-unit {
  font-size: 10px;
  font-weight: 500;
  color: var(--lumina-text-secondary);
}

.credits-container {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.credit-pill {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  background: var(--lumina-surface-2);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-sm);
  padding: 4px 10px;
  font-size: 11.5px;
}

.credit-pill-label {
  color: var(--lumina-text-secondary);
  font-weight: 500;
}

.credit-pill-num-group {
  display: inline-flex;
  align-items: baseline;
  gap: 2px;
}

.credit-pill-val {
  font-family: var(--lumina-font-mono);
  font-weight: 700;
  color: var(--lumina-primary);
  font-size: 13px;
}

.credit-pill-total {
  font-family: var(--lumina-font-mono);
  color: var(--lumina-text-tertiary);
  font-size: 10.5px;
}

.pace-section {
  padding: 8px 10px;
  border-radius: 6px;
  display: flex;
  flex-direction: column;
  gap: 3px;
  font-size: 11px;
  margin-top: 4px;

  &.pace-onPace {
    background: rgba(16, 185, 129, 0.08);
    border: 0.5px solid rgba(16, 185, 129, 0.2);

    .pace-dot {
      background: #10b981;
    }
    .pace-level-text {
      color: #10b981;
    }
  }

  &.pace-tight {
    background: rgba(245, 158, 11, 0.08);
    border: 0.5px solid rgba(245, 158, 11, 0.2);

    .pace-dot {
      background: #f59e0b;
    }
    .pace-level-text {
      color: #f59e0b;
    }
  }

  &.pace-overPace {
    background: rgba(239, 68, 68, 0.08);
    border: 0.5px solid rgba(239, 68, 68, 0.2);

    .pace-dot {
      background: #ef4444;
    }
    .pace-level-text {
      color: #ef4444;
    }
  }
}

.pace-header {
  display: flex;
  align-items: center;
  gap: 6px;
}

.pace-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
}

.pace-level-text {
  font-weight: 600;
}

.projected-tag {
  margin-left: auto;
  font-size: 10px;
  color: var(--lumina-text-tertiary);
}

.pace-desc {
  margin: 0;
  font-size: 10.5px;
  color: var(--lumina-text-secondary);
  line-height: 1.35;
}

.card-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-top: 10px;
  border-top: 0.5px solid var(--lumina-separator);
  margin-top: 4px;
}

.update-time {
  font-size: 10px;
  color: var(--lumina-text-tertiary);
}

.footer-actions {
  display: flex;
  align-items: center;
  gap: 4px;
}

.action-btn {
  width: 26px;
  height: 26px;
  border-radius: 6px;
  background: transparent;
  border: 0;
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all var(--lumina-duration-fast);

  &:hover {
    background: var(--lumina-control-hover);
    color: var(--lumina-text);
  }

  svg {
    width: 14px;
    height: 14px;
  }

  &.is-refreshing svg {
    animation: spin 1s linear infinite;
  }
}

@keyframes spin {
  100% {
    transform: rotate(360deg);
  }
}
</style>
