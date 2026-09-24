<template>
  <div v-if="balanceQuotas.length > 0" class="quota-balance-wrapper">
    <!-- 余额卡片列表 -->
    <div v-for="(item, idx) in balanceQuotas" :key="'bal-' + idx" class="balance-section">
      <div class="balance-card">
        <div class="balance-card-header">
          <span class="balance-card-label">{{ item.currency === 'USD' ? '美元可用余额' : '人民币可用余额' }}</span>
          <span class="currency-tag">{{ item.currency }}</span>
        </div>
        <div class="balance-main">
          <span class="currency-symbol">{{ item.currency === 'USD' ? '$' : '¥' }}</span>
          <span class="balance-value">{{ item.totalRemaining.toFixed(2) }}</span>
        </div>

        <div v-if="item.toppedUp > 0 || item.granted > 0" class="balance-breakdown">
          <div v-if="item.toppedUp > 0" class="breakdown-item">
            <span class="breakdown-label">{{ t('quota.toppedUp') }}</span>
            <span class="breakdown-val">{{ item.currency === 'USD' ? '$' : '¥' }}{{ item.toppedUp.toFixed(2) }}</span>
          </div>
          <div v-if="item.granted > 0" class="breakdown-item breakdown-item--granted">
            <span class="breakdown-label">{{ t('quota.granted') }}</span>
            <span class="breakdown-val">{{ item.currency === 'USD' ? '$' : '¥' }}{{ item.granted.toFixed(2) }}</span>
          </div>
        </div>
      </div>
    </div>

    <!-- 如果仅有余额且无其它限额，展示精致的 API 待命状态消除空白感 -->
    <div
      v-if="!hasOtherQuotas"
      class="api-service-status"
    >
      <div class="status-left">
        <span class="pulse-dot-green"></span>
        <span>API 接口运行平稳 · 随时待命</span>
      </div>
      <span class="api-protocol-badge">REST API</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { QuotaKind } from '@/services/quota/quota-service'

defineProps<{
  balanceQuotas: Extract<QuotaKind, { type: 'balance' }>[]
  hasOtherQuotas: boolean
}>()

const { t } = useI18n({ useScope: 'global' })
</script>

<style scoped lang="scss">
.quota-balance-wrapper {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.balance-section {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.balance-card {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px 14px;
  border-radius: 10px;
  background: linear-gradient(135deg, color-mix(in srgb, var(--lumina-primary, #39786f) 5%, transparent) 0%, rgba(52, 199, 89, 0.04) 100%), var(--lumina-surface-2);
  border: 0.5px solid var(--lumina-separator);
}

.balance-card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.balance-card-label {
  font-size: 11px;
  font-weight: 500;
  color: var(--lumina-text-secondary);
}

.currency-tag {
  font-size: 10px;
  font-weight: 600;
  font-family: var(--lumina-font-mono);
  padding: 1px 5px;
  border-radius: 4px;
  background: var(--lumina-control-bg);
  color: var(--lumina-text-tertiary);
  border: 0.5px solid var(--lumina-separator);
}

.balance-main {
  display: flex;
  align-items: baseline;
  gap: 4px;
}

.currency-symbol {
  font-size: 16px;
  font-weight: 600;
  color: var(--lumina-text-secondary);
}

.balance-value {
  font-size: 26px;
  font-weight: 700;
  font-family: var(--lumina-font-mono);
  font-feature-settings: 'tnum';
  font-variant-numeric: tabular-nums;
  color: var(--lumina-text);
  line-height: 1;
  letter-spacing: -0.02em;
}

.balance-breakdown {
  display: flex;
  align-items: center;
  gap: 14px;
  padding-top: 8px;
  margin-top: 2px;
  border-top: 0.5px dashed var(--lumina-separator);
  font-size: 11px;
}

.breakdown-item {
  display: flex;
  align-items: center;
  gap: 4px;
}

.breakdown-label {
  color: var(--lumina-text-tertiary);
  font-size: 10.5px;
}

.breakdown-val {
  font-family: var(--lumina-font-mono);
  font-weight: 600;
  color: var(--lumina-text-secondary);
  font-feature-settings: 'tnum';
}

.breakdown-item--granted .breakdown-val {
  color: var(--lumina-primary, #39786f);
}

.api-service-status {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  border-radius: 8px;
  background: var(--lumina-surface-2);
  border: 0.5px solid var(--lumina-separator);
  font-size: 11px;
  color: var(--lumina-text-secondary);
}

.status-left {
  display: flex;
  align-items: center;
  gap: 8px;
}

.pulse-dot-green {
  position: relative;
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--lumina-primary, #39786f);
}

.pulse-dot-green::after {
  content: '';
  position: absolute;
  inset: -2px;
  border-radius: 50%;
  border: 1.5px solid var(--lumina-primary, #39786f);
  opacity: 0.5;
  animation: pulse-glow 2.5s infinite cubic-bezier(0.4, 0, 0.6, 1);
  pointer-events: none;
}

@keyframes pulse-glow {
  0%, 100% {
    opacity: 0.25;
    transform: scale(1);
  }
  50% {
    opacity: 0.75;
    transform: scale(1.3);
  }
}

.api-protocol-badge {
  font-size: 9.5px;
  font-weight: 600;
  font-family: var(--lumina-font-mono);
  padding: 1px 6px;
  border-radius: 4px;
  background: color-mix(in srgb, var(--lumina-primary, #39786f) 10%, transparent);
  color: var(--lumina-primary, #39786f);
  border: 0.5px solid color-mix(in srgb, var(--lumina-primary, #39786f) 22%, transparent);
}
</style>
