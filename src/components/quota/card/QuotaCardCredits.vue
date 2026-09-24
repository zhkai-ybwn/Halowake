<template>
  <div v-if="creditsQuotas.length > 0" class="credits-container">
    <div
      v-for="(item, idx) in creditsQuotas"
      :key="'cred-' + idx"
      class="credit-pill"
      :class="{
        'has-expiry': Boolean(item.expiresAt || item.remainingDays !== undefined),
        'expiring-soon': isExpiringSoon(item),
      }"
    >
      <div class="credit-pill-top">
        <span class="credit-pill-label" :title="item.label">{{ item.label || t('quota.credits') }}</span>
        <span v-if="getCreditExpiryBadge(item)" class="credit-expiry-badge" :class="{ urgent: isExpiringSoon(item) }">
          <Icon icon="solar:clock-circle-linear" class="expire-icon" />
          {{ getCreditExpiryBadge(item) }}
        </span>
      </div>
      <div class="credit-pill-num-group">
        <span class="credit-pill-val">{{ formatCreditValue(item.remaining) }}</span>
        <span v-if="item.total" class="credit-pill-total">/ {{ formatCreditValue(item.total) }}</span>
        <span v-if="getCreditUnit(item)" class="credit-pill-unit">{{ getCreditUnit(item) }}</span>
      </div>
      <div v-if="item.cycleType" class="credit-pill-cycle" :title="item.cycleType">
        <Icon icon="solar:tag-linear" class="cycle-icon" />
        <span>{{ item.cycleType }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { Icon } from '@iconify/vue'
import type { QuotaKind } from '@/services/quota/quota-service'

defineProps<{
  creditsQuotas: Extract<QuotaKind, { type: 'credits' }>[]
}>()

const { t } = useI18n({ useScope: 'global' })

function formatCreditValue(val: number | undefined | null): string {
  if (val === undefined || val === null) return '0'
  return val.toLocaleString(undefined, { maximumFractionDigits: 2 })
}

function getCreditUnit(item: Extract<QuotaKind, { type: 'credits' }>): string {
  if (item.unit !== undefined && item.unit !== null) {
    return item.unit
  }
  if (item.cycleType?.toLowerCase().includes('token')) {
    return 'Tokens'
  }
  return t('quota.creditsUnit', '点')
}

function isExpiringSoon(item: Extract<QuotaKind, { type: 'credits' }>): boolean {
  if (item.remainingDays !== undefined && item.remainingDays !== null) {
    return item.remainingDays <= 7
  }
  return false
}

function getCreditExpiryBadge(item: Extract<QuotaKind, { type: 'credits' }>): string | null {
  if (item.remainingDays !== undefined && item.remainingDays !== null) {
    if (item.remainingDays === 0) {
      return t('quota.expiresToday', '今日到期')
    }
    if (item.expiresAt) {
      const shortDate = item.expiresAt.includes('-')
        ? item.expiresAt.split(' ')[0].slice(5)
        : item.expiresAt
      return `剩${item.remainingDays}天 · ${shortDate}`
    }
    return `剩 ${item.remainingDays} 天到期`
  }
  if (item.expiresAt) {
    const shortDate = item.expiresAt.includes('-')
      ? item.expiresAt.split(' ')[0].slice(5)
      : item.expiresAt
    return `${shortDate} 到期`
  }
  return null
}
</script>

<style scoped lang="scss">
.credits-container {
  display: flex;
  flex-direction: column;
  gap: 8px;
  width: 100%;
}

.credit-pill {
  display: flex;
  flex-direction: column;
  gap: 6px;
  background: var(--lumina-surface-2);
  border: 0.5px solid var(--lumina-separator);
  border-radius: 8px;
  padding: 8px 12px;
  transition: border-color var(--lumina-duration-fast), background var(--lumina-duration-fast);
}

.credit-pill.expiring-soon {
  border-color: rgba(255, 149, 0, 0.4);
  background: rgba(255, 149, 0, 0.05);
}

.credit-pill-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.credit-pill-label {
  color: var(--lumina-text-secondary);
  font-weight: 600;
  font-size: 12px;
}

.credit-expiry-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 10.5px;
  padding: 2px 6px;
  border-radius: 4px;
  background: var(--lumina-surface-1);
  color: var(--lumina-text-tertiary);
  border: 0.5px solid var(--lumina-separator);
}

.credit-expiry-badge .expire-icon {
  font-size: 12px;
}

.credit-expiry-badge.urgent {
  background: rgba(255, 149, 0, 0.12);
  color: #ff9500;
  border-color: rgba(255, 149, 0, 0.35);
  font-weight: 600;
}

.credit-pill-num-group {
  display: inline-flex;
  align-items: baseline;
  gap: 4px;
}

.credit-pill-val {
  font-family: var(--lumina-font-mono);
  font-feature-settings: 'tnum';
  font-variant-numeric: tabular-nums;
  font-weight: 700;
  color: var(--lumina-primary);
  font-size: 16px;
}

.credit-pill-total {
  font-family: var(--lumina-font-mono);
  font-feature-settings: 'tnum';
  font-variant-numeric: tabular-nums;
  color: var(--lumina-text-tertiary);
  font-size: 12px;
}

.credit-pill-unit {
  font-size: 11px;
  color: var(--lumina-text-tertiary);
  margin-left: 2px;
}

.credit-pill-cycle {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 10.5px;
  color: var(--lumina-text-tertiary);
  margin-top: 2px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;

  .cycle-icon {
    font-size: 11px;
    flex-shrink: 0;
    opacity: 0.7;
  }
}
</style>
