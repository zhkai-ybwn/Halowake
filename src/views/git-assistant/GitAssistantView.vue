<template>
  <div class="git-assistant-page">
    <div v-if="error" class="error-banner" role="alert">
      <span>{{ error }}</span>
      <button type="button" :aria-label="t('common.dismiss')" @click="error = ''">
        <Icon icon="solar:close-circle-linear" />
      </button>
    </div>

    <GitStatusBar
      :repo-path="displayRepoPath"
      :branch="snapshot?.branch ?? ''"
      :loading="loading"
      :fetching="fetchLoading"
      :pushing="pushLoading"
      :pulling="pullLoading"
      :summary="summary"
      :recommended-count="recommendedFiles.length"
      :repository-state="snapshot?.repositoryState ?? null"
      :recent-repos="recentRepos"
      :has-snapshot="Boolean(snapshot)"
      @pick-directory="handleSelectDirectory"
      @refresh="handleRefresh"
      @sync-action="handleSyncAction"
      @manage-repos="recentRepoManagerOpen = true"
      @open-branch-selector="openBranchSelector"
      @open-merge="openMergeDialog"
      @clone-repository="openRepositorySetup('clone')"
      @init-repository="openRepositorySetup('init')"
    />

    <section class="workspace-body">
      <GitChangeExplorer
        class="change-table"
        :has-snapshot="Boolean(snapshot)"
        :loading="loading"
        :keyword="keyword"
        :status-filter="statusFilter"
        :recommended-only="recommendedOnly"
        :summary="summary"
        :groups="filteredFileGroups"
        :filtered-count="filteredFiles.length"
        :total-count="allFiles.length"
        :active-file-raw="activeFileRaw"
        :review-selected-raws="reviewSelectedRaws"
        :review-scoring="reviewScoring"
        :has-review-scores="reviewScores.size > 0"
        :review-score-progress="reviewScoreProgress"
        @update:keyword="keyword = $event"
        @update:status-filter="handleStatusFilterChange"
        @update:recommended-only="recommendedOnly = $event"
        @select-file="handleSelectFile"
        @open-diff="handleOpenDiff"
        @file-action="handleFileAction"
        @request-refresh="handleRefresh"
        @toggle-review-selection="toggleReviewSelection"
        @set-review-selection="setReviewSelection"
        @request-review-score="loadReviewScores"
      />

      <section class="commit-area">
        <GitCommitAssistant
          class="commit-workbench"
          :committing="commitLoading"
          :pushing="pushLoading"
          :pulling="pullLoading"
          :submit-disabled="!snapshot || !reviewSelectedRaws.length || !commitTitle.trim()"
          :selected-count="reviewSelectedRaws.length"
          :title="commitTitle"
          :body="commitBody"
          @submit="handleCommit"
          @update:title="commitTitle = $event"
          @update:body="commitBody = $event"
        />

        <aside class="commit-side">
          <div class="ai-panel-title">
            <span>{{ t('gitAssistant.ai.actionsTitle') }}</span>
            <strong>{{ reviewSelectedRaws.length }}</strong>
          </div>

          <details class="ai-settings">
            <summary>
              <span>{{ t('gitAssistant.ai.currentModel') }}</span>
              <strong>{{ selectedCommitModelLabel }} · {{ selectedCommitLanguageLabel }}</strong>
              <Icon icon="solar:alt-arrow-down-linear" />
            </summary>
            <section class="ai-tool-section">
              <label class="model-field">
                <span>{{ t('gitAssistant.ai.currentModel') }}</span>
                <NSelect
                  class="model-select"
                  :value="aiSettings.taskModelMap['commit-message'] || aiSettings.defaultModelId"
                  :options="modelSelectOptions"
                  :disabled="!aiSettings.enabledModels.length"
                  size="small"
                  :consistent-menu-width="false"
                  @update:value="value => aiSettings.setTaskModel('commit-message', String(value ?? ''))"
                />
              </label>

              <label class="model-field">
                <span>{{ t('gitAssistant.ai.commitLanguage') }}</span>
                <NSelect
                  class="model-select"
                  :value="commitLanguage"
                  :options="commitLanguageOptions"
                  size="small"
                  :consistent-menu-width="false"
                  @update:value="(value: string) => commitLanguage = value as 'en' | 'zh'"
                />
              </label>

              <NCheckbox v-model:checked="autoSendPromptToApi" class="ai-toggle">
                {{ t('gitAssistant.ai.autoSendPrompt') }}
              </NCheckbox>
            </section>
          </details>

          <section v-if="showRemoteTools" class="remote-tools">
            <div class="remote-tools__header">
              <span>{{ t('gitAssistant.remote.title') }}</span>
              <strong>{{ remoteToolStatus }}</strong>
            </div>
            <NInput
              v-if="needsRemoteUrl"
              v-model:value="remoteUrlDraft"
              size="small"
              clearable
              :placeholder="t('gitAssistant.remote.urlPlaceholder')"
            />
            <div class="remote-actions">
              <NButton
                v-if="isDiverged"
                size="small"
                :disabled="pullLoading || remoteLoading"
                @click="handlePull"
              >
                {{ t('gitAssistant.remote.mergeRemote') }}
              </NButton>
              <NButton
                v-if="isDiverged"
                size="small"
                type="primary"
                :disabled="rebaseLoading || remoteLoading"
                @click="handleRebase"
              >
                {{ t('gitAssistant.remote.rebaseRemote') }}
              </NButton>
              <NButton
                v-if="needsRemoteUrl"
                size="small"
                type="primary"
                :disabled="!remoteUrlDraft.trim() || remoteLoading"
                @click="handleConfigureOrigin"
              >
                {{ t('gitAssistant.remote.connectOrigin') }}
              </NButton>
              <NButton
                v-if="canRepairUpstream"
                size="small"
                :disabled="remoteLoading"
                @click="handleRepairUpstream"
              >
                {{ t('gitAssistant.remote.repairUpstream') }}
              </NButton>
              <NButton
                v-if="canPublishBranch"
                size="small"
                :disabled="pushLoading || remoteLoading"
                @click="handlePush"
              >
                {{ t('gitAssistant.remote.publishBranch') }}
              </NButton>
            </div>
            <p>{{ remoteToolHint }}</p>
          </section>

          <section v-if="showConflictTools" class="conflict-tools">
            <div class="conflict-tools__header">
              <span>{{ t('gitAssistant.conflict.title') }}</span>
              <strong>{{ t('gitAssistant.conflict.count', { count: conflictedFiles.length }) }}</strong>
            </div>
            <NButton size="small" type="primary" :disabled="conflictLoading" @click="openConflictDialog">
              {{ t('gitAssistant.conflict.resolve') }}
            </NButton>
          </section>

          <section class="ai-tool-section ai-tool-section--actions">
            <button class="ai-action primary-action" type="button" :disabled="!snapshot && !aiLoading" @click="aiLoading ? handleCancelAiAnalysis() : handleGenerateAiAnalysis()">
              {{ aiLoading ? t('gitAssistant.ai.stopGenerating') : t('gitAssistant.ai.generate') }}
            </button>
            <div class="ai-action-grid">
              <button class="ai-action" type="button" :disabled="!promptPreview" @click="promptDrawerOpen = true">
                {{ t('gitAssistant.ai.viewPrompt') }}
              </button>
              <button class="ai-action" type="button" :disabled="!filteredCommitMessageHistory.length" @click="historyDrawerOpen = true">
                {{ t('gitAssistant.history.open') }}
              </button>
              <button class="ai-action" type="button" :disabled="!snapshot" @click="handleOpenLog()">
                {{ t('gitAssistant.log.open') }}
              </button>
              <button class="ai-action" type="button" disabled>
                {{ t('gitAssistant.ai.reviewCode') }}
              </button>
            </div>
          </section>
          <div v-if="aiLoading" class="ai-progress">
            <span class="ai-progress__dot"></span>
            <span>{{ promptGenerationStep }}</span>
          </div>
        </aside>
      </section>

      <NModal v-model:show="showDiff" class="diff-modal" :mask-closable="true">
        <WorkbenchModalPanel size="diff" :close-label="t('gitAssistant.prompt.close')" @close="showDiff = false">
          <GitDiffViewer
            class="diff-window"
            :has-snapshot="Boolean(snapshot)"
            :active-file="selectedFile"
            :diff-text="currentDiff"
            :loading="diffLoading"
            :current-mode="diffMode"
            @update:mode="diffMode = $event"
          />
        </WorkbenchModalPanel>
      </NModal>

      <NModal
        v-model:show="recentRepoManagerOpen"
        class="recent-repo-modal"
        :auto-focus="false"
        :mask-closable="true"
        :trap-focus="false"
      >
        <section class="recent-repo-dialog">
          <button class="modal-close-button" type="button" :aria-label="t('gitAssistant.prompt.close')" @click="recentRepoManagerOpen = false">
            <Icon icon="solar:close-circle-linear" />
          </button>
          <header class="recent-repo-dialog__header">
            <div>
              <h3>{{ t('gitAssistant.repo.recentRepoManage') }}</h3>
              <p>{{ t('gitAssistant.repo.recentRepoManageHint') }}</p>
            </div>
          </header>

          <section v-if="recentRepos.length" class="recent-repo-list">
            <article v-for="repo in recentRepos" :key="repo.path" class="recent-repo-item">
              <div class="recent-repo-item__main">
                <div v-if="editingAliasPath === repo.path" class="recent-repo-alias-edit">
                  <input
                    :ref="el => setAliasInputRef(el, repo.path)"
                    class="recent-repo-alias-input"
                    type="text"
                    :value="repo.name"
                    :placeholder="t('gitAssistant.repo.recentRepoAliasPlaceholder')"
                    @input="event => renameRecentRepo(repo.path, (event.target as HTMLInputElement).value)"
                    @blur="finishEditAlias(repo)"
                    @keydown.enter="finishEditAlias(repo)"
                    @keydown.escape="cancelEditAlias()"
                    @click.stop
                    @mousedown.stop
                  />
                </div>
                <template v-else>
                  <div class="recent-repo-alias-text" :title="repo.name || repo.path">{{ repo.name || t('gitAssistant.repo.recentRepoAliasPlaceholder') }}</div>
                </template>
                <div class="recent-repo-path mono" :title="repo.path">{{ repo.path }}</div>
              </div>
              <div class="recent-repo-item__actions">
                <NButton v-if="editingAliasPath !== repo.path" size="small" quaternary @click.stop="startEditAlias(repo.path)">
                  {{ t('gitAssistant.repo.recentRepoRename') }}
                </NButton>
                <NButton v-else size="small" quaternary @click.stop="finishEditAlias(repo)">
                  {{ t('gitAssistant.repo.recentRepoRenameConfirm') }}
                </NButton>
                <NButton size="small" :disabled="normalizePath(repo.path) === normalizePath(displayRepoPath)" @click="handleSwitchRecentRepoFromManager(repo.path)">
                  {{ t('gitAssistant.repo.recentRepoSwitch') }}
                </NButton>
                <NButton size="small" quaternary type="error" @click="removeRecentRepo(repo.path)">
                  {{ t('gitAssistant.repo.recentRepoRemove') }}
                </NButton>
              </div>
            </article>
          </section>
          <div v-else class="recent-repo-empty">
            {{ t('gitAssistant.repo.recentRepoEmpty') }}
          </div>
        </section>
      </NModal>

      <NModal v-model:show="branchSelectorOpen" class="repository-action-modal" :auto-focus="false" :mask-closable="true" :trap-focus="false">
        <section class="repository-action-dialog branch-selector-dialog">
          <header>
            <div>
              <h3>{{ t('gitAssistant.repo.manageBranches') }}</h3>
            </div>
            <button class="modal-close-button" type="button" :aria-label="t('gitAssistant.prompt.close')" @click="branchSelectorOpen = false"><Icon icon="solar:close-circle-linear" /></button>
          </header>
          <NSelect
            v-model:value="branchSelectionValue"
            :options="branchOptions"
            :loading="branchLoading"
            filterable
            :placeholder="branchLoading ? t('gitAssistant.repo.loadingBranches') : t('gitAssistant.repo.selectBranch')"
            @update:value="value => handleBranchSelection(String(value ?? ''))"
          />
        </section>
      </NModal>

      <NModal v-model:show="conflictDialogOpen" class="conflict-modal" :auto-focus="false" :mask-closable="true" :trap-focus="false">
        <section class="conflict-dialog">
          <header class="conflict-dialog__header">
            <div>
              <h3>{{ t('gitAssistant.conflict.title') }}</h3>
              <p>{{ t('gitAssistant.conflict.dialogHint') }}</p>
            </div>
            <button class="modal-close-button" type="button" :aria-label="t('gitAssistant.prompt.close')" @click="conflictDialogOpen = false">
              <Icon icon="solar:close-circle-linear" />
            </button>
          </header>

          <div v-if="conflictedFiles.length" class="conflict-file-list">
            <div
              v-for="file in conflictedFiles"
              :key="file.path"
              class="conflict-file-row"
              :title="t('gitAssistant.conflict.openExternalHint')"
              @dblclick="handleOpenExternalFile(file.path)"
            >
              <NCheckbox
                :checked="conflictSelectedPaths.includes(file.path)"
                @update:checked="checked => toggleConflictSelection(file.path, checked)"
              />
              <span>{{ file.path }}</span>
              <small>{{ t('gitAssistant.conflict.doubleClickToOpen') }}</small>
            </div>
          </div>
          <div v-else class="conflict-dialog__empty">{{ t('gitAssistant.conflict.noFiles') }}</div>

          <footer class="conflict-dialog__footer">
            <div>
              <NButton
                v-if="conflictedFiles.length"
                type="primary"
                :disabled="!conflictSelectedPaths.length || conflictLoading"
                @click="handleMarkConflictPathsResolved"
              >
                {{ t('gitAssistant.conflict.markSelectedResolved') }}
              </NButton>
              <NButton
                v-if="repositoryState?.mergeInProgress"
                :disabled="conflictedFiles.length > 0 || conflictLoading"
                @click="handleContinueMerge"
              >
                {{ t('gitAssistant.conflict.continueMerge') }}
              </NButton>
              <NButton
                v-if="repositoryState?.rebaseInProgress"
                :disabled="conflictedFiles.length > 0 || conflictLoading"
                @click="handleContinueRebase"
              >
                {{ t('gitAssistant.conflict.continueRebase') }}
              </NButton>
            </div>
            <NButton
              type="error"
              tertiary
              :disabled="conflictLoading"
              @click="repositoryState?.rebaseInProgress ? handleAbortRebase() : handleAbortMerge()"
            >
              {{ repositoryState?.rebaseInProgress ? t('gitAssistant.conflict.abortRebase') : t('gitAssistant.conflict.abortMerge') }}
            </NButton>
          </footer>
        </section>
      </NModal>

      <NModal v-model:show="mergeDialogOpen" class="repository-action-modal" :auto-focus="false" :mask-closable="true" :trap-focus="false">
        <section class="repository-action-dialog merge-dialog">
          <header>
            <div>
              <h3>{{ t('gitAssistant.repo.mergeBranch') }}</h3>
              <p>{{ t('gitAssistant.repo.mergeInto', { branch: snapshot?.branch ?? '--' }) }}</p>
            </div>
            <button class="modal-close-button" type="button" :aria-label="t('gitAssistant.prompt.close')" @click="mergeDialogOpen = false"><Icon icon="solar:close-circle-linear" /></button>
          </header>
          <label class="repository-action-field">
            <span>{{ t('gitAssistant.repo.mergeSource') }}</span>
            <NSelect
              v-model:value="mergeSourceValue"
              :options="mergeSourceOptions"
              filterable
              :placeholder="t('gitAssistant.repo.mergeSourcePlaceholder')"
            />
          </label>
          <label class="repository-action-field">
            <span>{{ t('gitAssistant.repo.mergeMode') }}</span>
            <NSelect v-model:value="mergeMode" :options="mergeModeOptions" />
          </label>
          <p class="repository-action-hint">{{ t('gitAssistant.repo.mergeCleanWorktreeHint') }}</p>
          <footer class="repository-action-footer">
            <NButton @click="mergeDialogOpen = false">{{ t('common.dismiss') }}</NButton>
            <NButton type="primary" :loading="mergeLoading" :disabled="!mergeSourceValue" @click="handleMergeBranch">
              {{ t('gitAssistant.repo.mergeStart') }}
            </NButton>
          </footer>
        </section>
      </NModal>

      <NModal v-model:show="repositorySetupOpen" class="repository-action-modal" :auto-focus="false" :mask-closable="true" :trap-focus="false">
        <section class="repository-action-dialog">
          <header>
            <div>
              <h3>{{ repositorySetupMode === 'clone' ? t('gitAssistant.repo.cloneRepository') : t('gitAssistant.repo.initRepository') }}</h3>
              <p>{{ t('gitAssistant.repo.repositorySetupHint') }}</p>
            </div>
            <button class="modal-close-button" type="button" :aria-label="t('gitAssistant.prompt.close')" @click="repositorySetupOpen = false"><Icon icon="solar:close-circle-linear" /></button>
          </header>
          <NInput v-if="repositorySetupMode === 'clone'" v-model:value="cloneUrlDraft" :placeholder="t('gitAssistant.repo.cloneUrlPlaceholder')" />
          <div class="repository-path-picker">
            <NInput :value="repositoryPathDraft" readonly :placeholder="t('gitAssistant.repo.repositoryPathPlaceholder')" />
            <NButton @click="pickRepositoryTarget">{{ t('gitAssistant.repo.chooseDirectory') }}</NButton>
          </div>
          <div class="repository-action-footer">
            <NButton @click="repositorySetupOpen = false">{{ t('common.dismiss') }}</NButton>
            <NButton type="primary" :disabled="repositoryLoading || !repositoryPathDraft || (repositorySetupMode === 'clone' && !cloneUrlDraft.trim())" @click="handleRepositorySetup">
              {{ repositorySetupMode === 'clone' ? t('gitAssistant.repo.cloneRepository') : t('gitAssistant.repo.initRepository') }}
            </NButton>
          </div>
        </section>
      </NModal>
    </section>

    <WorkbenchDrawer
      v-if="promptDrawerOpen"
      size="wide"
      :title="t('gitAssistant.prompt.title')"
      :description="t('gitAssistant.prompt.description')"
      :close-label="t('gitAssistant.prompt.close')"
      @close="promptDrawerOpen = false"
    >
      <div v-if="promptPreview" class="prompt-drawer__body">
        <details class="prompt-section" open>
          <summary>{{ t('gitAssistant.prompt.overview') }}</summary>
          <section class="prompt-stats">
            <div>
              <span>{{ t('gitAssistant.prompt.selectedFiles') }}</span>
              <strong>{{ promptPreview.trace.selectedFiles.length }}</strong>
            </div>
            <div>
              <span>{{ t('gitAssistant.prompt.rawChars') }}</span>
              <strong>{{ promptPreview.trace.rawChars }}</strong>
            </div>
            <div>
              <span>{{ t('gitAssistant.prompt.cleanedChars') }}</span>
              <strong>{{ promptPreview.trace.cleanedChars }}</strong>
            </div>
            <div>
              <span>{{ t('gitAssistant.prompt.evidenceCount') }}</span>
              <strong>{{ promptPreview.trace.evidenceCount }}</strong>
            </div>
          </section>
        </details>

        <details class="prompt-section" open>
          <summary>{{ t('gitAssistant.prompt.rules') }}</summary>
          <section class="prompt-rules">
            <p>{{ t('gitAssistant.prompt.rulesHint') }}</p>
            <ol>
              <li v-for="rule in promptPreview.trace.rules" :key="rule">{{ rule }}</li>
            </ol>
          </section>
        </details>

        <details class="prompt-section" open>
          <summary>{{ t('gitAssistant.prompt.files') }}</summary>
          <section class="prompt-files">
            <div v-for="group in promptFileGroups" :key="group.kind" class="prompt-file-group">
              <div class="prompt-file-group__title">
                <strong>{{ group.kind }}</strong>
                <span>{{ group.files.length }}</span>
              </div>
              <div class="prompt-file-table wb-table">
                <div class="prompt-file-table__head wb-table-head">
                  <span>{{ t('gitAssistant.prompt.columnPath') }}</span>
                  <span>{{ t('gitAssistant.prompt.columnRole') }}</span>
                  <span>{{ t('gitAssistant.prompt.columnScope') }}</span>
                  <span>{{ t('gitAssistant.prompt.columnStrategy') }}</span>
                  <span>{{ t('gitAssistant.prompt.columnEvidence') }}</span>
                  <span>{{ t('gitAssistant.prompt.columnChars') }}</span>
                  <span>{{ t('gitAssistant.prompt.columnReason') }}</span>
                </div>
                <div v-for="file in group.files" :key="file.path" class="prompt-file-table__row">
                  <span class="mono" :title="file.path">{{ file.path }}</span>
                  <span>{{ file.role }}</span>
                  <span>{{ file.scope }}</span>
                  <span>{{ file.strategy }}</span>
                  <span>{{ file.evidenceCount }}</span>
                  <span>{{ file.cleanedChars }} / {{ file.rawChars }}</span>
                  <span :title="file.reason || ''">{{ file.reason || '-' }}</span>
                </div>
              </div>
            </div>
          </section>
        </details>

        <details class="prompt-section" open>
          <summary>{{ t('gitAssistant.prompt.finalPrompt') }}</summary>
          <section class="prompt-text">
            <textarea :value="promptPreview.prompt" readonly spellcheck="false"></textarea>
          </section>
        </details>
      </div>
    </WorkbenchDrawer>

    <WorkbenchDrawer
      v-if="historyDrawerOpen"
      :title="t('gitAssistant.history.title')"
      :description="t('gitAssistant.history.description')"
      :close-label="t('gitAssistant.prompt.close')"
      @close="historyDrawerOpen = false"
    >
      <section class="history-list">
        <article v-for="entry in filteredCommitMessageHistory" :key="entry.id" class="history-item">
          <div class="history-item__main">
            <span>{{ formatHistoryTime(entry.createdAt) }} 路 {{ historySourceLabel(entry.source) }}</span>
            <strong>{{ entry.title }}</strong>
            <p v-if="entry.body">{{ entry.body }}</p>
            <small>{{ entry.repoName }} 路 {{ t('gitAssistant.history.fileCount', { count: entry.selectedFileCount }) }}</small>
          </div>
          <button type="button" @click="restoreCommitMessage(entry)">{{ t('gitAssistant.history.restore') }}</button>
        </article>
        <div v-if="!filteredCommitMessageHistory.length" class="history-empty">
          {{ t('gitAssistant.history.empty') }}
        </div>
      </section>
    </WorkbenchDrawer>

    <GitCommandDialog
      :visible="gitCommandDialog.visible"
      :title="gitCommandDialog.title"
      :repo-path="displayRepoPath"
      :phase="gitCommandDialog.phase"
      :running="gitCommandDialog.running"
      :success="gitCommandDialog.success"
      :command="gitCommandDialog.command"
      :active-command="gitCommandDialog.activeCommand"
      :stdout="gitCommandDialog.stdout"
      :stderr="gitCommandDialog.stderr"
      :message="gitCommandDialog.message"
      :suggestion="gitCommandDialog.suggestion"
      :started-at="gitCommandDialog.startedAt"
      :finished-at="gitCommandDialog.finishedAt"
      :progress-percent="gitCommandDialog.progressPercent"
      :progress-phase="gitCommandDialog.progressPhase"
      :transfer="gitCommandDialog.transfer"
      :next-action-label="gitCommandDialog.nextActionLabel"
      :close-label="t('gitAssistant.gitCommand.close')"
      :abort-label="t('gitAssistant.gitCommand.abort')"
      @close="gitCommandDialog.visible = false"
      @next-action="handleCommandNextAction"
    />
  </div>
</template>
<script setup lang="ts">
import { computed, defineAsyncComponent, onActivated, onDeactivated, onMounted, onUnmounted, ref, watch } from 'vue'
import { NButton, NCheckbox, NInput, NModal, NSelect, type SelectOption } from 'naive-ui'
import { open } from '@tauri-apps/plugin-dialog'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useLocale } from '@/hooks/useLocale'
import {
  cloneGitRepository,
  checkoutGitRemoteBranch,
  initGitRepository,
  mergeGitBranch,
  openGitFileExternal,
  revertGitFile,
  stageGitFiles,
  scoreGitReviewFiles,
  switchGitBranch,
  unstageGitFiles,
} from '@/services/git/git-service'
import { openGitLogWindow } from '@/services/git/git-log-window'
import { openGitDiffWindow } from '@/services/git/git-diff-window'
import { useAiSettingsStore } from '@/stores/ai-settings'
import { parseGitStatusList } from '@/utils/git-status'
import GitChangeExplorer from './components/GitChangeExplorer.vue'
import GitCommandDialog from './components/GitCommandDialog.vue'
import GitCommitAssistant from './components/GitCommitAssistant.vue'
import GitStatusBar from './components/GitStatusBar.vue'
import WorkbenchDrawer from '@/components/workbench/WorkbenchDrawer.vue'
import WorkbenchModalPanel from '@/components/workbench/WorkbenchModalPanel.vue'
import { GIT_REPO_STORAGE_KEY } from './git-assistant.config'
import type {
  GitAssistantFileGroup,
  GitAssistantFileView,
  GitAssistantStatusFilter,
} from './git-assistant.types'
import { useGitSnapshot, useGitDiff, useGitRemote, useGitCommit } from '@/composables/git-assistant'
import {
  normalizePath,
  getFileName,
  getFileExtension,
  formatHistoryTime,
} from '@/composables/git-assistant/utils'
import type { GitFileStatus } from '@/types/git'

const GitDiffViewer = defineAsyncComponent(() => import('./components/GitDiffViewer.vue'))

const { t } = useLocale()
const aiSettings = useAiSettingsStore()

// ── Composables ──
const {
  loading, error, repoPath, snapshot, recentRepos, editingAliasPath,
  displayRepoPath, repositoryState, reviewSelectedRaws,
  loadRecentRepos, loadSnapshotByPath, handleSelectDirectory, handleRefresh,
  handleSwitchRecentRepo, renameRecentRepo, setAliasInputRef,
  startEditAlias, finishEditAlias, cancelEditAlias, removeRecentRepo,
} = useGitSnapshot()

function clearReviewSelection() { reviewSelectedRaws.value = [] }

const {
  currentDiff, diffMode, diffLoading, showDiff, activeFileRaw,
} = useGitDiff(() => displayRepoPath.value, (msg) => { error.value = msg })

const {
  fetchLoading, pushLoading, pullLoading, rebaseLoading, remoteLoading,
  conflictLoading, remoteUrlDraft, gitCommandDialog,
  handleConfigureOrigin, handleRepairUpstream, handlePush, handleFetch,
  handlePull, handleRebase, handleMarkResolved, handleAbortMerge,
  handleContinueMerge, handleContinueRebase, handleAbortRebase,
  handleCommandNextAction, startGitCommand, finishGitCommand, failGitCommand,
} = useGitRemote(
  () => displayRepoPath.value, () => repositoryState.value,
  (msg) => { error.value = msg }, loadSnapshotByPath, clearReviewSelection,
)

const {
  commitTitle, commitBody, commitLoading, aiLoading, promptPreview,
  promptDrawerOpen, historyDrawerOpen, promptGenerationStep, autoSendPromptToApi,
  commitMessageHistory, commitLanguage, loadCommitMessageHistory, restoreCommitMessage,
  handleGenerateAiAnalysis, handleCancelAiAnalysis, handleCommit,
} = useGitCommit(
  () => displayRepoPath.value, () => snapshot.value,
  () => selectedFileViews.value, () => conflictedFiles.value,
  () => reviewSelectedRaws.value, (msg) => { error.value = msg },
  startGitCommand, finishGitCommand, failGitCommand,
  loadSnapshotByPath, clearReviewSelection,
)

// ── UI-only state ──
const keyword = ref('')
const statusFilter = ref<GitAssistantStatusFilter>('all')
const recommendedOnly = ref(false)
const recentRepoManagerOpen = ref(false)
const branchLoading = ref(false)
const branchSelectorOpen = ref(false)
const branchSelectionValue = ref<string | null>(null)
const mergeDialogOpen = ref(false)
const mergeLoading = ref(false)
const mergeSourceValue = ref<string | null>(null)
const mergeMode = ref<'default' | 'no-ff'>('default')
const conflictDialogOpen = ref(false)
const conflictSelectedPaths = ref<string[]>([])
const repositorySetupOpen = ref(false)
const repositorySetupMode = ref<'init' | 'clone'>('init')
const repositoryLoading = ref(false)
const repositoryPathDraft = ref('')
const cloneUrlDraft = ref('')
const reviewScores = ref(new Map<string, { score: number; categories: string[]; eligible: boolean; skipped: boolean }>())
const reviewScoring = ref(false)
const reviewScoreProgress = ref({ completed: 0, total: 0, phase: '', filePath: '' })
let reviewScoreRequestId = 0
let unlistenReviewScoreProgress: UnlistenFn | null = null
let refreshOnNextActivation = false

// ── Computed ──
const parsedFiles = computed<GitFileStatus[]>(() => parseGitStatusList(snapshot.value?.status ?? []))

const allFiles = computed<GitAssistantFileView[]>(() => {
  const statsByPath = new Map((snapshot.value?.fileStats ?? []).map(stat => [normalizePath(stat.path), stat]))
  return parsedFiles.value.map(file => {
    const reviewScore = reviewScores.value.get(normalizePath(file.path))
    const stats = statsByPath.get(normalizePath(file.path))
    return {
      ...file,
      fileName: getFileName(file.path),
      directory: file.path.slice(0, Math.max(0, file.path.length - getFileName(file.path).length)).replace(/[\\/]$/, ''),
      extension: getFileExtension(file.path),
      addedLines: stats?.added ?? null,
      removedLines: stats?.removed ?? null,
      score: reviewScore && !reviewScore.skipped ? reviewScore.score : null,
      scoreCategories: reviewScore?.categories ?? [],
      recommended: reviewScore?.eligible ?? false,
    }
  })
})

const summary = computed(() => {
  const files = parsedFiles.value
  return {
    total: files.length,
    modified: files.filter(f => f.type === 'modified').length,
    added: files.filter(f => f.type === 'added').length,
    deleted: files.filter(f => f.type === 'deleted').length,
    renamed: files.filter(f => f.type === 'renamed').length,
    copied: files.filter(f => f.type === 'copied').length,
    untracked: files.filter(f => f.type === 'untracked').length,
    conflicted: files.filter(f => f.type === 'updated-but-unmerged').length,
    staged: files.filter(f => f.staged).length,
    unstaged: files.filter(f => f.unstaged).length,
  }
})

const recommendedFiles = computed(() =>
  [...allFiles.value].filter(f => f.recommended).sort((a, b) => (b.score ?? 0) - (a.score ?? 0)),
)
const conflictedFiles = computed(() => allFiles.value.filter(f => f.type === 'updated-but-unmerged'))
const selectedFileViews = computed(() => allFiles.value.filter(f => reviewSelectedRaws.value.includes(f.raw)))
const branchOptions = computed<SelectOption[]>(() => (snapshot.value?.branches ?? []).map(branch => ({
  label: branch.current ? `${branch.name} (${t('gitAssistant.repo.currentBranch')})` : branch.name,
  value: `${branch.kind}:${branch.name}`,
})))
const currentBranchValue = computed(() => {
  const branch = snapshot.value?.branches.find(item => item.current)
  return branch ? `${branch.kind}:${branch.name}` : null
})
const mergeSourceOptions = computed<SelectOption[]>(() => (snapshot.value?.branches ?? [])
  .filter(branch => !branch.current && !branch.name.endsWith('/HEAD'))
  .map(branch => ({ label: branch.name, value: branch.name })))
const mergeModeOptions = computed<SelectOption[]>(() => [
  { label: t('gitAssistant.repo.mergeModeDefault'), value: 'default' },
  { label: t('gitAssistant.repo.mergeModeNoFastForward'), value: 'no-ff' },
])
console.log(branchOptions.value)
const promptFileGroups = computed(() => {
  if (!promptPreview.value) return []
  const groups = new Map<string, { path: string; role: string; scope: string; kind: string; strategy: string; evidenceCount: number; rawChars: number; cleanedChars: number; skipped: boolean; reason?: string | null }[]>()
  for (const file of promptPreview.value.trace.selectedFiles) {
    const key = file.kind || 'other'
    groups.set(key, [...(groups.get(key) ?? []), file])
  }
  return Array.from(groups.entries())
    .map(([kind, files]) => ({ kind, files }))
    .sort((a, b) => b.files.length - a.files.length || a.kind.localeCompare(b.kind))
})

const filteredFiles = computed(() => {
  let files = [...allFiles.value]
  if (statusFilter.value !== 'all') {
    if (statusFilter.value === 'staged') files = files.filter(f => f.staged)
    else if (statusFilter.value === 'unstaged') files = files.filter(f => f.unstaged)
    else if (statusFilter.value === 'versioned') files = files.filter(f => f.type !== 'untracked')
    else if (statusFilter.value === 'recommended') files = files.filter(f => f.recommended)
    else files = files.filter(f => f.type === statusFilter.value)
  }
  if (recommendedOnly.value) files = files.filter(f => f.recommended)
  const kw = keyword.value.trim().toLowerCase()
  if (kw) files = files.filter(f => `${f.path} ${f.fileName} ${f.directory}`.toLowerCase().includes(kw))
  return files.sort((a, b) => (b.score ?? -1) - (a.score ?? -1) || a.path.localeCompare(b.path))
})

const filteredFileGroups = computed<GitAssistantFileGroup[]>(() => [{
  key: 'all-files',
  label: recommendedOnly.value ? t('gitAssistant.files.groupRecommended') : t('gitAssistant.files.groupAll'),
  files: filteredFiles.value,
}])

const selectedFile = computed(() => {
  if (!activeFileRaw.value) return filteredFiles.value[0] ?? allFiles.value[0] ?? null
  return allFiles.value.find(f => f.raw === activeFileRaw.value) ?? filteredFiles.value[0] ?? null
})

const modelSelectOptions = computed(() => {
  if (!aiSettings.enabledModels.length) return [{ label: t('gitAssistant.ai.noModelConfigured'), value: '' }]
  return aiSettings.enabledModels.map(m => ({ label: m.name, value: m.id }))
})

const commitLanguageOptions = computed(() => [
  { label: 'English', value: 'en' },
  { label: '中文', value: 'zh' },
])
const selectedCommitModelLabel = computed(() => {
  const value = aiSettings.taskModelMap['commit-message'] || aiSettings.defaultModelId
  return modelSelectOptions.value.find(option => option.value === value)?.label ?? t('gitAssistant.ai.noModelConfigured')
})
const selectedCommitLanguageLabel = computed(() => commitLanguageOptions.value.find(option => option.value === commitLanguage.value)?.label ?? '')

const filteredCommitMessageHistory = computed(() => {
  const cur = normalizePath(displayRepoPath.value).toLowerCase()
  return commitMessageHistory.value.filter(e => !cur || normalizePath(e.repoPath).toLowerCase() === cur)
})

const needsRemoteUrl = computed(() => Boolean(snapshot.value && !repositoryState.value?.remoteName))
const canRepairUpstream = computed(() => {
  const s = repositoryState.value
  return Boolean(s?.remoteName && s.hasCommits && (s.upstreamGone || !s.upstream))
})
const canPublishBranch = computed(() => {
  const s = repositoryState.value
  return Boolean(s?.remoteName && s.hasCommits && (!s.upstream || s.upstreamGone))
})
const isDiverged = computed(() => Boolean(repositoryState.value && repositoryState.value.ahead > 0 && repositoryState.value.behind > 0))
const showRemoteTools = computed(() => needsRemoteUrl.value || canRepairUpstream.value || canPublishBranch.value || isDiverged.value)
const showConflictTools = computed(() =>
  conflictedFiles.value.length > 0 || Boolean(repositoryState.value?.mergeInProgress || repositoryState.value?.rebaseInProgress),
)
const remoteToolStatus = computed(() => {
  if (isDiverged.value) return t('gitAssistant.remote.diverged')
  if (needsRemoteUrl.value) return t('gitAssistant.remote.missingOrigin')
  if (repositoryState.value?.upstreamGone) return t('gitAssistant.remote.upstreamGone')
  if (!repositoryState.value?.upstream) return t('gitAssistant.remote.upstreamMissing')
  return t('gitAssistant.remote.ready')
})
const remoteToolHint = computed(() => {
  if (isDiverged.value) return t('gitAssistant.remote.divergedHint')
  if (needsRemoteUrl.value) return t('gitAssistant.remote.originHint')
  if (repositoryState.value?.upstreamGone) return t('gitAssistant.remote.upstreamGoneHint')
  if (!repositoryState.value?.upstream) return t('gitAssistant.remote.upstreamMissingHint')
  return t('gitAssistant.remote.readyHint')
})

// ── Functions ──
function historySourceLabel(source: 'ai' | 'manual') {
  return source === 'manual' ? t('gitAssistant.history.manual') : t('gitAssistant.history.ai')
}

function handleStatusFilterChange(value: string) {
  statusFilter.value = value as GitAssistantStatusFilter
}

function handleSelectFile(raw: string) {
  activeFileRaw.value = raw
}

async function handleOpenDiff(raw: string) {
  activeFileRaw.value = raw
  const file = allFiles.value.find(i => i.raw === raw)
  if (!file || !displayRepoPath.value) return
  const mode = file.unstaged || !file.staged ? 'unstaged' : 'staged'
  await openGitDiffWindow({ kind: 'working-tree', repoPath: displayRepoPath.value, filePath: file.path, mode })
}

function handleSyncAction(action: string) {
  if (action === 'pull') void handlePull()
  else if (action === 'fetch') void handleFetch()
  else if (action === 'push') void handlePush()
}

async function handleFileAction(payload: { action: 'open-diff' | 'diff-previous' | 'file-history' | 'open-external' | 'mark-resolved' | 'revert' | 'stage' | 'unstage'; raw: string }) {
  const file = allFiles.value.find(i => i.raw === payload.raw)
  if (!file) return
  if (payload.action === 'open-diff') { await handleOpenDiff(payload.raw); return }
  if (payload.action === 'diff-previous') { activeFileRaw.value = payload.raw; diffMode.value = 'head'; showDiff.value = true; return }
  if (payload.action === 'file-history') { await handleOpenLog(file.path); return }
  if (payload.action === 'open-external') { await handleOpenExternalFile(file.path); return }
  if (payload.action === 'mark-resolved') { await handleMarkResolved([file.path]) }
  if (payload.action === 'revert') { await handleRevertFile(file.path) }
  if (payload.action === 'stage') { await handleStageFiles([file.raw], true) }
  if (payload.action === 'unstage') { await handleStageFiles([file.raw], false) }
}

async function handleStageFiles(raws: string[], stage: boolean) {
  if (!displayRepoPath.value) return
  const paths = raws
    .map(raw => allFiles.value.find(file => file.raw === raw))
    .filter((file): file is GitAssistantFileView => Boolean(file && (stage ? file.unstaged : file.staged)))
    .map(file => file.path)
  if (!paths.length) return

  error.value = ''
  startGitCommand(stage ? t('gitAssistant.files.stageVisible') : t('gitAssistant.files.unstageVisible'), stage ? 'Staging files' : 'Unstaging files')
  try {
    const result = stage
      ? await stageGitFiles(displayRepoPath.value, paths)
      : await unstageGitFiles(displayRepoPath.value, paths)
    finishGitCommand(result)
    await loadSnapshotByPath(displayRepoPath.value)
  } catch (err) {
    console.error(err)
    failGitCommand(err)
  }
}

function openBranchSelector() {
  branchSelectionValue.value = currentBranchValue.value
  branchSelectorOpen.value = true
}

function openMergeDialog() {
  mergeSourceValue.value = null
  mergeMode.value = 'default'
  mergeDialogOpen.value = true
}

async function handleMergeBranch() {
  if (!displayRepoPath.value || !mergeSourceValue.value) return
  mergeLoading.value = true
  error.value = ''
  startGitCommand(t('gitAssistant.repo.mergeBranch'), 'Merging branch')
  try {
    const result = await mergeGitBranch(displayRepoPath.value, mergeSourceValue.value, mergeMode.value === 'no-ff')
    finishGitCommand(result)
    mergeDialogOpen.value = false
    await loadSnapshotByPath(displayRepoPath.value, true)
  } catch (err) {
    console.error(err)
    failGitCommand(err)
    error.value = err instanceof Error ? err.message : t('gitAssistant.errorFallback')
    await loadSnapshotByPath(displayRepoPath.value, true)
  } finally {
    mergeLoading.value = false
  }
}

async function handleBranchSelection(value: string) {
  const [kind, ...nameParts] = value.split(':')
  const branch = nameParts.join(':')
  if (!displayRepoPath.value || !branch) return
  branchSelectorOpen.value = false
  if (kind === 'local') {
    await runBranchAction('Switching branch', branch, () => switchGitBranch(displayRepoPath.value, branch))
    return
  }
  if (kind === 'remote') {
    const localBranch = branch.split('/').slice(1).join('/')
    if (!localBranch) return
    await runBranchAction('Checking out remote branch', localBranch, () => checkoutGitRemoteBranch(displayRepoPath.value, branch, localBranch))
  }
}

async function runBranchAction(phase: string, nextBranch: string, action: () => ReturnType<typeof switchGitBranch>) {
  branchLoading.value = true
  error.value = ''
  startGitCommand(t('gitAssistant.repo.branch'), phase)
  try {
    const result = await action()
    finishGitCommand(result)
    if (snapshot.value) {
      snapshot.value = {
        ...snapshot.value,
        branch: nextBranch,
        branches: (snapshot.value.branches ?? []).map(branch => ({ ...branch, current: branch.name === nextBranch && branch.kind === 'local' })),
      }
    }
    void loadSnapshotByPath(displayRepoPath.value, true)
  } catch (err) {
    console.error(err)
    failGitCommand(err)
    error.value = err instanceof Error ? err.message : t('gitAssistant.errorFallback')
  } finally {
    branchLoading.value = false
  }
}

function openRepositorySetup(mode: 'init' | 'clone') {
  repositorySetupMode.value = mode
  repositoryPathDraft.value = ''
  cloneUrlDraft.value = ''
  repositorySetupOpen.value = true
}

async function pickRepositoryTarget() {
  const selected = await open({ directory: true, multiple: false, title: t('gitAssistant.repo.chooseDirectory') })
  if (selected && !Array.isArray(selected)) repositoryPathDraft.value = selected
}

async function handleRepositorySetup() {
  if (!repositoryPathDraft.value) return
  repositoryLoading.value = true
  error.value = ''
  try {
    const result = repositorySetupMode.value === 'clone'
      ? await cloneGitRepository(cloneUrlDraft.value.trim(), repositoryPathDraft.value)
      : await initGitRepository(repositoryPathDraft.value)
    finishGitCommand(result)
    repositorySetupOpen.value = false
    repoPath.value = repositoryPathDraft.value
    await loadSnapshotByPath(repositoryPathDraft.value)
  } catch (err) {
    console.error(err)
    error.value = err instanceof Error ? err.message : t('gitAssistant.errorFallback')
  } finally {
    repositoryLoading.value = false
  }
}

async function handleOpenExternalFile(filePath: string) {
  if (!displayRepoPath.value) return
  try { await openGitFileExternal(displayRepoPath.value, filePath) } catch (err) {
    console.error(err); error.value = err instanceof Error ? err.message : t('gitAssistant.errorFallback')
  }
}

function openConflictDialog() {
  conflictSelectedPaths.value = conflictedFiles.value.map(file => file.path)
  conflictDialogOpen.value = true
}

function toggleConflictSelection(filePath: string, checked: boolean) {
  conflictSelectedPaths.value = checked
    ? [...new Set([...conflictSelectedPaths.value, filePath])]
    : conflictSelectedPaths.value.filter(path => path !== filePath)
}

async function handleMarkConflictPathsResolved() {
  await handleMarkResolved(conflictSelectedPaths.value)
}

async function handleRevertFile(filePath: string) {
  if (!displayRepoPath.value) return
  error.value = ''
  startGitCommand(t('gitAssistant.gitCommand.revertFileTitle'), t('gitAssistant.gitCommand.revertingFile'))
  try {
    const result = await revertGitFile(displayRepoPath.value, filePath)
    finishGitCommand(result)
    reviewSelectedRaws.value = reviewSelectedRaws.value.filter(raw => {
      const file = allFiles.value.find(item => item.raw === raw)
      return file?.path !== filePath
    })
    await loadSnapshotByPath(displayRepoPath.value)
  } catch (err) {
    console.error(err)
    failGitCommand(err)
  }
}

async function handleOpenLog(filePath = '') {
  if (!displayRepoPath.value) return
  error.value = ''
  await openGitLogWindow(displayRepoPath.value, filePath, snapshot.value?.branch || '')
}

function toggleReviewSelection(payload: { raw: string; checked: boolean }) {
  if (payload.checked) { if (!reviewSelectedRaws.value.includes(payload.raw)) reviewSelectedRaws.value = [...reviewSelectedRaws.value, payload.raw]; return }
  reviewSelectedRaws.value = reviewSelectedRaws.value.filter(r => r !== payload.raw)
}

function setReviewSelection(raws: string[]) {
  const valid = new Set(allFiles.value.map(f => f.raw))
  reviewSelectedRaws.value = [...new Set(raws.filter(r => valid.has(r)))]
}

async function loadReviewScores() {
  const repoPath = displayRepoPath.value
  const files = parsedFiles.value.map(file => file.path)
  const requestId = ++reviewScoreRequestId
  reviewScores.value = new Map()
  if (!repoPath || !files.length) return
  reviewScoring.value = true
  reviewScoreProgress.value = { completed: 0, total: files.length, phase: 'preparing', filePath: '' }

  try {
    const result = await scoreGitReviewFiles(repoPath, files)
    if (requestId !== reviewScoreRequestId) return
    reviewScores.value = new Map(result.files.map(file => [normalizePath(file.path), file]))
    reviewScoreProgress.value = { completed: files.length, total: files.length, phase: 'complete', filePath: '' }
  } catch (err) {
    console.error(err)
    if (requestId === reviewScoreRequestId) {
      error.value = err instanceof Error ? err.message : t('gitAssistant.errorFallback')
    }
  } finally {
    if (requestId === reviewScoreRequestId) reviewScoring.value = false
  }
}

function handleSwitchRecentRepoFromManager(path: string) {
  recentRepoManagerOpen.value = false
  void handleSwitchRecentRepo(path)
}

// ── Watchers ──
watch(filteredFiles, files => {
  if (files.length === 0) { activeFileRaw.value = null; return }
  if (!files.some(f => f.raw === activeFileRaw.value)) activeFileRaw.value = files[0]?.raw ?? null
}, { immediate: true })

watch(allFiles, files => {
  const fileSet = new Set(files.map(f => f.raw))
  reviewSelectedRaws.value = reviewSelectedRaws.value.filter(r => fileSet.has(r))
}, { immediate: true })

watch(() => snapshot.value?.status, () => {
  reviewScoreRequestId += 1
  reviewScores.value = new Map()
  reviewScoring.value = false
  reviewScoreProgress.value = { completed: 0, total: 0, phase: '', filePath: '' }
}, { immediate: true })

watch(showConflictTools, (active, wasActive) => {
  if (active && !wasActive) openConflictDialog()
  if (!active) conflictDialogOpen.value = false
})

watch(selectedFile, file => {
  if (diffMode.value === 'head') return
  if (!file) { diffMode.value = 'unstaged'; return }
  diffMode.value = file.unstaged ? 'unstaged' : 'staged'
}, { immediate: true })

watch([selectedFile, diffMode, showDiff], async ([file, mode, visible]) => {
  if (!visible || !file || !displayRepoPath.value) { currentDiff.value = ''; return }
  if (mode === 'head') {
    diffLoading.value = true
    try { const r = await (await import('@/services/git/git-service')).loadGitFileHeadDiff(displayRepoPath.value, file.path); currentDiff.value = r.diff }
    catch (err) { console.error(err); currentDiff.value = ''; error.value = err instanceof Error ? err.message : t('gitAssistant.errorFallback') }
    finally { diffLoading.value = false }
    return
  }
  const staged = mode === 'staged'
  if ((staged && !file.staged) || (!staged && !file.unstaged)) { currentDiff.value = ''; return }
  diffLoading.value = true
  try { const { loadGitFileDiff } = await import('@/services/git/git-service'); const r = await loadGitFileDiff({ repoPath: displayRepoPath.value, filePath: file.path, staged }); currentDiff.value = r.diff }
  catch (err) { console.error(err); currentDiff.value = ''; error.value = err instanceof Error ? err.message : t('gitAssistant.errorFallback') }
  finally { diffLoading.value = false }
}, { immediate: true })

// ── Lifecycle ──
onActivated(() => {
  if (!refreshOnNextActivation || !displayRepoPath.value) return
  refreshOnNextActivation = false
  void handleRefresh()
})

onDeactivated(() => {
  refreshOnNextActivation = true
})

onMounted(async () => {
  unlistenReviewScoreProgress = await listen<{
    repoPath: string
    completed: number
    total: number
    phase: string
    filePath?: string | null
  }>('git-review-score-progress', event => {
    if (!reviewScoring.value) return
    if (normalizePath(event.payload.repoPath).toLowerCase() !== normalizePath(displayRepoPath.value).toLowerCase()) return
    reviewScoreProgress.value = {
      completed: event.payload.completed,
      total: event.payload.total,
      phase: event.payload.phase,
      filePath: event.payload.filePath ?? '',
    }
  })
  loadRecentRepos()
  loadCommitMessageHistory()
  const saved = localStorage.getItem(GIT_REPO_STORAGE_KEY)
  if (!saved) return
  repoPath.value = saved
  await loadSnapshotByPath(saved)
})

onUnmounted(() => {
  unlistenReviewScoreProgress?.()
})
</script>
<style scoped lang="scss">
.git-assistant-page {
  background:
    linear-gradient(180deg, color-mix(in srgb, var(--lumina-bg) 82%, var(--lumina-surface-1)), var(--lumina-bg)),
    var(--lumina-bg);
  box-sizing: border-box;
  color: var(--lumina-text);
  display: flex;
  flex-direction: column;
  gap: 8px;
  height: 100%;
  min-height: 0;
  min-width: 0;
  overflow: hidden;
  padding: 8px;
  position: relative;
}

.error-banner {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-danger) 12%, var(--lumina-surface-2));
  border: 1px solid color-mix(in srgb, var(--lumina-danger) 28%, transparent);
  border-radius: 10px;
  color: var(--lumina-danger);
  display: flex;
  flex: 0 0 auto;
  font-size: 12px;
  gap: 10px;
  justify-content: space-between;
  padding: 8px 10px;

  span {
    min-width: 0;
  }

  button {
    align-items: center;
    background: transparent;
    border: 0;
    border-radius: var(--lumina-radius-sm);
    color: currentcolor;
    cursor: pointer;
    display: inline-flex;
    flex: 0 0 auto;
    height: 24px;
    justify-content: center;
    padding: 0;
    width: 24px;

    &:hover {
      background: color-mix(in srgb, var(--lumina-danger) 10%, transparent);
    }

    svg {
      height: 16px;
      width: 16px;
    }
  }
}

.workspace-body {
  display: grid;
  flex: 1 1 auto;
  gap: 8px;
  grid-template-rows: minmax(260px, 1fr) auto;
  min-height: 0;
  min-width: 0;
  overflow: hidden;
}

.commit-area {
  display: grid;
  gap: 8px;
  grid-template-columns: minmax(0, 1fr) minmax(340px, 370px);
  min-height: 0;
}

.commit-workbench {
  border: 1px solid var(--lumina-card-border);
  border-radius: var(--lumina-radius-lg);
  box-shadow: var(--lumina-shadow-sm);
}

.commit-side {
  background: var(--lumina-surface-1);
  border: 1px solid var(--lumina-card-border);
  border-radius: var(--lumina-radius-lg);
  box-shadow: var(--lumina-shadow-sm);
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 10px;
}

.ai-panel-title {
  align-items: center;
  border-bottom: 1px solid var(--lumina-card-border);
  display: flex;
  justify-content: space-between;
  min-height: 30px;
  padding: 0 2px 8px;

  span {
    color: var(--lumina-text);
    font-size: 14px;
    font-weight: 650;
  }

  strong {
    color: var(--lumina-primary);
    font-size: 18px;
    font-weight: 700;
    line-height: 1;
  }
}

.ai-settings {
  background: var(--lumina-surface-2);
  border: 1px solid var(--lumina-card-border);
  border-radius: var(--lumina-radius-md);

  summary {
    align-items: center;
    cursor: pointer;
    display: grid;
    gap: 8px;
    grid-template-columns: auto minmax(0, 1fr) 16px;
    min-height: 34px;
    padding: 0 9px;
    user-select: none;

    &::marker {
      content: '';
    }

    span {
      color: var(--lumina-text-secondary);
      font-size: 11px;
    }

    strong {
      font-size: 11px;
      font-weight: 600;
      overflow: hidden;
      text-align: right;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    svg {
      color: var(--lumina-text-secondary);
      height: 16px;
      transition: transform 160ms ease;
      width: 16px;
    }
  }

  &[open] summary svg {
    transform: rotate(180deg);
  }

  .ai-tool-section {
    border: 0;
    border-top: 1px solid var(--lumina-card-border);
    border-radius: 0 0 var(--lumina-radius-md) var(--lumina-radius-md);
  }
}

.ai-tool-section {
  background: var(--lumina-surface-2);
  border: 1px solid var(--lumina-card-border);
  border-radius: var(--lumina-radius-md);
  display: grid;
  gap: 8px;
  padding: 9px;
}

.ai-tool-section--actions {
  background: color-mix(in srgb, var(--lumina-surface-2) 76%, var(--lumina-surface-1));
}

.model-field {
  display: grid;
  gap: 6px;

  span {
    color: var(--lumina-text-secondary);
    font-size: 11px;
  }

  :deep(.model-select .n-base-selection) {
    background: color-mix(in srgb, var(--lumina-input-bg) 92%, transparent);
    border-radius: 7px;
    min-height: 32px;
  }

  :deep(.model-select .n-base-selection-label) {
    color: var(--lumina-text);
  }
}

.remote-tools {
  background: var(--lumina-surface-2);
  border: 1px solid var(--lumina-card-border);
  border-radius: var(--lumina-radius-md);
  display: grid;
  gap: 9px;
  padding: 10px;

  p {
    color: var(--lumina-text-secondary);
    font-size: 11px;
    line-height: 1.45;
    margin: 0;
  }
}

.remote-tools__header {
  align-items: center;
  display: flex;
  justify-content: space-between;

  span {
    color: var(--lumina-text);
    font-size: 12px;
    font-weight: 650;
  }

  strong {
    color: var(--lumina-warning);
    font-size: 11px;
    font-weight: 650;
  }
}

.remote-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.conflict-tools {
  background:
    linear-gradient(180deg, color-mix(in srgb, var(--lumina-danger) 7%, transparent), transparent),
    color-mix(in srgb, var(--lumina-surface-2) 66%, transparent);
  border: 1px solid color-mix(in srgb, var(--lumina-danger) 28%, var(--lumina-card-border));
  border-radius: var(--lumina-radius-md);
  align-items: center;
  display: flex;
  gap: 10px;
  justify-content: space-between;
  padding: 10px;
}

.conflict-tools__header {
  align-items: center;
  display: flex;
  gap: 8px;

  span {
    color: var(--lumina-text);
    font-size: 12px;
    font-weight: 650;
  }

  strong {
    color: var(--lumina-danger);
    font-size: 11px;
    font-weight: 650;
  }
}

.ai-toggle {
  color: var(--lumina-text-secondary);
  font-size: 12px;
  min-height: 24px;
}

:deep(.ai-toggle .n-checkbox__label) {
  color: var(--lumina-text-secondary);
  font-size: 12px;
}

.ai-action-grid {
  display: grid;
  gap: 6px;
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.ai-action,
.secondary-btn {
  background: var(--lumina-button-secondary-bg);
  border: 1px solid var(--lumina-card-border);
  border-radius: var(--lumina-radius-sm);
  color: var(--lumina-text);
  cursor: pointer;
  font-size: 12px;
  font-weight: 600;
  min-height: 30px;
  padding: 0 10px;
  width: 100%;

  &:disabled {
    cursor: not-allowed;
    opacity: 0.55;
  }
}

.ai-progress {
  align-items: center;
  color: var(--lumina-text-secondary);
  display: flex;
  font-size: 11px;
  gap: 7px;
  min-height: 18px;
}

.ai-progress__dot {
  animation: ai-progress-pulse 1s ease-in-out infinite;
  background: var(--lumina-primary);
  border-radius: 999px;
  height: 7px;
  width: 7px;
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

.primary-action {
  background: var(--lumina-primary);
  border-color: var(--lumina-primary);
  box-shadow: none;
  color: #fff;
  font-size: 13px;
  min-height: 34px;
}

.change-table {
  min-height: 0;
}

.diff-window {
  height: 100%;
  width: 100%;
}

:deep(.diff-window.diff-viewer) {
  border: 0;
  border-radius: var(--lumina-radius-lg);
  box-shadow: none;
  height: 100%;
  width: 100%;
}

:deep(.diff-window .diff-header) {
  padding-right: 56px;
}

.recent-repo-dialog {
  background: var(--lumina-surface-1);
  border: 1px solid var(--lumina-card-border);
  border-radius: var(--lumina-radius-lg);
  box-shadow: var(--lumina-shadow-md);
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  max-height: min(640px, calc(100vh - 96px));
  overflow: hidden;
  position: relative;
  width: min(760px, calc(100vw - 72px));
}

.modal-close-button {
  align-items: center;
  background: var(--lumina-surface-2);
  border: 1px solid var(--lumina-card-border);
  border-radius: var(--lumina-radius-sm);
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: flex;
  height: 30px;
  justify-content: center;
  padding: 0;
  position: absolute;
  right: 12px;
  top: 12px;
  width: 30px;
  z-index: 3;

  &:hover {
    background: var(--lumina-button-secondary-hover);
    color: var(--lumina-text);
  }

  &:focus-visible {
    box-shadow: 0 0 0 3px var(--lumina-accent-ring);
    outline: none;
  }

  svg {
    height: 16px;
    width: 16px;
  }
}

.recent-repo-dialog__header {
  align-items: flex-start;
  border-bottom: 1px solid var(--lumina-card-border);
  display: flex;
  gap: 14px;
  justify-content: space-between;
  padding: 16px 56px 16px 16px;

  h3 {
    font-size: 16px;
    margin: 0 0 4px;
  }

  p {
    color: var(--lumina-text-secondary);
    font-size: 12px;
    line-height: 1.5;
    margin: 0;
  }
}

.recent-repo-list {
  display: grid;
  gap: 10px;
  min-height: 0;
  overflow: auto;
  padding: 14px;
}

.recent-repo-item {
  align-items: center;
  background: var(--lumina-surface-2);
  border: 1px solid var(--lumina-card-border);
  border-radius: var(--lumina-radius-md);
  display: grid;
  gap: 12px;
  grid-template-columns: minmax(0, 1fr) auto;
  padding: 12px;
}

.recent-repo-item__main {
  display: grid;
  gap: 7px;
  min-width: 0;

  strong {
    color: var(--lumina-text);
    font-size: 13px;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
}

.recent-repo-alias-text {
  color: var(--lumina-text);
  font-size: 13px;
  font-weight: 600;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.recent-repo-alias-edit {
  min-width: 0;
  width: 100%;
}

.recent-repo-alias-input {
  background: var(--lumina-input-bg);
  border: 1px solid var(--lumina-input-border);
  border-radius: var(--lumina-radius-sm);
  color: var(--lumina-text);
  font: inherit;
  font-size: 13px;
  height: 28px;
  min-width: 0;
  padding: 0 10px;
  width: 100%;

  &::placeholder {
    color: var(--lumina-text-secondary);
  }

  &:hover {
    border-color: var(--lumina-primary);
  }

  &:focus {
    border-color: var(--lumina-primary);
    box-shadow: 0 0 0 2px var(--lumina-accent-ring);
    outline: none;
  }
}

.recent-repo-path {
  color: var(--lumina-text-secondary);
  font-size: 11px;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.recent-repo-item__actions {
  align-items: center;
  display: flex;
  gap: 8px;
  justify-content: flex-end;
  min-width: 112px;
}

.recent-repo-empty {
  align-items: center;
  color: var(--lumina-text-secondary);
  display: flex;
  font-size: 12px;
  justify-content: center;
  min-height: 180px;
  padding: 20px;
}

.repository-action-dialog {
  background: var(--lumina-surface-1);
  border: 1px solid var(--lumina-card-border);
  border-radius: var(--lumina-radius-lg);
  box-shadow: var(--lumina-shadow-lg);
  color: var(--lumina-text);
  display: grid;
  gap: 14px;
  min-width: 480px;
  padding: 16px;
}

.repository-action-dialog header,
.repository-path-picker,
.repository-action-footer,
.repository-action-create {
  align-items: center;
  display: flex;
  gap: 10px;
}

.repository-action-dialog header {
  justify-content: space-between;

  h3,
  p {
    margin: 0;
  }

  h3 {
    font-size: 15px;
  }

  p {
    color: var(--lumina-text-secondary);
    font-size: 12px;
    margin-top: 4px;
  }
}

.repository-action-create > :first-child,
.repository-path-picker > :first-child {
  flex: 1 1 auto;
}

.repository-action-footer {
  justify-content: flex-end;
}

.conflict-dialog {
  background: var(--lumina-surface-1);
  border: 1px solid var(--lumina-card-border);
  border-radius: var(--lumina-radius-lg);
  box-shadow: var(--lumina-shadow-lg);
  color: var(--lumina-text);
  display: grid;
  grid-template-rows: auto minmax(140px, 1fr) auto;
  max-height: min(680px, calc(100vh - 96px));
  overflow: hidden;
  position: relative;
  width: min(760px, calc(100vw - 72px));
}

.conflict-dialog__header {
  border-bottom: 1px solid var(--lumina-card-border);
  padding: 16px 56px 16px 16px;

  h3 {
    font-size: 16px;
    margin: 0 0 4px;
  }

  p {
    color: var(--lumina-text-secondary);
    font-size: 12px;
    line-height: 1.5;
    margin: 0;
  }
}

.conflict-file-list {
  display: grid;
  gap: 8px;
  overflow: auto;
  padding: 14px;
}

.conflict-file-row {
  align-items: center;
  background: var(--lumina-surface-2);
  border: 1px solid var(--lumina-card-border);
  border-radius: var(--lumina-radius-md);
  cursor: default;
  display: grid;
  gap: 10px;
  grid-template-columns: auto minmax(0, 1fr) auto;
  min-height: 42px;
  padding: 0 12px;

  &:hover {
    border-color: color-mix(in srgb, var(--lumina-primary) 55%, var(--lumina-card-border));
  }

  span {
    font-family: var(--lumina-font-mono);
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  small {
    color: var(--lumina-text-secondary);
    font-size: 11px;
  }
}

.conflict-dialog__empty {
  align-items: center;
  color: var(--lumina-text-secondary);
  display: flex;
  justify-content: center;
  min-height: 140px;
}

.conflict-dialog__footer {
  align-items: center;
  border-top: 1px solid var(--lumina-card-border);
  display: flex;
  gap: 12px;
  justify-content: space-between;
  padding: 12px 14px;

  > div {
    display: flex;
    gap: 8px;
  }
}

.repository-action-field {
  display: grid;
  gap: 6px;

  > span {
    color: var(--lumina-text-secondary);
    font-size: 12px;
  }
}

.repository-action-hint {
  color: var(--lumina-text-secondary);
  font-size: 12px;
  line-height: 1.6;
  margin: 0;
}

.history-list {
  display: flex;
  flex-direction: column;
  gap: 0;
  min-height: 0;
  overflow: auto;
  padding: 0;
}

.history-item {
  background: transparent;
  border: 0;
  border-bottom: 1px solid var(--lumina-card-border);
  border-radius: 0;
  display: grid;
  gap: 10px;
  grid-template-columns: minmax(0, 1fr) 54px;
  padding: 10px 12px;

  &:hover {
    background: var(--lumina-button-secondary-hover);
  }

  button {
    align-self: start;
    background: var(--lumina-primary);
    border: 1px solid var(--lumina-primary);
    border-radius: var(--lumina-radius-sm);
    color: #fff;
    cursor: pointer;
    font-size: 12px;
    height: 28px;
    padding: 0 10px;
  }
}

.history-item__main {
  display: grid;
  gap: 5px;
  min-width: 0;

  span,
  small {
    color: var(--lumina-text-secondary);
    font-size: 11px;
  }

  strong {
    color: var(--lumina-text);
    font-size: 13px;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  p {
    color: var(--lumina-text-secondary);
    display: -webkit-box;
    font-size: 12px;
    line-height: 1.45;
    margin: 0;
    overflow: hidden;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
  }
}

.history-empty {
  align-items: center;
  border: 1px dashed var(--lumina-card-border);
  border-radius: 9px;
  color: var(--lumina-text-secondary);
  display: flex;
  font-size: 12px;
  justify-content: center;
  min-height: 120px;
  padding: 16px;
  text-align: center;
}

.prompt-drawer__body {
  display: flex;
  flex-direction: column;
  gap: 8px;
  min-height: 0;
  overflow: auto;
  padding: 10px;
}

.prompt-section {
  border: 1px solid var(--lumina-card-border);
  border-radius: var(--lumina-radius-md);
  background: var(--lumina-surface-2);
  overflow: hidden;

  summary {
    align-items: center;
    background: color-mix(in srgb, var(--lumina-surface-2) 70%, var(--lumina-surface-1));
    border-bottom: 1px solid var(--lumina-card-border);
    cursor: pointer;
    display: flex;
    font-size: 13px;
    font-weight: 700;
    min-height: 32px;
    padding: 0 10px;
  }
}

.prompt-stats {
  display: grid;
  gap: 8px;
  grid-template-columns: repeat(4, minmax(0, 1fr));

  div {
    background: var(--lumina-surface-1);
    border: 1px solid var(--lumina-card-border);
    border-radius: var(--lumina-radius-sm);
    padding: 8px;
  }

  span {
    color: var(--lumina-text-secondary);
    display: block;
    font-size: 11px;
  }

  strong {
    display: block;
    font-size: 15px;
    margin-top: 4px;
  }
}

.prompt-files,
.prompt-text {
  min-width: 0;
}

.prompt-rules {
  color: var(--lumina-text-secondary);
  font-size: 12px;
  padding: 0 12px 12px;

  p {
    margin: 0 0 8px;
  }

  ol {
    margin: 0;
    padding-left: 18px;
  }

  li + li {
    margin-top: 5px;
  }
}

.prompt-files {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 0 10px 10px;
}

.prompt-file-group {
  border: 1px solid var(--lumina-card-border);
  border-radius: var(--lumina-radius-sm);
  overflow: hidden;
}

.prompt-file-group__title {
  align-items: center;
  background: var(--lumina-surface-2);
  border-bottom: 1px solid var(--lumina-card-border);
  display: flex;
  justify-content: space-between;
  min-height: 30px;
  padding: 0 10px;

  strong {
    font-size: 13px;
  }

  span {
    color: var(--lumina-text-secondary);
    font-size: 12px;
  }
}

.prompt-file-table {
  overflow: auto;
  width: 100%;
}

.prompt-file-table__head,
.prompt-file-table__row {
  align-items: center;
  display: grid;
  gap: 10px;
  grid-template-columns: minmax(260px, 2fr) 86px 86px minmax(150px, 1fr) 70px 96px minmax(180px, 1.2fr);
  min-width: 920px;
  min-height: 29px;
  padding: 0 10px;
}

.prompt-file-table__head {
  color: var(--lumina-text-secondary);
  font-size: 11px;
  font-weight: 700;
}

.prompt-file-table__row {
  border-top: 1px solid var(--lumina-card-border);
  font-size: 12px;

  span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  span:not(.mono) {
    color: var(--lumina-text-secondary);
  }
}

.prompt-text {
  padding: 0 10px 10px;

  textarea {
    background: var(--lumina-diff-bg);
    border: 1px solid var(--lumina-card-border);
    border-radius: var(--lumina-radius-sm);
    color: var(--lumina-text);
    font-family: SFMono-Regular, Consolas, 'Liberation Mono', Menlo, monospace;
    font-size: 12px;
    line-height: 1.6;
    min-height: 380px;
    padding: 10px;
    resize: none;
    width: 100%;
  }
}

@media (max-width: 980px) {
  .commit-area {
    grid-template-columns: 1fr;
  }
}
</style>
