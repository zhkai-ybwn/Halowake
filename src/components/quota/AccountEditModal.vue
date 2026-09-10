<template>
  <n-modal
    :show="show"
    preset="card"
    :title="isCreating ? t('quota.createAccount') : t('quota.editAccount')"
    class="account-edit-modal"
    style="width: 520px; max-width: calc(100vw - 32px)"
    @update:show="emit('update:show', $event)"
  >
    <div class="edit-form-content">
      <div class="form-item">
        <label>{{ t('quota.providerType') }}</label>
        <NSelect
          v-model:value="formData.providerType"
          :options="providerOptions"
          :disabled="!isCreating"
        />
      </div>

      <div class="form-item">
        <label>{{ t('quota.accountAlias') }}</label>
        <NInput
          v-model:value="formData.name"
          :placeholder="t('quota.accountAliasPlaceholder')"
        />
      </div>

      <div v-if="formData.providerType !== 'codex'" class="form-item">
        <label>{{ t('quota.apiKey') }}</label>
        <NInput
          v-model:value="formData.apiKey"
          type="password"
          show-password-on="click"
          :placeholder="apiKeyPlaceholder"
        />
        <span v-if="isCliProvider" class="form-hint">
          {{ t('quota.cliAutoDetectHint', '支持自动读取本地 CLI 登录态配置，亦可在此填入 Token/Key 覆盖') }}
        </span>
      </div>

      <div v-if="supportsBaseUrl" class="form-item">
        <label>{{ t('quota.baseUrlOptional') }}</label>
        <NInput
          v-model:value="formData.baseUrl"
          :placeholder="t('quota.baseUrlPlaceholder')"
        />
      </div>
    </div>

    <template #footer>
      <div class="form-actions">
        <WorkbenchButton @click="emit('update:show', false)">{{ t('common.cancel') }}</WorkbenchButton>
        <WorkbenchButton variant="primary" :disabled="saving" @click="handleSave">
          {{ saving ? t('common.saving') : t('common.save') }}
        </WorkbenchButton>
      </div>
    </template>
  </n-modal>
</template>

<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { NSelect, NInput, useMessage } from 'naive-ui'
import WorkbenchButton from '@/components/workbench/WorkbenchButton.vue'
import {
  loadQuotaAccounts,
  saveQuotaAccounts,
  type AccountConfig,
} from '@/services/quota/quota-service'

const props = defineProps<{
  show: boolean
  account?: AccountConfig | null
}>()

const emit = defineEmits<{
  (e: 'update:show', val: boolean): void
  (e: 'saved'): void
}>()

const { t } = useI18n({ useScope: 'global' })
const message = useMessage()

const isCreating = computed(() => !props.account)
const saving = ref(false)

const formData = ref<AccountConfig>({
  id: '',
  providerType: 'deepseek',
  name: '',
  apiKey: '',
  baseUrl: '',
  enabled: true,
  autoDiscovered: false,
})

const providerOptions = computed(() => [
  { label: 'Claude Code (Anthropic / CLI)', value: 'claude' },
  { label: 'Codex (OpenAI / CLI)', value: 'codex' },
  { label: 'OpenCode (Go / Zen)', value: 'opencode' },
  { label: 'Tencent WorkBuddy (企业助手 / 积分)', value: 'workbuddy' },
  { label: 'Cursor IDE', value: 'cursor' },
  { label: '阿里灵码 (Qoder CN)', value: 'qcode' },
  { label: 'Trae (字节跳动 AI IDE)', value: 'trae' },
  { label: 'Z-Code (智谱 GLM)', value: 'zcode' },
  { label: 'DeepSeek', value: 'deepseek' },
  { label: 'SiliconFlow (硅基流动)', value: 'siliconflow' },
  { label: 'Moonshot (月之暗面 / Kimi)', value: 'moonshot' },
  { label: '智谱 GLM (BigModel)', value: 'zhipu' },
  { label: '通义千问（暂不支持自动额度查询）', value: 'qwen', disabled: true },
  { label: 'MiniMax（暂不支持自动额度查询）', value: 'minimax', disabled: true },
  { label: 'Google AI Pro (Gemini)', value: 'gemini' },
  { label: 'OpenRouter', value: 'openrouter' },
  { label: 'Custom OpenAI-compatible', value: 'custom' },
])

const isCliProvider = computed(() => {
  return ['claude', 'opencode', 'workbuddy', 'cursor', 'qcode', 'trae', 'zcode'].includes(
    formData.value.providerType
  )
})

const supportsBaseUrl = computed(() => {
  return ['deepseek', 'siliconflow', 'moonshot', 'zhipu', 'qwen', 'minimax', 'custom'].includes(
    formData.value.providerType
  )
})

const apiKeyPlaceholder = computed(() => {
  if (formData.value.providerType === 'claude') return '可留空以自动读取 ~/.claude.json 或输入 sk-ant-...'
  if (formData.value.providerType === 'opencode') return '可留空以读取本地配置，或输入 OpenCode API Key'
  if (formData.value.providerType === 'workbuddy') return '可留空以读取本地 ~/.workbuddy 凭据'
  if (formData.value.providerType === 'cursor') return '可留空以读取 Cursor 登录态，或输入 Access Token'
  if (formData.value.providerType === 'qcode') return '可留空读取 Qoder 登录态，或输入 dt- / jt- / pt- Token'
  if (formData.value.providerType === 'trae') return 'Trae 当前仅支持读取本地登录状态'
  if (formData.value.providerType === 'zcode') return '可留空以读取 ~/.zcode/v2/credentials.json'
  return t('quota.apiKeyPlaceholder')
})

watch(
  () => props.show,
  (val) => {
    if (val) {
      if (props.account) {
        formData.value = { ...props.account }
      } else {
        formData.value = {
          id: 'acc-' + Date.now(),
          providerType: 'deepseek',
          name: 'DeepSeek 主力账号',
          apiKey: '',
          baseUrl: '',
          enabled: true,
          autoDiscovered: false,
        }
      }
    }
  }
)

async function handleSave() {
  if (!formData.value.name.trim()) {
    message.warning(t('quota.nameRequired'))
    return
  }

  saving.value = true
  try {
    const existing = await loadQuotaAccounts()
    if (isCreating.value) {
      existing.push({ ...formData.value })
    } else {
      const idx = existing.findIndex((a) => a.id === formData.value.id)
      if (idx !== -1) {
        existing[idx] = { ...formData.value }
      } else {
        existing.push({ ...formData.value })
      }
    }
    await saveQuotaAccounts(existing)
    message.success(t('common.savedSuccessfully'))
    emit('saved')
    emit('update:show', false)
  } catch (err) {
    message.error(String(err))
  } finally {
    saving.value = false
  }
}
</script>

<style scoped lang="scss">
.edit-form-content {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.form-item {
  display: flex;
  flex-direction: column;
  gap: 6px;

  label {
    font-size: 11px;
    color: var(--lumina-text-secondary);
  }

  .form-hint {
    font-size: 11px;
    color: var(--lumina-text-tertiary);
    line-height: 1.4;
  }
}

.form-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}
</style>
