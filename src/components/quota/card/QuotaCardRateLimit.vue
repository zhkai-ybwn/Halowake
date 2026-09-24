<template>
  <div v-if="rateLimitQuotas.length > 0" class="ratelimit-section">
    <div v-for="(item, idx) in rateLimitQuotas" :key="'rate-' + idx" class="ratelimit-row">
      <div class="ratelimit-header">
        <span class="rate-label">{{ item.periodLabel }}</span>
        <div class="rate-percent-group">
          <span class="rate-remaining-badge" :class="getRateColorClass(item.usedPercent)">
            剩余 {{ Math.max(0, 100 - item.usedPercent).toFixed(0) }}%
          </span>
          <span class="rate-used-sub">
            已用 {{ item.usedPercent.toFixed(0) }}%
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
    <div
      v-if="resetCredits"
      class="reset-credits-panel"
      :title="formatResetCreditsTooltip(resetCredits)"
    >
      <div class="reset-credits-left">
        <div class="reset-icon-pill">
          <Icon icon="solar:restart-square-linear" />
        </div>
        <div class="reset-info">
          <div class="reset-title-row">
            <span class="reset-label">{{ t('quota.resetCredits') }}</span>
            <span
              v-if="resetCredits.nearestExpiresAt || resetCredits.nearestExpiresInSeconds"
              class="reset-expire-badge"
            >
              <Icon icon="solar:calendar-date-linear" />
              {{ formatExpiryTag(resetCredits.nearestExpiresAt, resetCredits.nearestExpiresInSeconds) }}
            </span>
          </div>
          <span class="reset-desc">
            {{ formatResetCreditsSubtitle(resetCredits) }}
          </span>
        </div>
      </div>
      <div class="reset-credits-count">
        <strong class="reset-number">{{ resetCredits.availableCount }}</strong>
        <span class="reset-unit">{{ t('quota.resetCreditsUnit') }}</span>
      </div>
    </div>
  </div>

  <!-- 无 RateLimit 时的独立重置次数展示兜底 -->
  <div
    v-else-if="resetCredits"
    class="reset-credits-panel"
    :title="formatResetCreditsTooltip(resetCredits)"
  >
    <div class="reset-credits-left">
      <div class="reset-icon-pill">
        <Icon icon="solar:restart-square-linear" />
      </div>
      <div class="reset-info">
        <div class="reset-title-row">
          <span class="reset-label">{{ t('quota.resetCredits') }}</span>
          <span
            v-if="resetCredits.nearestExpiresAt || resetCredits.nearestExpiresInSeconds"
            class="reset-expire-badge"
          >
            <Icon icon="solar:calendar-date-linear" />
            {{ formatExpiryTag(resetCredits.nearestExpiresAt, resetCredits.nearestExpiresInSeconds) }}
          </span>
        </div>
        <span class="reset-desc">
          {{ formatResetCreditsSubtitle(resetCredits) }}
        </span>
      </div>
    </div>
    <div class="reset-credits-count">
      <strong class="reset-number">{{ resetCredits.availableCount }}</strong>
      <span class="reset-unit">{{ t('quota.resetCreditsUnit') }}</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { Icon } from '@iconify/vue'
import type { ProviderQuota, QuotaKind } from '@/services/quota/quota-service'

defineProps<{
  rateLimitQuotas: Extract<QuotaKind, { type: 'rateLimit' }>[]
  resetCredits?: ProviderQuota['resetCredits']
}>()

const { t } = useI18n({ useScope: 'global' })

function getRateColorClass(usedPercent: number): string {
  const remaining = 100 - usedPercent
  if (remaining <= 10) return 'is-danger'
  if (remaining <= 25) return 'is-warning'
  return 'is-healthy'
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
</script>

<style scoped lang="scss">
.ratelimit-section {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.ratelimit-row {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.ratelimit-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 11px;
}

.rate-label {
  color: var(--lumina-text-secondary);
  font-weight: 500;
}

.rate-percent-group {
  display: flex;
  align-items: center;
  gap: 6px;
}

.rate-remaining-badge {
  font-size: 11px;
  font-weight: 600;
  font-family: var(--lumina-font-mono);
  font-feature-settings: 'tnum';
  font-variant-numeric: tabular-nums;
  padding: 1px 6px;
  border-radius: 4px;

  &.is-healthy {
    background: color-mix(in srgb, var(--lumina-primary, #39786f) 12%, transparent);
    color: var(--lumina-primary, #39786f);
  }
  &.is-warning {
    background: rgba(255, 149, 0, 0.12);
    color: #ff9500;
  }
  &.is-danger {
    background: rgba(255, 59, 48, 0.12);
    color: #ff3b30;
  }
}

.rate-used-sub {
  color: var(--lumina-text-tertiary);
  font-size: 10px;
  font-family: var(--lumina-font-mono);
  font-feature-settings: 'tnum';
  font-variant-numeric: tabular-nums;
}

.progress-bar-bg {
  width: 100%;
  height: 6px;
  background: var(--lumina-control-bg);
  border-radius: 3px;
  overflow: hidden;
  box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.06);
}

.progress-bar-fill {
  height: 100%;
  border-radius: 3px;
  transition: width 0.3s cubic-bezier(0.25, 1, 0.5, 1);

  &.is-healthy {
    background: linear-gradient(90deg, var(--lumina-primary, #39786f), color-mix(in srgb, var(--lumina-primary, #39786f) 78%, #ffffff));
  }
  &.is-warning {
    background: linear-gradient(90deg, #ff9500, #ffb340);
  }
  &.is-danger {
    background: linear-gradient(90deg, #ff3b30, #ff453a);
  }
}

.reset-time-hint {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 10.5px;
  color: var(--lumina-text-tertiary);

  svg {
    width: 12px;
    height: 12px;
    flex-shrink: 0;
  }
}

.reset-credits-panel {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  border-radius: 8px;
  background: color-mix(in srgb, var(--lumina-primary, #39786f) 6%, transparent);
  border: 0.5px solid color-mix(in srgb, var(--lumina-primary, #39786f) 20%, transparent);
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
  border-radius: 6px;
  background: color-mix(in srgb, var(--lumina-primary, #39786f) 14%, transparent);
  color: var(--lumina-primary, #39786f);
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
  border-radius: 4px;
  background: color-mix(in srgb, var(--lumina-primary, #39786f) 12%, transparent);
  color: var(--lumina-primary, #39786f);
  border: 0.5px solid color-mix(in srgb, var(--lumina-primary, #39786f) 25%, transparent);
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
  font-size: 16px;
  font-weight: 700;
  font-family: var(--lumina-font-mono);
  font-feature-settings: 'tnum';
  font-variant-numeric: tabular-nums;
  color: var(--lumina-primary, #39786f);
  line-height: 1;
}

.reset-unit {
  font-size: 10px;
  font-weight: 500;
  color: var(--lumina-text-secondary);
}
</style>
