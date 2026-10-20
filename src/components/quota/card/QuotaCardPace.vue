<template>
  <div class="pace-section" :class="'pace-' + pace.level">
    <div class="pace-header">
      <div class="pace-header-left">
        <span class="pace-dot"></span>
        <span class="pace-level-text">{{ getPaceLabel(pace.level) }}</span>
      </div>
      <span
        v-if="pace.projectedUsagePercent !== undefined && pace.projectedUsagePercent !== null"
        class="projected-tag"
      >
        {{ t('quota.projected') }} {{ Math.round(pace.projectedUsagePercent) }}%
      </span>
    </div>
    <p class="pace-desc">{{ formatPaceMessage(pace.message) }}</p>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { PaceLevel, ProviderQuota } from '@/services/quota/quota-service'

defineProps<{
  pace: NonNullable<ProviderQuota['pace']>
}>()

const { t } = useI18n({ useScope: 'global' })

function getPaceLabel(level: PaceLevel): string {
  switch (level) {
    case 'onPace':
      return t('quota.paceHealthy')
    case 'tight':
      return t('quota.paceTight')
    case 'overPace':
      return t('quota.paceOver')
    default:
      return ''
  }
}

function formatPaceMessage(msg?: string): string {
  if (!msg) return ''
  return msg.replace(/(\d+)\.\d+(?=\s*[%％])/g, '$1')
}
</script>

<style scoped lang="scss">
.pace-section {
  padding: 8px 12px;
  border-radius: 8px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 11px;
  margin-top: 4px;

  &.pace-onPace {
    background: color-mix(in srgb, var(--lumina-primary, #39786f) 8%, transparent);
    border: 0.5px solid color-mix(in srgb, var(--lumina-primary, #39786f) 25%, transparent);

    .pace-dot {
      background: var(--lumina-primary, #39786f);
    }
    .pace-level-text {
      color: var(--lumina-primary, #39786f);
    }
  }

  &.pace-tight {
    background: rgba(255, 149, 0, 0.08);
    border: 0.5px solid rgba(255, 149, 0, 0.25);

    .pace-dot {
      background: #ff9500;
    }
    .pace-level-text {
      color: #ff9500;
    }
  }

  &.pace-overPace {
    background: rgba(255, 59, 48, 0.08);
    border: 0.5px solid rgba(255, 59, 48, 0.25);

    .pace-dot {
      background: #ff3b30;
    }
    .pace-level-text {
      color: #ff3b30;
    }
  }
}

.pace-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.pace-header-left {
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
  font-size: 10px;
  color: var(--lumina-text-tertiary);
  font-family: var(--lumina-font-mono);
  font-feature-settings: 'tnum';
  font-variant-numeric: tabular-nums;
}

.pace-desc {
  margin: 0;
  font-size: 10.5px;
  color: var(--lumina-text-secondary);
  line-height: 1.35;
}
</style>
