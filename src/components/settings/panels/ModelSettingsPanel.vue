<template>
  <section class="panel-section">
    <div class="panel-header">
      <div>
        <h2>{{ t('settings.aiModels.title') }}</h2>
        <p>{{ t('settings.aiModels.description') }}</p>
      </div>
      <button class="action-button" type="button" @click="createModel">
        {{ t('settings.aiModels.add') }}
      </button>
    </div>

    <div class="model-list">
      <div v-if="!aiSettings.models.length" class="empty-state">
        {{ t('settings.aiModels.empty') }}
      </div>

      <article v-for="model in aiSettings.models" :key="model.id" class="model-item">
        <div class="model-summary" @click="toggleExpanded(model.id)">
          <div class="model-summary__main">
            <Icon :icon="expandedId === model.id ? 'solar:alt-arrow-down-linear' : 'solar:alt-arrow-right-linear'" />
            <div class="model-summary__text">
              <strong>{{ model.name || t('settings.aiModels.untitled') }}</strong>
              <span>{{ providerLabel(model.provider) }} · {{ model.model || t('settings.aiModels.modelNameEmpty') }}</span>
            </div>
          </div>

          <span class="model-url">{{ model.baseUrl }}</span>

          <div class="model-summary__meta">
            <span v-if="model.id === aiSettings.defaultModelId" class="badge primary">{{ t('settings.aiModels.defaultBadge') }}</span>
            <span class="badge" :class="{ muted: !model.enabled }">
              {{ model.enabled ? t('settings.aiModels.enabled') : t('settings.aiModels.disabled') }}
            </span>
          </div>
        </div>

          <div v-if="expandedId === model.id" class="model-editor">
          <div class="model-editor__toolbar">
            <NCheckbox
              class="inline-toggle"
              :checked="model.enabled"
              @update:checked="checked => updateModel(model.id, { enabled: Boolean(checked) })"
            >
              {{ t('settings.aiModels.enabled') }}
            </NCheckbox>

            <button
              class="ghost-button"
              type="button"
              :disabled="model.id === aiSettings.defaultModelId"
              @click="setDefaultModel(model.id)"
            >
              {{ t('settings.aiModels.setDefault') }}
            </button>
          </div>

          <div class="form-grid">
            <label class="field">
              <span>{{ t('settings.aiModels.name') }}</span>
              <input :value="model.name" @input="updateModel(model.id, { name: ($event.target as HTMLInputElement).value })" />
            </label>

            <label class="field">
              <span>{{ t('settings.aiModels.provider') }}</span>
              <NSelect
                class="field-select"
                :value="model.provider"
                :options="providerOptions"
                size="small"
                @update:value="value => updateModel(model.id, { provider: value as AiProviderType })"
              />
            </label>

            <label class="field">
              <span>{{ t('settings.aiModels.baseUrl') }}</span>
              <input
                :value="model.baseUrl"
                placeholder="https://api.example.com/v1"
                @input="updateModel(model.id, { baseUrl: ($event.target as HTMLInputElement).value })"
              />
            </label>

            <label class="field">
              <span>{{ t('settings.aiModels.modelName') }}</span>
              <input
                :value="model.model"
                placeholder="qwen3:8b"
                @input="updateModel(model.id, { model: ($event.target as HTMLInputElement).value })"
              />
            </label>

            <label class="field field--wide">
              <span>{{ t('settings.aiModels.apiKey') }}</span>
              <input
                :value="model.apiKey ?? ''"
                type="password"
                autocomplete="off"
                :placeholder="model.provider === 'ollama' ? t('settings.aiModels.apiKeyOptional') : 'sk-...'"
                @input="updateModel(model.id, { apiKey: ($event.target as HTMLInputElement).value })"
              />
            </label>
          </div>

          <div class="model-item__footer">
            <span
              class="test-result"
              :class="{ error: testErrors[model.id] }"
              role="status"
              aria-live="polite"
            >
              {{ testErrors[model.id] || testResults[model.id] || maskApiKey(model.apiKey) }}
            </span>
            <div class="footer-actions">
              <button class="ghost-button" type="button" :disabled="testingId === model.id" @click="testModel(model)">
                {{ testingId === model.id ? t('settings.aiModels.testing') : t('settings.aiModels.test') }}
              </button>
              <button class="ghost-button danger" type="button" @click="deleteModel(model.id)">
                {{ t('settings.aiModels.delete') }}
              </button>
            </div>
          </div>
        </div>
      </article>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import { NCheckbox, NSelect } from 'naive-ui'
import { useI18n } from 'vue-i18n'
import { testAiModelConnection } from '@/services/ai-settings-service'
import { useAiSettingsStore } from '@/stores/ai-settings'
import type { AiModelConfig, AiProviderType } from '@/types/ai-settings'

const aiSettings = useAiSettingsStore()
const { t } = useI18n({ useScope: 'global' })
const testingId = ref('')
const expandedId = ref('')
const testResults = reactive<Record<string, string>>({})
const testErrors = reactive<Record<string, string>>({})
const providerOptions = computed(() => [
  { label: t('settings.aiModels.openaiCompatible'), value: 'openai-compatible' },
  { label: t('settings.aiModels.ollama'), value: 'ollama' },
])

function createModel() {
  expandedId.value = aiSettings.createModel()
}

function updateModel(modelId: string, patch: Partial<AiModelConfig>) {
  aiSettings.updateModel(modelId, patch)
  delete testResults[modelId]
  delete testErrors[modelId]
}

function deleteModel(modelId: string) {
  aiSettings.deleteModel(modelId)
  if (expandedId.value === modelId) expandedId.value = ''
}

async function testModel(model: AiModelConfig) {
  testingId.value = model.id
  delete testResults[model.id]
  delete testErrors[model.id]

  try {
    testResults[model.id] = await testAiModelConnection(model)
  } catch (error) {
    testErrors[model.id] = error instanceof Error ? error.message : String(error)
  } finally {
    testingId.value = ''
  }
}

function maskApiKey(apiKey?: string) {
  if (!apiKey) return t('settings.aiModels.apiKeyEmpty')
  const trimmed = apiKey.trim()
  if (trimmed.length <= 8) return '****'
  return `${trimmed.slice(0, 4)}****${trimmed.slice(-4)}`
}

function toggleExpanded(modelId: string) {
  expandedId.value = expandedId.value === modelId ? '' : modelId
}

function setDefaultModel(modelId: string) {
  aiSettings.setDefaultModel(modelId)
}

function providerLabel(provider: AiProviderType) {
  return provider === 'ollama' ? t('settings.aiModels.ollama') : t('settings.aiModels.openaiCompatible')
}
</script>

<style scoped lang="scss">
.panel-section,
.model-item {
  background: var(--lumina-card-bg);
  border: 1px solid var(--lumina-card-border);
  border-radius: 8px;
}

.panel-section {
  max-width: 1040px;
  overflow: hidden;
  padding: 0;
}

.panel-header,
.model-item__footer {
  align-items: center;
  gap: var(--lumina-gap-md);
}

.panel-header {
  background: color-mix(in srgb, var(--lumina-surface-2) 70%, var(--lumina-card-bg));
  border-bottom: 1px solid var(--lumina-card-border);
  display: flex;
  justify-content: space-between;
  padding: 18px 20px;
}

h2 {
  font-size: 18px;
  line-height: 1.25;
  margin: 0 0 6px;
}

p {
  color: var(--lumina-text-secondary);
  margin: 0;
}

.model-list {
  background: var(--lumina-surface-1);
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 14px;
}

.model-item {
  background: var(--lumina-surface-2);
  overflow: hidden;
}

.empty-state {
  border: 1px dashed var(--lumina-card-border);
  border-radius: var(--lumina-radius-lg);
  color: var(--lumina-text-secondary);
  padding: var(--lumina-gap-lg);
  text-align: center;
}

.model-summary {
  align-items: center;
  cursor: pointer;
  display: grid;
  gap: 16px;
  grid-template-columns: minmax(260px, 1.2fr) minmax(220px, 0.8fr) auto;
  min-height: 58px;
  padding: 0 14px;
  transition: background 0.18s ease;

  &:hover {
    background: var(--lumina-button-secondary-hover);
  }
}

.model-summary__main,
.model-summary__meta,
.model-editor__toolbar {
  align-items: center;
  display: flex;
  gap: var(--lumina-gap-sm);
  min-width: 0;
}

.model-summary__main {
  flex: 1;
}

.model-summary__main > svg {
  color: var(--lumina-text-secondary);
  flex: 0 0 auto;
  height: 16px;
  width: 16px;
}

.model-summary__text {
  display: flex;
  flex-direction: column;
  gap: 3px;
  min-width: 0;
}

.model-summary__text strong,
.model-summary__text span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.model-summary__text span {
  color: var(--lumina-text-secondary);
  font-size: 12px;
}

.model-url {
  color: var(--lumina-text-secondary);
  font-size: 12px;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.badge {
  align-items: center;
  background: var(--lumina-surface-1);
  border-radius: 999px;
  color: var(--lumina-text-secondary);
  display: inline-flex;
  font-size: 11px;
  height: 22px;
  padding: 0 8px;
  white-space: nowrap;
}

.badge.primary {
  background: var(--lumina-primary-soft);
  color: var(--lumina-primary);
}

.badge.muted {
  opacity: 0.65;
}

.model-editor {
  background: var(--lumina-card-bg);
  border-top: 1px solid var(--lumina-card-border);
  padding: 0 14px 14px;
}

.model-editor__toolbar {
  justify-content: space-between;
  padding-top: var(--lumina-gap-md);
}

.form-grid {
  display: grid;
  gap: 12px;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  margin: 12px 0;
}

.field {
  color: var(--lumina-text-secondary);
  display: flex;
  flex-direction: column;
  font-size: 12px;
  gap: 6px;
}

.field--wide {
  grid-column: 1 / -1;
}

input {
  background: var(--lumina-surface-2);
  border: 1px solid var(--lumina-card-border);
  border-radius: 6px;
  box-sizing: border-box;
  color: var(--lumina-text);
  height: 32px;
  padding: 0 10px;
}

.field-select {
  width: 100%;
}

:deep(.field-select .n-base-selection) {
  background: var(--lumina-surface-2);
  border-radius: 6px;
  min-height: 32px;
}

:deep(.field-select .n-base-selection-label) {
  color: var(--lumina-text);
}

.inline-toggle {
  color: var(--lumina-text-secondary);
  font-size: 12px;
}

:deep(.inline-toggle .n-checkbox__label) {
  color: var(--lumina-text-secondary);
  font-size: 12px;
}

.action-button,
.ghost-button {
  background: var(--lumina-button-secondary-bg);
  border: 1px solid var(--lumina-card-border);
  border-radius: 6px;
  color: var(--lumina-text);
  cursor: pointer;
  height: 30px;
  padding: 0 12px;
}

.action-button {
  background: var(--lumina-primary);
  border-color: var(--lumina-primary);
  color: var(--lumina-on-accent);
}

.danger {
  color: var(--lumina-danger);
}

.footer-actions {
  display: flex;
  flex: 0 0 auto;
  gap: 8px;
}

.footer-actions .ghost-button {
  white-space: nowrap;
}

.model-item__footer {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
}

.test-result {
  color: var(--lumina-text-secondary);
  font-size: 12px;
  line-height: 1.5;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.test-result.error {
  background: color-mix(in srgb, var(--lumina-danger) 8%, transparent);
  border: 1px solid color-mix(in srgb, var(--lumina-danger) 22%, transparent);
  border-radius: 6px;
  color: var(--lumina-danger);
  max-height: 64px;
  overflow: auto;
  overflow-wrap: anywhere;
  padding: 8px 10px;
  text-overflow: clip;
  white-space: normal;
}

@media (max-width: 720px) {
  .model-item__footer {
    grid-template-columns: 1fr;
  }

  .footer-actions {
    justify-content: flex-end;
  }
}
</style>
