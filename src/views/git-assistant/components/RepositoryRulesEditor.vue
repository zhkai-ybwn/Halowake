<template>
  <section class="repository-rules-editor">
    <div v-if="!repoPath" class="empty-state">
      {{ t('gitAssistant.repositoryRules.noRepo') }}
    </div>

    <template v-else>
      <div class="repository-rules-toolbar">
        <div class="repository-rules-meta">
          <span>{{ t('gitAssistant.repositoryRules.profilePath') }}</span>
          <strong :title="profile?.profilePath || profileFilePath">
            {{ profile?.profilePath || profileFilePath }}
          </strong>
        </div>
        <div class="repository-rules-actions">
          <NButton size="small" :disabled="loading || saving" @click="loadProfile">
            {{ loading ? t('gitAssistant.repositoryRules.loading') : t('gitAssistant.repositoryRules.reload') }}
          </NButton>
          <NButton
            size="small"
            type="primary"
            :disabled="loading || saving || !content.trim()"
            :loading="saving"
            @click="saveProfile"
          >
            {{ saving ? t('gitAssistant.repositoryRules.saving') : t('common.save') }}
          </NButton>
        </div>
      </div>

      <div v-if="notice" class="notice" role="status" aria-live="polite">{{ notice }}</div>
      <div v-if="error" class="error" role="alert">{{ error }}</div>

      <label class="repository-rules-field">
        <span>{{ t('gitAssistant.repositoryRules.editorLabel') }}</span>
        <textarea
          v-model="content"
          class="json-editor"
          spellcheck="false"
          :placeholder="t('gitAssistant.repositoryRules.placeholder')"
        />
      </label>
    </template>
  </section>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { NButton } from 'naive-ui'
import { useI18n } from 'vue-i18n'
import {
  ensureGitProjectProfile,
  saveGitProjectProfile,
} from '@/services/git/git-profile-service'
import type { GitProjectProfileFile } from '@/types/git-profile'

const props = defineProps<{
  repoPath: string
}>()

const { t } = useI18n({ useScope: 'global' })
const content = ref('')
const loading = ref(false)
const saving = ref(false)
const error = ref('')
const notice = ref('')
const profile = ref<GitProjectProfileFile | null>(null)

const profileFilePath = computed(() => {
  const separator = props.repoPath.includes('\\') ? '\\' : '/'
  return `${props.repoPath.replace(/[\\/]+$/, '')}${separator}.lumina${separator}git-profile.json`
})

watch(
  () => props.repoPath,
  async (repoPath) => {
    content.value = ''
    profile.value = null
    error.value = ''
    notice.value = ''
    if (repoPath) await loadProfile()
  },
  { immediate: true },
)

async function loadProfile() {
  if (!props.repoPath) return

  loading.value = true
  error.value = ''
  notice.value = ''

  try {
    const result = await ensureGitProjectProfile(props.repoPath)
    profile.value = result
    content.value = result.content
    notice.value = result.created
      ? t('gitAssistant.repositoryRules.created')
      : t('gitAssistant.repositoryRules.loaded')
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
  } finally {
    loading.value = false
  }
}

async function saveProfile() {
  if (!props.repoPath) return

  error.value = ''
  notice.value = ''

  try {
    JSON.parse(content.value)
  } catch {
    error.value = t('gitAssistant.repositoryRules.invalidJson')
    return
  }

  saving.value = true
  try {
    const result = await saveGitProjectProfile({
      repoPath: props.repoPath,
      content: content.value,
    })
    profile.value = result
    content.value = result.content
    notice.value = t('gitAssistant.repositoryRules.saved')
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
  } finally {
    saving.value = false
  }
}
</script>

<style scoped lang="scss">
.repository-rules-editor {
  display: flex;
  flex-direction: column;
  gap: 12px;
  height: 100%;
  min-height: 0;
  padding: 16px;
}

.repository-rules-toolbar {
  align-items: center;
  display: flex;
  gap: 16px;
  justify-content: space-between;
}

.repository-rules-meta {
  min-width: 0;

  span {
    color: var(--lumina-text-secondary);
    display: block;
    font-size: 11px;
    margin-bottom: 3px;
  }

  strong {
    display: block;
    font-family: "JetBrains Mono", SFMono-Regular, Consolas, monospace;
    font-size: 12px;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
}

.repository-rules-actions {
  display: flex;
  flex: 0 0 auto;
  gap: 8px;
}

.empty-state,
.notice,
.error {
  border-radius: var(--lumina-radius-sm);
  font-size: 12px;
  line-height: 1.5;
  padding: 10px 12px;
}

.empty-state {
  background: var(--lumina-empty-bg);
  border: 1px dashed var(--lumina-empty-border);
  color: var(--lumina-text-secondary);
}

.notice {
  background: var(--lumina-primary-soft);
  border: 0.5px solid color-mix(in srgb, var(--lumina-primary) 28%, transparent);
  color: var(--lumina-primary);
}

.error {
  background: color-mix(in srgb, var(--lumina-danger) 10%, var(--lumina-surface-1));
  border: 0.5px solid color-mix(in srgb, var(--lumina-danger) 28%, transparent);
  color: var(--lumina-danger);
}

.repository-rules-field {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 6px;
  min-height: 0;

  > span {
    color: var(--lumina-text-secondary);
    font-size: 11px;
  }
}

.json-editor {
  background: var(--lumina-diff-bg);
  border: 0.5px solid var(--lumina-card-border);
  border-radius: var(--lumina-radius-md);
  color: var(--lumina-text);
  flex: 1;
  font-family: "JetBrains Mono", SFMono-Regular, Consolas, monospace;
  font-size: 12px;
  line-height: 1.65;
  min-height: 360px;
  outline: none;
  padding: 14px;
  resize: none;
  width: 100%;

  &:focus-visible {
    border-color: var(--lumina-primary);
    box-shadow: 0 0 0 3px var(--lumina-accent-ring);
  }
}
</style>
