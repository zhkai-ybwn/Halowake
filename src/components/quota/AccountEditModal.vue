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
          :placeholder="t('quota.apiKeyPlaceholder')"
        />
      </div>

      <div v-if="formData.providerType === 'deepseek' || formData.providerType === 'custom'" class="form-item">
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
  { label: 'DeepSeek', value: 'deepseek' },
  { label: 'OpenRouter', value: 'openrouter' },
  { label: 'Google AI Pro (Gemini)', value: 'gemini' },
  { label: 'Codex (OpenAI / CLI)', value: 'codex' },
  { label: 'Custom OpenAI-compatible', value: 'custom' },
])

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
}

.form-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}
</style>
