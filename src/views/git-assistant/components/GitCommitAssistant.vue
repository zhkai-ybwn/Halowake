<template>
  <section class="commit-panel panel-shell">
    <header class="commit-header">
      <div>
        <div class="panel-eyebrow">{{ t('gitAssistant.ai.title') }}</div>
        <h3>Commit Message</h3>
      </div>
    </header>

    <div class="commit-flow" :class="{ 'commit-flow--ready': selectedCount > 0 }">
      <span class="commit-flow__step">1</span>
      <span>{{ selectedCount ? t('gitAssistant.ai.filesSelected', { count: selectedCount }) : t('gitAssistant.ai.selectFilesFirst') }}</span>
      <i aria-hidden="true"></i>
      <span class="commit-flow__step">2</span>
      <span>{{ t('gitAssistant.ai.writeMessageNext') }}</span>
    </div>

    <!-- AI Generation Toolbar (Input Assistant) -->
    <div class="ai-generate-bar">
      <!-- In-progress state: Stop button -->
      <button
        v-if="aiLoading"
        class="ai-gen-btn ai-gen-btn--loading"
        type="button"
        @click="$emit('cancelGenerate')"
      >
        <Icon icon="solar:stop-circle-linear" class="ai-gen-btn__icon" />
        <span>{{ t('gitAssistant.ai.stopGenerating') }}</span>
      </button>

      <!-- Existing content: Regenerate with Popconfirm overwrite guard -->
      <NPopconfirm
        v-else-if="hasExistingContent"
        :positive-text="t('common.ok')"
        :negative-text="t('common.cancel')"
        @positive-click="$emit('generate')"
      >
        <template #trigger>
          <button
            class="ai-gen-btn ai-gen-btn--regenerate"
            :disabled="!canGenerate"
            :title="!selectedCount ? t('gitAssistant.ai.noSelectedFiles') : ''"
            type="button"
          >
            <Icon icon="solar:refresh-linear" class="ai-gen-btn__icon" />
            <span>{{ t('gitAssistant.ai.regenerate') }}</span>
          </button>
        </template>
        {{ t('gitAssistant.ai.overwriteConfirm') }}
      </NPopconfirm>

      <!-- Initial empty state: Direct generate button -->
      <button
        v-else
        class="ai-gen-btn ai-gen-btn--initial"
        :disabled="!canGenerate"
        :title="!selectedCount ? t('gitAssistant.ai.noSelectedFiles') : ''"
        type="button"
        @click="$emit('generate')"
      >
        <Icon icon="solar:magic-stick-3-linear" class="ai-gen-btn__icon" />
        <span>{{ t('gitAssistant.ai.generate') }}</span>
      </button>

      <div v-if="currentModelLabel" class="ai-gen-meta">
        <span class="ai-gen-meta__model" :title="currentModelLabel">{{ currentModelLabel }}</span>
        <span v-if="currentLanguageLabel" class="ai-gen-meta__lang">{{ currentLanguageLabel }}</span>
      </div>
    </div>

    <div v-if="aiLoading" class="ai-gen-progress">
      <span class="ai-gen-progress__dot"></span>
      <span>{{ promptGenerationStep }}</span>
    </div>

    <div class="commit-form">
      <label class="field">
        <span class="field-label">{{ t('gitAssistant.ai.editTitle') }}</span>
        <input
          :value="title"
          class="field-input"
          type="text"
          @input="$emit('update:title', ($event.target as HTMLInputElement).value)"
        />
      </label>

      <label class="field field--body">
        <span class="field-label">{{ t('gitAssistant.ai.editBody') }}</span>
        <textarea
          :value="body"
          class="field-textarea"
          @input="$emit('update:body', ($event.target as HTMLTextAreaElement).value)"
        />
      </label>
    </div>

    <footer class="commit-actions">
      <div v-if="submitDisabled" class="hint-text">{{ t('gitAssistant.ai.submitDisabled') }}</div>
      <button
        class="action-btn primary commit-submit-btn"
        :disabled="submitDisabled || committing || pulling || pushing"
        type="button"
        @click="$emit('submit')"
      >
        <Icon icon="solar:check-circle-linear" class="commit-submit-icon" />
        <span>{{ committing ? t('gitAssistant.ai.submitting') : t('gitAssistant.ai.submitShortcut', { shortcut: shortcutLabel }) }}</span>
      </button>
    </footer>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted } from 'vue'
import { NPopconfirm } from 'naive-ui'
import { Icon } from '@iconify/vue'
import { useLocale } from '@/composables/useLocale'
import { formatPrimaryShortcut, hasPrimaryModifier } from '@/utils/platform-shortcuts'

const props = withDefaults(
  defineProps<{
    committing: boolean
    pushing: boolean
    pulling: boolean
    submitDisabled: boolean
    selectedCount: number
    title: string
    body: string
    aiLoading?: boolean
    canGenerate?: boolean
    promptGenerationStep?: string
    currentModelLabel?: string
    currentLanguageLabel?: string
  }>(),
  {
    aiLoading: false,
    canGenerate: false,
    promptGenerationStep: '',
    currentModelLabel: '',
    currentLanguageLabel: '',
  }
)

const emit = defineEmits<{
  (e: 'submit'): void
  (e: 'update:title', value: string): void
  (e: 'update:body', value: string): void
  (e: 'generate'): void
  (e: 'cancelGenerate'): void
}>()

const { t } = useLocale()

const hasExistingContent = computed(() => Boolean(props.title.trim() || props.body.trim()))
const shortcutLabel = computed(() => formatPrimaryShortcut('Enter'))

function handleSubmitShortcut(event: KeyboardEvent) {
  if (!hasPrimaryModifier(event) || event.key !== 'Enter') return
  if (event.defaultPrevented || document.querySelector('[aria-modal="true"]')) return
  if (props.submitDisabled || props.committing || props.pulling || props.pushing) return
  event.preventDefault()
  emit('submit')
}

onMounted(() => window.addEventListener('keydown', handleSubmitShortcut))
onUnmounted(() => window.removeEventListener('keydown', handleSubmitShortcut))
</script>

<style scoped lang="scss">
.panel-shell {
  background: var(--lumina-surface-1);
  overflow: hidden;
  position: relative;

  &::before {
    background: linear-gradient(180deg, color-mix(in srgb, var(--lumina-surface-1) 64%, transparent), transparent 68%);
    content: '';
    inset: 0;
    pointer-events: none;
    position: absolute;
  }
}

.commit-panel {
  display: grid;
  gap: 10px;
  grid-template-rows: auto auto auto auto 1fr auto;
  padding: 12px;

  > * {
    position: relative;
  }
}

.commit-flow {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-surface-2) 78%, transparent);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-sm);
  color: var(--lumina-text-secondary);
  display: flex;
  gap: 8px;
  min-height: 32px;
  padding: 0 10px;
  width: fit-content;

  i {
    background: var(--lumina-card-border);
    height: 1px;
    width: 18px;
  }
}

.commit-flow--ready {
  background: color-mix(in srgb, var(--lumina-primary-soft) 48%, var(--lumina-surface-2));
  color: var(--lumina-text);
}

.commit-flow__step {
  align-items: center;
  background: var(--lumina-surface-1);
  border: 0.5px solid var(--lumina-separator);
  border-radius: 999px;
  display: inline-flex;
  font-size: 11px;
  font-weight: 700;
  height: 20px;
  justify-content: center;
  width: 20px;
}

.commit-header {
  align-items: center;
  display: flex;
  gap: 10px;
  justify-content: flex-start;
}

.panel-eyebrow,
.field-label,
.hint-text {
  color: var(--lumina-text-secondary);
  font-size: 11px;
}

h3 {
  font-size: 17px;
  letter-spacing: 0;
  margin: 0;
}

/* AI Generation Toolbar */
.ai-generate-bar {
  align-items: center;
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  justify-content: space-between;
  margin-top: 2px;
}

.ai-gen-btn {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-primary) 10%, var(--lumina-surface-2));
  border: 1px solid color-mix(in srgb, var(--lumina-primary) 28%, transparent);
  border-radius: var(--lumina-radius-sm);
  color: var(--lumina-primary);
  cursor: pointer;
  display: inline-flex;
  font-size: 12px;
  font-weight: 600;
  gap: 6px;
  height: 30px;
  padding: 0 12px;
  transition: all 0.18s ease;
  user-select: none;

  &:hover:not(:disabled) {
    background: color-mix(in srgb, var(--lumina-primary) 18%, var(--lumina-surface-2));
    border-color: color-mix(in srgb, var(--lumina-primary) 45%, transparent);
  }

  &:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }
}

.ai-gen-btn--loading {
  background: color-mix(in srgb, var(--lumina-danger, #ef4444) 12%, var(--lumina-surface-2));
  border-color: color-mix(in srgb, var(--lumina-danger, #ef4444) 30%, transparent);
  color: var(--lumina-danger, #ef4444);

  &:hover:not(:disabled) {
    background: color-mix(in srgb, var(--lumina-danger, #ef4444) 20%, var(--lumina-surface-2));
  }
}

.ai-gen-btn__icon {
  font-size: 15px;
}

.ai-gen-meta {
  align-items: center;
  color: var(--lumina-text-secondary);
  display: inline-flex;
  font-size: 11px;
  gap: 6px;
  margin-left: auto;
}

.ai-gen-meta__model {
  background: color-mix(in srgb, var(--lumina-surface-3, var(--lumina-surface-2)) 70%, transparent);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-sm);
  font-size: 11px;
  max-width: 140px;
  overflow: hidden;
  padding: 2px 6px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.ai-gen-meta__lang {
  color: var(--lumina-text-secondary);
  font-size: 11px;
}

.ai-gen-progress {
  align-items: center;
  color: var(--lumina-primary);
  display: flex;
  font-size: 11px;
  gap: 7px;
  margin-top: -2px;
  min-height: 18px;
}

.ai-gen-progress__dot {
  animation: ai-progress-pulse 1s ease-in-out infinite;
  background: var(--lumina-primary);
  border-radius: 999px;
  height: 6px;
  width: 6px;
}

@keyframes ai-progress-pulse {
  0%,
  100% {
    opacity: 0.35;
    transform: scale(0.85);
  }
  50% {
    opacity: 1;
    transform: scale(1);
  }
}

.commit-form {
  display: grid;
  gap: 8px;
}

.field {
  display: grid;
  gap: 6px;
}

.field-input,
.field-textarea {
  background: var(--lumina-input-bg);
  border: 0.5px solid var(--lumina-input-border);
  border-radius: var(--lumina-radius-sm);
  box-sizing: border-box;
  color: var(--lumina-text);
  padding: 10px 12px;
  width: 100%;

  &:focus {
    border-color: var(--lumina-input-focus);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--lumina-input-focus) 18%, transparent);
    outline: none;
  }
}

.field-input {
  min-height: 46px;
}

.field-textarea {
  line-height: 1.55;
  min-height: clamp(140px, 20vh, 240px);
  resize: vertical;
}

/* Commit Actions Footer */
.commit-actions {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-top: 4px;
}

.hint-text {
  color: var(--lumina-text-secondary);
  font-size: 11px;
  line-height: 1.4;
  text-align: center;
}

.commit-submit-btn {
  align-items: center;
  background: var(--lumina-primary);
  border: 1px solid var(--lumina-primary);
  border-radius: var(--lumina-radius-sm);
  box-shadow: 0 1px 2px color-mix(in srgb, var(--lumina-primary) 30%, transparent);
  color: var(--lumina-on-accent, #ffffff);
  cursor: pointer;
  display: flex;
  font-size: 13px;
  font-weight: 650;
  gap: 6px;
  height: 36px;
  justify-content: center;
  letter-spacing: 0.3px;
  padding: 0 16px;
  transition: all 0.18s ease;
  width: 100%;

  &:hover:not(:disabled) {
    background: color-mix(in srgb, var(--lumina-primary) 88%, white);
    box-shadow: 0 2px 6px color-mix(in srgb, var(--lumina-primary) 38%, transparent);
    transform: translateY(-0.5px);
  }

  &:active:not(:disabled) {
    transform: translateY(0);
  }

  &:disabled {
    box-shadow: none;
    cursor: not-allowed;
    opacity: 0.45;
  }
}

.commit-submit-icon {
  font-size: 16px;
}
</style>
