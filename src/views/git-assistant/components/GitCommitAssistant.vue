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
      <span v-if="submitDisabled" class="hint-text">{{ t('gitAssistant.ai.submitDisabled') }}</span>
      <button class="action-btn primary" :disabled="submitDisabled || committing || pulling || pushing" type="button" @click="$emit('submit')">
        {{ committing ? t('gitAssistant.ai.submitting') : t('gitAssistant.ai.submit') }}
      </button>
    </footer>
  </section>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'
import { useLocale } from '@/hooks/useLocale'
import { hasPrimaryModifier } from '@/utils/platform-shortcuts'

const props = defineProps<{
  committing: boolean
  pushing: boolean
  pulling: boolean
  submitDisabled: boolean
  selectedCount: number
  title: string
  body: string
}>()

const emit = defineEmits<{
  (e: 'submit'): void
  (e: 'update:title', value: string): void
  (e: 'update:body', value: string): void
}>()

const { t } = useLocale()

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
  gap: 8px;
  grid-template-rows: auto auto auto auto;
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

.commit-header,
.commit-actions {
  align-items: center;
  display: flex;
  gap: 10px;
}

.commit-header {
  justify-content: flex-start;
}

.commit-actions {
  justify-content: flex-end;
}

.panel-eyebrow,
.field-label,
.hint-text {
  color: var(--lumina-text-secondary);
  font-size: 11px;
}

.hint-text {
  margin-right: auto;
}

h3 {
  font-size: 17px;
  letter-spacing: 0;
  margin: 0;
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
  min-height: clamp(160px, 22vh, 260px);
  resize: vertical;
}

.action-btn {
  background: var(--lumina-button-secondary-bg);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-sm);
  color: var(--lumina-text);
  cursor: pointer;
  font-weight: 600;
  height: 32px;
  min-width: 86px;
  padding: 0 14px;
  transition:
    background 0.18s ease,
    border-color 0.18s ease,
    color 0.18s ease;

  &:hover:not(:disabled) {
    background: color-mix(in srgb, var(--lumina-button-secondary-hover) 82%, transparent);
  }

  &:disabled {
    cursor: not-allowed;
    opacity: 0.55;
  }
}

.primary {
  background: var(--lumina-primary);
  border-color: var(--lumina-primary);
  box-shadow: none;
  color: var(--lumina-on-accent);
  min-width: 108px;
}

</style>
