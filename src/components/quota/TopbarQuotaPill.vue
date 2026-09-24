<template>
  <div class="topbar-quota-pill-host" @mousedown.stop @dblclick.stop>
    <button
      type="button"
      class="topbar-quota-pill"
      :class="{
        'has-warning': quotaStore.statusTone === 'warning',
        'has-danger': quotaStore.statusTone === 'danger',
        'is-refreshing': quotaStore.refreshing,
      }"
      :title="statusTooltip"
      @click="quotaStore.openModal"
    >
      <span class="pill-dot" :class="quotaStore.statusDotClass"></span>
      <Icon
        :icon="quotaStore.refreshing ? 'solar:restart-linear' : 'solar:wallet-money-linear'"
        class="pill-icon"
        :class="{ 'spin-icon': quotaStore.refreshing }"
      />
      <span class="pill-text">{{ pillLabel }}</span>
    </button>

    <!-- Quota Manager Modal -->
    <QuotaManagerModal />
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { Icon } from '@iconify/vue'
import { useI18n } from 'vue-i18n'
import { useQuotaStore } from '@/stores/quota'
import { isMacPlatform } from '@/utils/platform-shortcuts'
import QuotaManagerModal from './QuotaManagerModal.vue'

const { t } = useI18n({ useScope: 'global' })
const quotaStore = useQuotaStore()
const isMac = isMacPlatform

const pillLabel = computed(() => {
  return t('workbench.aiQuota')
})

const statusTooltip = computed(() => {
  const parts: string[] = []
  if (quotaStore.statusTone === 'danger') {
    parts.push(`${t('workbench.aiQuota')} (${t('quota.statusDanger')})`)
  } else if (quotaStore.statusTone === 'warning') {
    parts.push(`${t('workbench.aiQuota')} (${t('quota.statusWarning')})`)
  } else {
    parts.push(t('workbench.aiQuota'))
  }

  if (quotaStore.quotas.length > 0) {
    parts.push(`¥${quotaStore.summary.totalCnyBalance.toFixed(2)} / $${quotaStore.summary.totalUsdBalance.toFixed(2)}`)
  }

  parts.push(isMac ? '⌘5' : 'Alt+5')
  return parts.join(' · ')
})

onMounted(() => {
  void quotaStore.ensureFresh(15000)
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

  &:hover {
    background: var(--lumina-control-hover);
    color: var(--lumina-text);
  }

  &.has-warning {
    border-color: rgba(245, 158, 11, 0.4);

    .pill-text {
      color: #f59e0b;
    }
  }

  &.has-danger {
    border-color: rgba(239, 68, 68, 0.5);

    .pill-text {
      color: #ef4444;
      font-weight: 600;
    }
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
    box-shadow: 0 0 6px rgba(255, 59, 48, 0.5);
  }

  &.idle {
    background: var(--lumina-text-tertiary);
  }
}

.pill-icon {
  font-size: 13px;

  &.spin-icon {
    animation: spin 1s linear infinite;
  }
}

.pill-text {
  font-family: var(--lumina-font-mono, monospace);
  font-weight: 550;
  white-space: nowrap;
}

@keyframes spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}
</style>
