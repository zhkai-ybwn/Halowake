<template>
  <main class="git-diff-page">
    <header class="git-diff-header" data-tauri-drag-region>
      <div>
        <strong>{{ title }}</strong>
        <span>{{ request?.filePath || '' }}</span>
      </div>
      <small>{{ comparisonLabel }}</small>
    </header>

    <WorkbenchEmptyState v-if="loading" icon="solar:refresh-circle-linear" :title="t('gitDiff.loading')" />
    <WorkbenchEmptyState v-else-if="error" class="git-diff-state--error" icon="solar:danger-triangle-linear" :title="error" />
    <WorkbenchEmptyState v-else-if="!diffText" icon="solar:check-circle-linear" :title="t('gitDiff.empty')" />
    <UnifiedDiffViewer v-else :diff="diffText" />
  </main>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { emit, listen } from '@tauri-apps/api/event'
import {
  loadGitCommitFileDiff,
  loadGitFileDiff,
  loadGitFileHeadDiff,
} from '@/services/git/git-service'
import type { GitDiffRequest } from '@/services/git/git-diff-window'
import UnifiedDiffViewer from './UnifiedDiffViewer.vue'
import { useLocale } from '@/hooks/useLocale'
import WorkbenchEmptyState from '@/components/workbench/WorkbenchEmptyState.vue'

const request = ref<GitDiffRequest | null>(null)
const loading = ref(false)
const error = ref('')
const diffText = ref('')
const { t } = useLocale()

const title = computed(() => request.value?.filePath.split(/[/\\]/).pop() || t('gitDiff.title'))
const comparisonLabel = computed(() => {
  if (!request.value) return ''
  if (request.value.kind === 'commit') return request.value.hash.slice(0, 12)
  return request.value.mode === 'head'
    ? t('gitDiff.comparisonHead')
    : request.value.mode === 'staged'
      ? t('gitDiff.comparisonStaged')
      : t('gitDiff.comparisonUnstaged')
})

async function loadDiff(nextRequest: GitDiffRequest) {
  request.value = nextRequest
  loading.value = true
  error.value = ''
  diffText.value = ''

  try {
    const result = nextRequest.kind === 'commit'
      ? await loadGitCommitFileDiff(nextRequest.repoPath, nextRequest.hash, nextRequest.filePath, true)
      : nextRequest.mode === 'head'
        ? await loadGitFileHeadDiff(nextRequest.repoPath, nextRequest.filePath, true)
        : await loadGitFileDiff({ repoPath: nextRequest.repoPath, filePath: nextRequest.filePath, staged: nextRequest.mode === 'staged', fullContext: true })
    diffText.value = result.diff
  } catch (err) {
    error.value = err instanceof Error ? err.message : t('gitDiff.loadFailed')
  } finally {
    loading.value = false
  }
}

let unlistenInit: (() => void) | null = null

onMounted(async () => {
  unlistenInit = await listen<GitDiffRequest>('git-diff-init', event => void loadDiff(event.payload))
  await emit('git-diff-request-init')
})

onUnmounted(() => unlistenInit?.())
</script>

<style scoped lang="scss">
.git-diff-page {
  background: var(--lumina-surface-1);
  color: var(--lumina-text);
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  height: 100vh;
  inset: 0;
  overflow: hidden;
  position: fixed;
  width: 100%;
}

.git-diff-header {
  align-items: center;
  background: var(--lumina-toolbar-bg);
  backdrop-filter: var(--lumina-vibrancy);
  border-bottom: 0.5px solid var(--lumina-separator);
  display: flex;
  justify-content: space-between;
  min-width: 0;
  min-height: 50px;
  padding: 7px 14px;
}

.git-diff-header div {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.git-diff-header strong {
  font-size: 13px;
  font-weight: 650;
}

.git-diff-header span,
.git-diff-header small {
  color: var(--lumina-text-secondary);
  font-size: 11px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.git-diff-state {
  align-items: center;
  color: var(--lumina-text-secondary);
  display: flex;
  justify-content: center;
  padding: 24px;
}

.git-diff-state--error { color: var(--lumina-danger); }
</style>
