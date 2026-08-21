<template>
  <n-modal
    :show="show"
    preset="card"
    :title="t('quota.manageAccounts')"
    class="account-modal"
    style="width: 540px; max-width: calc(100vw - 32px)"
    @update:show="emit('update:show', $event)"
  >
    <div class="modal-container">
      <!-- 账号状态与启停列表 -->
      <div class="accounts-list">
        <div v-if="accounts.length === 0" class="empty-hint">
          <Icon icon="solar:box-minimalistic-linear" />
          <p>{{ t('quota.noAccountsConfigured') }}</p>
        </div>

        <div
          v-for="(acc, index) in accounts"
          :key="acc.id"
          class="account-item"
          :class="{ 'is-disabled': !acc.enabled }"
        >
          <div class="account-item-leading">
            <div class="provider-tag" :data-provider="acc.providerType">
              {{ formatProviderName(acc.providerType) }}
            </div>
            <div class="account-meta">
              <strong class="account-name-text">{{ acc.name }}</strong>
              <span v-if="acc.autoDiscovered" class="discovered-badge">
                {{ t('quota.autoDiscovered') }}
              </span>
            </div>
          </div>

          <div class="account-item-actions">
            <NSwitch
              size="small"
              :value="acc.enabled"
              @update:value="toggleAccountEnabled(index, $event)"
            />
            <button
              class="icon-action-btn danger"
              type="button"
              :title="t('quota.deleteAccount')"
              @click="deleteAccount(index)"
            >
              <Icon icon="solar:trash-bin-trash-linear" />
            </button>
          </div>
        </div>
      </div>
    </div>
  </n-modal>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { NSwitch, useMessage } from 'naive-ui'
import {
  loadQuotaAccounts,
  saveQuotaAccounts,
  type AccountConfig,
  type ProviderType,
} from '@/services/quota/quota-service'

const props = defineProps<{
  show: boolean
}>()

const emit = defineEmits<{
  (e: 'update:show', val: boolean): void
  (e: 'saved'): void
}>()

const { t } = useI18n({ useScope: 'global' })
const message = useMessage()

const accounts = ref<AccountConfig[]>([])

watch(
  () => props.show,
  async (val) => {
    if (val) {
      await reloadAccounts()
    }
  }
)

async function reloadAccounts() {
  try {
    accounts.value = await loadQuotaAccounts()
  } catch (err) {
    message.error(t('quota.loadAccountsFailed') + ': ' + String(err))
  }
}

function formatProviderName(provider: ProviderType) {
  switch (provider) {
    case 'codex':
      return 'Codex'
    case 'deepseek':
      return 'DeepSeek'
    case 'openrouter':
      return 'OpenRouter'
    case 'gemini':
      return 'Google AI Pro'
    default:
      return 'Custom'
  }
}

async function deleteAccount(index: number) {
  accounts.value.splice(index, 1)
  await persistAccounts()
}

async function toggleAccountEnabled(index: number, val: boolean) {
  accounts.value[index].enabled = val
  await persistAccounts()
}

async function persistAccounts() {
  try {
    await saveQuotaAccounts(accounts.value)
    emit('saved')
  } catch (err) {
    message.error(String(err))
  }
}
</script>

<style scoped lang="scss">
.modal-container {
  display: flex;
  flex-direction: column;
}

.accounts-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-height: 380px;
  overflow-y: auto;
  padding-right: 4px;
}

.empty-hint {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 32px 0;
  color: var(--lumina-text-tertiary);

  svg {
    width: 32px;
    height: 32px;
  }
}

.account-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 14px;
  border-radius: var(--lumina-radius-md);
  background: var(--lumina-surface-secondary);
  border: 0.5px solid var(--lumina-separator);
  transition: all var(--lumina-duration-fast);

  &.is-disabled {
    opacity: 0.6;
  }
}

.account-item-leading {
  display: flex;
  align-items: center;
  gap: 10px;
}

.provider-tag {
  font-size: 11px;
  font-weight: 600;
  padding: 2px 8px;
  border-radius: 4px;
  background: var(--lumina-control-bg);
  border: 0.5px solid var(--lumina-separator);
  color: var(--lumina-text);

  &[data-provider='deepseek'] {
    color: #0284c7;
  }
  &[data-provider='codex'] {
    color: #10b981;
  }
  &[data-provider='openrouter'] {
    color: #8b5cf6;
  }
  &[data-provider='gemini'] {
    color: #f59e0b;
  }
}

.account-meta {
  display: flex;
  align-items: center;
  gap: 6px;
}

.account-name-text {
  font-size: 13px;
  font-weight: 500;
  color: var(--lumina-text);
}

.discovered-badge {
  font-size: 10px;
  padding: 1px 5px;
  border-radius: 3px;
  background: var(--lumina-primary-soft);
  color: var(--lumina-primary);
}

.account-item-actions {
  display: flex;
  align-items: center;
  gap: 12px;
}

.icon-action-btn {
  width: 26px;
  height: 26px;
  border-radius: 4px;
  background: transparent;
  border: 0;
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;

  &:hover {
    background: var(--lumina-control-hover);
    color: var(--lumina-text);
  }

  &.danger:hover {
    color: var(--lumina-danger);
    background: rgba(220, 38, 38, 0.1);
  }

  svg {
    width: 14px;
    height: 14px;
  }
}
</style>
