<template>
  <div class="quota-card" :class="{ 'is-unhealthy': !quota.isHealthy || quota.errorMessage }">
    <!-- 卡片头部：图标 + 标题 + Plan 标签（纯净通透，无操作按钮挤压） -->
    <header class="card-header">
      <div class="header-main">
        <div class="provider-badge" :data-provider="quota.providerType">
          <ProviderBrandLogo :provider="quota.providerType" :size="20" />
        </div>
        <div class="provider-info">
          <div class="title-row">
            <h3 class="account-name" :title="quota.name">{{ quota.name }}</h3>
            <span v-if="quota.plan" class="plan-tag">{{ quota.plan }}</span>
          </div>
          <span v-if="cardSubtitle" class="provider-type-label">{{ cardSubtitle }}</span>
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

        <!-- 1. 余额类型展示 -->
        <QuotaCardBalance
          :balance-quotas="balanceQuotas"
          :has-other-quotas="hasOtherQuotas"
        />

        <!-- 2. 周期限额类型与重置次数展示 -->
        <QuotaCardRateLimit
          :rate-limit-quotas="rateLimitQuotas"
          :reset-credits="quota.resetCredits"
        />

        <!-- 3. 点数/积分/Credits 展示 -->
        <QuotaCardCredits
          :credits-quotas="creditsQuotas"
        />
      </div>

      <!-- 4. Pace 节奏健康评估预测 -->
      <QuotaCardPace
        v-if="quota.pace"
        :pace="quota.pace"
      />
    </main>

    <!-- 卡片底部：左侧更新时间，右侧操作按钮栏 -->
    <footer class="card-footer">
      <span class="update-time">
        {{ t('quota.lastChecked') }} {{ formatRelativeTime(quota.lastUpdated) }}
      </span>

      <div class="footer-actions">
        <button
          v-if="isDesktopClient"
          class="macos-card-action-btn primary"
          type="button"
          :disabled="launching"
          @click="emit('launch', quota.accountId, quota.providerType)"
        >
          <Icon :icon="launching ? 'solar:restart-linear' : 'solar:play-circle-linear'" :class="{ spinning: launching }" />
          <span>{{ launching ? t('quota.launchingClient') : launchActionLabel }}</span>
        </button>
        <button
          v-else-if="quota.officialDashboardUrl"
          class="macos-card-action-btn"
          type="button"
          @click="handleOpenExternal(quota.officialDashboardUrl)"
        >
          <Icon icon="solar:link-linear" />
          <span>{{ t('quota.openDashboardShort') }}</span>
        </button>

        <NDropdown
          :options="cardActionOptions"
          placement="top-end"
          trigger="click"
          @select="handleActionSelect"
        >
          <button class="macos-card-more-btn" type="button" :title="t('quota.moreActions')">
            <Icon icon="solar:menu-dots-bold" />
          </button>
        </NDropdown>
      </div>
    </footer>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { NDropdown, type DropdownOption } from 'naive-ui'
import { Icon } from '@iconify/vue'
import ProviderBrandLogo from './ProviderBrandLogo.vue'
import QuotaCardBalance from './card/QuotaCardBalance.vue'
import QuotaCardRateLimit from './card/QuotaCardRateLimit.vue'
import QuotaCardCredits from './card/QuotaCardCredits.vue'
import QuotaCardPace from './card/QuotaCardPace.vue'
import { openExternalUrl } from '@/services/app-service'
import type { ProviderQuota, QuotaKind } from '@/services/quota/quota-service'

const props = defineProps<{
  quota: ProviderQuota
  refreshing?: boolean
  launching?: boolean
}>()

const emit = defineEmits<{
  (e: 'refresh', accountId: string): void
  (e: 'edit', accountId: string): void
  (e: 'launch', accountId: string, providerType: ProviderQuota['providerType']): void
}>()

const { t } = useI18n({ useScope: 'global' })

const isDesktopClient = computed(() => {
  if (['codex', 'cursor', 'qcode', 'trae', 'zcode', 'workbuddy', 'mimo'].includes(props.quota.providerType)) return true
  if (props.quota.providerType !== 'gemini') return false
  return `${props.quota.name} ${props.quota.plan ?? ''}`.toLowerCase().includes('antigravity')
})

const launchActionLabel = computed(() =>
  props.quota.isHealthy && !props.quota.errorMessage
    ? t('quota.launchClient')
    : t('quota.launchAndRefresh')
)

const cardActionOptions = computed<DropdownOption[]>(() => {
  const options: DropdownOption[] = [
    { label: t('quota.editAccount'), key: 'edit' },
    { label: t('quota.refreshSingle'), key: 'refresh', disabled: props.refreshing },
  ]
  if (isDesktopClient.value && props.quota.officialDashboardUrl) {
    options.push({ type: 'divider', key: 'divider' })
    options.push({ label: t('quota.openDashboard'), key: 'dashboard' })
  }
  return options
})

function handleActionSelect(key: string | number) {
  if (key === 'edit') emit('edit', props.quota.accountId)
  if (key === 'refresh') emit('refresh', props.quota.accountId)
  if (key === 'dashboard' && props.quota.officialDashboardUrl) {
    void handleOpenExternal(props.quota.officialDashboardUrl)
  }
}

const providerDisplayName = computed(() => {
  switch (props.quota.providerType) {
    case 'codex':
      return 'OpenAI / Codex'
    case 'claude':
      return 'Anthropic / Claude Code'
    case 'gemini':
      return 'Google Gemini / Antigravity'
    case 'deepseek':
      return 'DeepSeek'
    case 'openrouter':
      return 'OpenRouter'
    case 'opencode':
      return 'OpenCode'
    case 'workbuddy':
      return 'Tencent WorkBuddy (企业助手)'
    case 'siliconflow':
      return 'SiliconFlow (硅基流动)'
    case 'moonshot':
      return 'Moonshot (月之暗面 / Kimi)'
    case 'zhipu':
      return '智谱 GLM (BigModel)'
    case 'qwen':
      return '通义千问 (阿里云百炼)'
    case 'minimax':
      return 'MiniMax (稀宇科技)'
    case 'cursor':
      return 'Cursor IDE'
    case 'qcode':
      return '阿里灵码 (Qoder CN)'
    case 'trae':
      return 'ByteDance Trae (AI IDE)'
    case 'zcode':
      return '智谱清言 Z-Code (GLM 智能客户端)'
    case 'mimo':
      return 'Xiaomi MiMo (小米大模型)'
    default:
      return 'OpenAI-Compatible'
  }
})

const cardSubtitle = computed(() => {
  const name = props.quota.name.trim().toLowerCase()
  const display = providerDisplayName.value.trim().toLowerCase()
  if (name === display || name.includes(display) || display.includes(name)) {
    switch (props.quota.providerType) {
      case 'qcode':
        return 'Alibaba Cloud · 阿里云通义灵码'
      case 'cursor':
        return 'Anysphere · Cursor AI 编辑器'
      case 'opencode':
        return 'OpenCode · 终端开源智能体'
      case 'trae':
        return 'ByteDance · 字节跳动原生 AI IDE'
      case 'zcode':
        return '智谱清言 · GLM 原生代码助手'
      case 'mimo':
        return 'Xiaomi · 开放平台 API 与 MiMo Code'
      case 'workbuddy':
        return 'Tencent · 腾讯企微与混元助手'
      case 'deepseek':
        return 'DeepSeek 官方 API 资产'
      case 'codex':
        return 'OpenAI 官方客户端与 ChatGPT'
      case 'claude':
        return 'Anthropic · Claude Code CLI'
      case 'gemini':
        return 'Google AI Studio · Gemini 算力'
      default:
        return undefined
    }
  }
  return providerDisplayName.value
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

const hasOtherQuotas = computed(() => {
  return rateLimitQuotas.value.length > 0 || creditsQuotas.value.length > 0
})

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
  border-radius: 12px;
  padding: 14px 16px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.04), 0 4px 12px rgba(0, 0, 0, 0.02);
  transition: transform var(--lumina-duration-fast) var(--lumina-ease-out),
              box-shadow var(--lumina-duration-fast) var(--lumina-ease-out),
              border-color var(--lumina-duration-fast) var(--lumina-ease-out);
  position: relative;

  &:hover {
    transform: translateY(-1px);
    border-color: var(--lumina-separator-strong);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.08);
  }

  &.is-unhealthy {
    border-color: rgba(255, 59, 48, 0.35);
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
  width: 36px;
  height: 36px;
  border-radius: 9px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  border: 0.5px solid rgba(0, 0, 0, 0.06);
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.04);

  :deep(.brand-svg),
  :deep(.brand-img),
  svg,
  img {
    width: 22px;
    height: 22px;
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
  &[data-provider='cursor'] {
    background: rgba(0, 0, 0, 0.08);
  }
  &[data-provider='qcode'] {
    background: rgba(99, 102, 241, 0.12);
  }
  &[data-provider='trae'] {
    background: rgba(0, 180, 255, 0.12);
  }
  &[data-provider='zcode'] {
    background: rgba(37, 99, 235, 0.12);
  }
  &[data-provider='mimo'] {
    background: rgba(255, 103, 0, 0.12);
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
  letter-spacing: -0.01em;
  flex-shrink: 1;
}

.plan-tag {
  font-size: 10.5px;
  font-weight: 500;
  padding: 1px 6px;
  border-radius: 4px;
  background: var(--lumina-control-bg);
  color: var(--lumina-text-secondary);
  border: 0.5px solid var(--lumina-separator);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 150px;
  flex-shrink: 0;
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
  border-radius: 8px;
  background: rgba(255, 59, 48, 0.08);
  border: 0.5px solid rgba(255, 59, 48, 0.25);
  color: #ff3b30;
  font-size: 11px;
  line-height: 1.4;

  svg {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
    margin-top: 1px;
  }
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
  gap: 6px;
}

.macos-card-action-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 5px;
  height: 26px;
  padding: 0 10px;
  border-radius: 6px;
  font-size: 11px;
  font-weight: 500;
  cursor: pointer;
  background: var(--lumina-control-bg);
  border: 0.5px solid var(--lumina-separator);
  color: var(--lumina-text);
  box-shadow: 0 0.5px 1px rgba(0, 0, 0, 0.04);
  transition: all var(--lumina-duration-fast);

  &:hover:not(:disabled) {
    background: var(--lumina-control-hover);
    border-color: var(--lumina-separator-strong);
  }

  &:active:not(:disabled) {
    transform: scale(0.98);
  }

  &:disabled {
    cursor: default;
    opacity: 0.55;
  }

  &.primary {
    background: var(--lumina-primary, #39786f);
    border-color: var(--lumina-primary, #39786f);
    color: var(--lumina-on-primary, #ffffff);
    box-shadow: 0 1px 2px color-mix(in srgb, var(--lumina-primary, #39786f) 28%, transparent);

    &:hover:not(:disabled) {
      filter: brightness(1.08);
      background: var(--lumina-primary, #39786f);
      border-color: var(--lumina-primary, #39786f);
    }

    &:active:not(:disabled) {
      transform: scale(0.98);
      filter: brightness(0.96);
    }
  }

  svg {
    width: 13px;
    height: 13px;
  }

  .spinning {
    animation: spin 1s linear infinite;
  }
}

.macos-card-more-btn {
  width: 26px;
  height: 26px;
  border-radius: 6px;
  background: var(--lumina-control-bg);
  border: 0.5px solid var(--lumina-separator);
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 0.5px 1px rgba(0, 0, 0, 0.04);
  transition: all var(--lumina-duration-fast);

  &:hover {
    background: var(--lumina-control-hover);
    color: var(--lumina-text);
    border-color: var(--lumina-separator-strong);
  }

  &:active {
    transform: scale(0.96);
  }

  svg {
    width: 14px;
    height: 14px;
  }
}

@keyframes spin {
  100% {
    transform: rotate(360deg);
  }
}
</style>
