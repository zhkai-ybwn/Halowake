<template>
  <section class="profile-panel">
    <header class="profile-header">
      <div>
        <h2>{{ t('settings.projectProfile.title') }}</h2>
        <p>{{ t('settings.projectProfile.description') }}</p>
      </div>
      <div class="profile-actions">
        <button class="secondary-btn" type="button" :disabled="!repoPath || loading" @click="loadProfile">
          {{ loading ? t('settings.projectProfile.loading') : t('settings.projectProfile.reload') }}
        </button>
        <button class="primary-btn" type="button" :disabled="!repoPath || saving || !content.trim()" @click="saveProfile">
          {{ saving ? t('settings.projectProfile.saving') : t('settings.save') }}
        </button>
      </div>
    </header>

    <div v-if="!repoPath" class="empty-state">
      {{ t('settings.projectProfile.noRepo') }}
    </div>

    <template v-else>
      <!-- <div class="profile-meta">
        <div>
          <span>{{ t('settings.projectProfile.currentRepo') }}</span>
          <strong>{{ repoPath }}</strong>
        </div>
        <div v-if="profile?.profilePath">
          <span>{{ t('settings.projectProfile.profilePath') }}</span>
          <strong>{{ profile.profilePath }}</strong>
        </div>
      </div> -->

      <div v-if="notice" class="notice">{{ notice }}</div>
      <div v-if="error" class="error">{{ error }}</div>

      <textarea
        v-model="content"
        class="json-editor"
        spellcheck="false"
        :placeholder="t('settings.projectProfile.placeholder')"
      />
    </template>
  </section>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { GIT_REPO_STORAGE_KEY } from '@/constants/git'
import {
  ensureGitProjectProfile,
  saveGitProjectProfile,
} from '@/services/git/git-profile-service'
import type { GitProjectProfileFile } from '@/types/git-profile'

const { t } = useI18n({ useScope: 'global' })

const repoPath = ref('')
const content = ref('')
const loading = ref(false)
const saving = ref(false)
const error = ref('')
const notice = ref('')
const profile = ref<GitProjectProfileFile | null>(null)

onMounted(async () => {
  repoPath.value = localStorage.getItem(GIT_REPO_STORAGE_KEY) ?? ''
  if (repoPath.value) {
    await loadProfile()
  }
})

async function loadProfile() {
  if (!repoPath.value) return

  loading.value = true
  error.value = ''
  notice.value = ''

  try {
    const result = await ensureGitProjectProfile(repoPath.value)
    profile.value = result
    content.value = result.content
    notice.value = result.created
      ? t('settings.projectProfile.created')
      : t('settings.projectProfile.loaded')
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
  } finally {
    loading.value = false
  }
}

async function saveProfile() {
  if (!repoPath.value) return

  error.value = ''
  notice.value = ''

  try {
    JSON.parse(content.value)
  } catch (err) {
    error.value = err instanceof Error ? err.message : t('settings.projectProfile.invalidJson')
    return
  }

  saving.value = true
  try {
    const result = await saveGitProjectProfile({
      repoPath: repoPath.value,
      content: content.value,
    })
    profile.value = result
    content.value = result.content
    notice.value = t('settings.projectProfile.saved')
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
  } finally {
    saving.value = false
  }
}
</script>

<style scoped lang="scss">
.profile-panel {
  display: flex;
  flex-direction: column;
  gap: 14px;
  max-width: 1040px;
}

.profile-header {
  align-items: flex-start;
  background: var(--lumina-surface-1);
  border: 1px solid var(--lumina-card-border);
  border-radius: 10px;
  display: flex;
  gap: 16px;
  justify-content: space-between;
  padding: 18px;

  h2 {
    font-size: 18px;
    margin: 0 0 6px;
  }

  p {
    color: var(--lumina-text-secondary);
    font-size: 13px;
    margin: 0;
  }
}

.profile-actions {
  display: flex;
  flex: 0 0 auto;
  gap: 8px;
}

.primary-btn,
.secondary-btn {
  border-radius: 8px;
  cursor: pointer;
  height: 34px;
  padding: 0 13px;

  &:disabled {
    cursor: not-allowed;
    opacity: 0.55;
  }
}

.primary-btn {
  background: var(--lumina-primary);
  color: var(--lumina-on-accent);
}

.secondary-btn {
  background: var(--lumina-button-secondary-bg);
  border: 1px solid var(--lumina-card-border);
  color: var(--lumina-text);
}

.empty-state,
.notice,
.error {
  border-radius: 8px;
  font-size: 13px;
  padding: 12px 14px;
}

.empty-state {
  background: var(--lumina-empty-bg);
  border: 1px dashed var(--lumina-empty-border);
  color: var(--lumina-text-secondary);
}

.notice {
  background: var(--lumina-primary-soft);
  border: 1px solid color-mix(in srgb, var(--lumina-primary) 28%, transparent);
  color: var(--lumina-primary);
}

.error {
  background: color-mix(in srgb, var(--lumina-danger) 10%, var(--lumina-surface-1));
  border: 1px solid color-mix(in srgb, var(--lumina-danger) 28%, transparent);
  color: var(--lumina-danger);
}

.profile-meta {
  background: var(--lumina-surface-2);
  border: 1px solid var(--lumina-card-border);
  border-radius: 8px;
  display: grid;
  gap: 8px;
  grid-template-columns: 1fr;
  padding: 12px 14px;

  div {
    min-width: 0;
  }

  span {
    color: var(--lumina-text-secondary);
    display: block;
    font-size: 11px;
    margin-bottom: 4px;
  }

  strong {
    color: var(--lumina-text);
    display: block;
    font-size: 13px;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
}

.json-editor {
  background: var(--lumina-diff-bg);
  border: 1px solid var(--lumina-card-border);
  border-radius: 10px;
  color: var(--lumina-text);
  font-family: "JetBrains Mono", SFMono-Regular, Consolas, monospace;
  font-size: 12px;
  line-height: 1.7;
  min-height: 520px;
  outline: none;
  padding: 14px;
  resize: vertical;
  width: 100%;

  &:focus {
    border-color: var(--lumina-primary);
    box-shadow: 0 0 0 2px var(--lumina-accent-ring);
  }
}
</style>
