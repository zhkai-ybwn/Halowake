<template>
  <section class="editor-detail-panel">
    <header class="panel-header">
      <div class="panel-title-group">
        <span class="panel-eyebrow">{{ t('codexReport.editable') }}</span>
        <h3>{{ t('codexReport.workRecord') }}</h3>
      </div>

      <!-- macOS Center Segmented Switcher -->
      <div class="segmented-control" role="tablist">
        <button
          type="button"
          role="tab"
          :class="{ active: editorMode === 'markdown' }"
          @click="emit('update:editorMode', 'markdown')"
        >
          Markdown
        </button>
        <button
          type="button"
          role="tab"
          :class="{ active: editorMode === 'preview' }"
          @click="emit('update:editorMode', 'preview')"
        >
          {{ t('codexReport.preview') }}
        </button>
      </div>

      <!-- Header Action Buttons -->
      <div class="editor-actions">
        <WorkbenchButton
          :disabled="!workRecord"
          @click="copyText(workRecord, t('codexReport.workRecordCopied'))"
        >
          <Icon icon="solar:copy-linear" />
          {{ t('codexReport.copyWorkRecord') }}
        </WorkbenchButton>
        <WorkbenchButton
          variant="primary"
          :disabled="!workRecord"
          @click="copyText(combinedContent, t('codexReport.combinedCopied'))"
        >
          <Icon icon="solar:copy-linear" />
          {{ t('codexReport.copyForAi') }}
        </WorkbenchButton>
      </div>
    </header>

    <!-- Editor & Preview Body -->
    <div class="editor-body">
      <WorkbenchEmptyState
        v-if="!workRecord"
        icon="solar:document-linear"
        :title="t('codexReport.noWorkRecord')"
        :description="t('codexReport.editorPlaceholder')"
      />
      <template v-else>
        <NInput
          v-if="editorMode === 'markdown'"
          :value="workRecord"
          type="textarea"
          :autosize="false"
          class="macos-report-editor"
          :placeholder="t('codexReport.editorPlaceholder')"
          @update:value="emit('update:workRecord', $event)"
        />
        <article v-else class="macos-markdown-preview" v-html="markdownPreview"></article>
      </template>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { marked } from 'marked'
import { NInput, useMessage } from 'naive-ui'
import WorkbenchButton from '@/components/workbench/WorkbenchButton.vue'
import WorkbenchEmptyState from '@/components/workbench/WorkbenchEmptyState.vue'
import { useLocale } from '@/composables/useLocale'

const props = defineProps<{
  workRecord: string
  combinedContent: string
  editorMode: 'markdown' | 'preview'
}>()

const emit = defineEmits<{
  (e: 'update:workRecord', value: string): void
  (e: 'update:editorMode', value: 'markdown' | 'preview'): void
}>()

const { t } = useLocale()
const message = useMessage()

const markdownPreview = computed(() => {
  if (props.editorMode !== 'preview') return ''
  return marked.parse(escapeHtml(props.workRecord || t('codexReport.noWorkRecord')), { async: false })
})

function escapeHtml(value: string) {
  return value.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;')
}

async function copyText(text: string, successMessage: string) {
  try {
    await navigator.clipboard.writeText(text)
    message.success(successMessage)
  } catch (error) {
    message.error(t('codexReport.copyFailed', { error: error instanceof Error ? error.message : String(error) }))
  }
}
</script>

<style scoped lang="scss">
.editor-detail-panel {
  background: var(--lumina-surface-2);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-md);
  box-shadow: var(--lumina-shadow-sm);
  display: flex;
  flex: 1;
  flex-direction: column;
  min-height: 0;
  min-width: 0;
  overflow: hidden;
}

.panel-header {
  align-items: center;
  border-bottom: 0.5px solid var(--lumina-separator);
  display: flex;
  flex: 0 0 auto;
  justify-content: space-between;
  padding: 8px 12px;
}

.panel-title-group {
  display: flex;
  flex-direction: column;
  gap: 1px;

  .panel-eyebrow {
    color: var(--lumina-text-tertiary);
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.05em;
    text-transform: uppercase;
  }

  h3 {
    color: var(--lumina-text);
    font-size: 13px;
    font-weight: 600;
    margin: 0;
  }
}

.segmented-control {
  background: var(--lumina-surface-3);
  border-radius: var(--lumina-radius-xs);
  display: flex;
  gap: 2px;
  padding: 2px;

  button {
    background: transparent;
    border: none;
    border-radius: var(--lumina-radius-xs);
    color: var(--lumina-text-secondary);
    cursor: pointer;
    font-size: 11px;
    font-weight: 500;
    padding: 3px 12px;
    text-align: center;
    transition: all var(--lumina-duration-fast) var(--lumina-ease-out);

    &:hover {
      color: var(--lumina-text);
    }

    &.active {
      background: var(--lumina-surface-elevated);
      box-shadow: var(--lumina-shadow-xs);
      color: var(--lumina-primary);
      font-weight: 600;
    }
  }
}

.editor-actions {
  display: flex;
  gap: 6px;
}

.editor-body {
  display: flex;
  flex: 1;
  min-height: 0;
  overflow: hidden;
  padding: 10px 12px;
}

.macos-report-editor {
  background: var(--lumina-surface-elevated);
  border-radius: var(--lumina-radius-sm);
  flex: 1;
  height: 100%;
}

.editor-detail-panel :deep(.macos-report-editor .n-input-wrapper),
.editor-detail-panel :deep(.macos-report-editor .n-input__textarea-el) {
  height: 100%;
}

.editor-detail-panel :deep(textarea) {
  font-family: var(--lumina-font-mono, ui-monospace, SFMono-Regular, Consolas, monospace);
  font-size: 12px;
  line-height: 1.7;
}

.macos-markdown-preview {
  background: var(--lumina-surface-elevated);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-sm);
  box-sizing: border-box;
  color: var(--lumina-text);
  flex: 1;
  font-size: 13px;
  line-height: 1.7;
  min-height: 0;
  overflow-y: auto;
  padding: 16px 20px;

  :deep(h1),
  :deep(h2),
  :deep(h3) {
    border-bottom: 0.5px solid var(--lumina-separator);
    color: var(--lumina-text);
    margin-top: 1.2em;
    padding-bottom: 0.3em;
  }

  :deep(h1) {
    font-size: 17px;
  }
  :deep(h2) {
    font-size: 15px;
  }
  :deep(h3) {
    font-size: 13.5px;
  }

  :deep(p),
  :deep(ul),
  :deep(ol) {
    margin: 0.6em 0;
  }

  :deep(li) {
    margin: 0.25em 0;
  }

  :deep(code) {
    background: var(--lumina-surface-3);
    border-radius: var(--lumina-radius-xs);
    color: var(--lumina-primary);
    font-family: var(--lumina-font-mono, monospace);
    font-size: 0.9em;
    padding: 2px 5px;
  }

  :deep(pre) {
    background: var(--lumina-surface-2);
    border: 0.5px solid var(--lumina-separator);
    border-radius: var(--lumina-radius-sm);
    overflow-x: auto;
    padding: 10px 14px;

    code {
      background: transparent;
      color: inherit;
      padding: 0;
    }
  }

  :deep(blockquote) {
    border-left: 3px solid var(--lumina-primary);
    color: var(--lumina-text-secondary);
    margin: 0.8em 0;
    padding-left: 12px;
  }
}
</style>
