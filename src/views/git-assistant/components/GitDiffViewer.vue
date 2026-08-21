<template>
  <section class="diff-viewer panel-shell">
    <header class="diff-header">
      <div class="diff-header__main">
        <div class="panel-eyebrow">{{ t('gitAssistant.detail.eyebrow') }}</div>
        <h2>{{ activeFile ? activeFile.fileName : t('gitAssistant.detail.emptyTitle') }}</h2>
        <div v-if="activeFile" class="meta-row">
          <span class="meta-pill" :class="`tone-${statusMeta[activeFile.type].tone}`">
            {{ t(statusMeta[activeFile.type].labelKey) }}
          </span>
          <span class="meta-pill">{{ activeFile.score === null ? '-' : t('gitAssistant.detail.score', { score: activeFile.score }) }}</span>
          <span v-if="activeFile.recommended" class="meta-pill meta-pill--accent">
            {{ t('gitAssistant.detail.reviewRecommended') }}
          </span>
        </div>
      </div>

      <div v-if="activeFile && hasToggle" class="diff-switches">
        <button
          v-if="currentMode === 'head'"
          class="switch-btn active"
          type="button"
          @click="$emit('update:mode', 'head')"
        >
          {{ t('gitAssistant.detail.previousView') }}
        </button>
        <button
          v-if="activeFile.staged"
          class="switch-btn"
          :class="{ active: currentMode === 'staged' }"
          type="button"
          @click="$emit('update:mode', 'staged')"
        >
          {{ t('gitAssistant.detail.stagedView') }}
        </button>
        <button
          v-if="activeFile.unstaged"
          class="switch-btn"
          :class="{ active: currentMode === 'unstaged' }"
          type="button"
          @click="$emit('update:mode', 'unstaged')"
        >
          {{ t('gitAssistant.detail.unstagedView') }}
        </button>
      </div>
    </header>

    <WorkbenchEmptyState v-if="!hasSnapshot" icon="solar:folder-open-linear" :title="t('gitAssistant.detail.emptyNoRepo')" />
    <WorkbenchEmptyState v-else-if="!activeFile" icon="solar:document-text-linear" :title="t('gitAssistant.detail.emptySelect')" />
    <WorkbenchEmptyState v-else-if="loading" icon="solar:refresh-circle-linear" :title="t('gitAssistant.detail.diffLoading')" />
    <WorkbenchEmptyState v-else-if="!diffText" icon="solar:check-circle-linear" :title="t('gitAssistant.detail.diffEmpty')" />

    <div v-else class="diff-body">
      <div class="diff2html-host" v-html="diffHtml"></div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useLocale } from '@/hooks/useLocale'
import { STATUS_META } from '../git-assistant.config'
import type { GitAssistantFileView } from '../git-assistant.types'
import WorkbenchEmptyState from '@/components/workbench/WorkbenchEmptyState.vue'
import 'diff2html/bundles/css/diff2html.min.css'

const props = defineProps<{
  hasSnapshot: boolean
  activeFile: GitAssistantFileView | null
  diffText: string
  loading: boolean
  currentMode: 'head' | 'staged' | 'unstaged'
}>()

defineEmits<{
  (e: 'update:mode', value: 'head' | 'staged' | 'unstaged'): void
}>()

const statusMeta = STATUS_META
const { t } = useLocale()

const hasToggle = ref(false)
const diffHtml = ref('')

watch(
  () => [props.currentMode, props.activeFile?.staged, props.activeFile?.unstaged] as const,
  () => {
    hasToggle.value = Boolean(props.currentMode === 'head' || (props.activeFile?.staged && props.activeFile?.unstaged))
  },
  { immediate: true },
)

watch(
  () => props.diffText,
  async diffText => {
    if (!diffText) {
      diffHtml.value = ''
      return
    }

    const { html } = await import('diff2html')
    diffHtml.value = html(diffText, {
      drawFileList: false,
      matching: 'lines',
      outputFormat: 'line-by-line',
    })
  },
  { immediate: true },
)
</script>

<style scoped lang="scss">
.panel-shell {
  background: var(--lumina-surface-1);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-lg);
  box-shadow: 0 0 0 0.5px color-mix(in srgb, var(--lumina-text) 4%, transparent), 0 1px 2px rgb(0 0 0 / 5%);
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  min-height: 0;
  overflow: hidden;
}

.diff-header {
  align-items: flex-start;
  border-bottom: 0.5px solid var(--lumina-separator);
  display: flex;
  gap: 12px;
  justify-content: space-between;
  padding: 12px 14px;
}

.panel-eyebrow {
  color: var(--lumina-text-secondary);
  font-size: 11px;
  margin-bottom: 4px;
}

h2 {
  font-size: 16px;
  margin: 0;
}

.meta-row {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 6px;
}

.meta-pill,
.switch-btn {
  align-items: center;
  background: var(--lumina-surface-3);
  border: 1px solid var(--lumina-card-border);
  border-radius: 999px;
  display: inline-flex;
  font-size: 11px;
  height: 24px;
  padding: 0 8px;
}

.meta-pill--accent,
.switch-btn.active {
  background: color-mix(in srgb, var(--lumina-primary-soft) 58%, var(--lumina-surface-2));
  border-color: var(--lumina-primary);
  color: var(--lumina-primary);
}

.diff-switches {
  display: flex;
  gap: 8px;
}

.switch-btn {
  cursor: pointer;
  transition:
    background 0.18s ease,
    border-color 0.18s ease,
    color 0.18s ease;

  &:hover {
    background: var(--lumina-button-secondary-hover);
  }
}

.panel-empty {
  align-items: center;
  background: var(--lumina-empty-bg);
  border: 1px dashed var(--lumina-empty-border);
  border-radius: var(--lumina-radius-md);
  color: var(--lumina-text-secondary);
  display: flex;
  font-size: 12px;
  justify-content: center;
  margin: 14px;
  text-align: center;
}

.diff-body {
  background: var(--lumina-diff-bg);
  min-height: 0;
  overflow: auto;
  padding: 0;
}

.diff2html-host {
  min-width: max-content;

  :deep(.d2h-wrapper) {
    color: var(--lumina-text);
    font-family: SFMono-Regular, Consolas, 'Liberation Mono', Menlo, monospace;
  }

  :deep(.d2h-file-wrapper) {
    border: 0;
    border-radius: 0;
    margin-bottom: 0;
  }

  :deep(.d2h-file-header) {
    background: var(--lumina-surface-2);
    border-bottom: 1px solid var(--lumina-card-border);
    color: var(--lumina-text-secondary);
    font-family: inherit;
    height: 34px;
  }

  :deep(.d2h-code-line),
  :deep(.d2h-code-side-line),
  :deep(.d2h-code-linenumber),
  :deep(.d2h-code-side-linenumber) {
    font-family: SFMono-Regular, Consolas, 'Liberation Mono', Menlo, monospace;
    font-size: 12px;
    line-height: 1.65;
  }

  :deep(.d2h-code-linenumber),
  :deep(.d2h-code-side-linenumber) {
    background: var(--lumina-surface-2);
    color: var(--lumina-text-secondary);
  }

  :deep(.d2h-ins) {
    background: var(--lumina-diff-add-bg);
  }

  :deep(.d2h-del) {
    background: var(--lumina-diff-remove-bg);
  }

  :deep(.d2h-info) {
    background: var(--lumina-diff-hunk-bg);
    color: var(--lumina-diff-hunk-text);
  }
}

.tone-warning {
  color: var(--lumina-warning);
}

.tone-success {
  color: var(--lumina-success);
}

.tone-danger {
  color: var(--lumina-danger);
}

.tone-info {
  color: var(--lumina-primary);
}

.mono {
  font-family: SFMono-Regular, Consolas, 'Liberation Mono', Menlo, monospace;
}
</style>
