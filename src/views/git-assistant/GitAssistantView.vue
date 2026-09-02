<template>
  <div class="git-assistant-page">
    <div v-if="error" class="error-banner" role="alert">
      <span>{{ error }}</span>
      <button type="button" :aria-label="t('common.dismiss')" @click="error = ''">
        <span class="close-glyph" aria-hidden="true">×</span>
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
      :panel-visibility="panelLayout.visible"
      @pick-directory="handleSelectDirectory"
      @refresh="handleRefresh"
      @sync-action="handleSyncAction"
      @manage-repos="recentRepoManagerOpen = true"
      @open-branch-selector="openBranchSelector"
      @open-merge="openMergeDialog"
      @clone-repository="openRepositorySetup('clone')"
      @init-repository="openRepositorySetup('init')"
      @open-repository-rules="repositoryRulesOpen = true"
      @toggle-panel="togglePanel"
      @reset-layout="resetPanelLayout"
    />

    <section ref="workspaceBody" class="workspace-body" :style="workspaceGridStyle">
      <GitChangeExplorer
        v-if="panelLayout.visible.changes"
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

      <WorkbenchSplitHandle
        v-if="panelLayout.visible.changes && (panelLayout.visible.diff || panelLayout.visible.commit)"
        :label="leadingHandleLabel"
        :value="leadingHandleValue"
        :min="leadingHandleMin"
        :max="leadingHandleMax"
        @resize="resizeLeadingPanel"
        @reset="resetLeadingPanelWidth"
      />

      <GitDiffViewer
        v-if="panelLayout.visible.diff"
        class="inline-diff"
        :has-snapshot="Boolean(snapshot)"
        :active-file="selectedFile"
        :diff-text="currentDiff"
        :loading="diffLoading"
        :current-mode="diffMode"
        @update:mode="diffMode = $event"
      />

      <WorkbenchSplitHandle
        v-if="panelLayout.visible.diff && panelLayout.visible.commit"
        :label="t('gitAssistant.layout.resizeCommit')"
        :value="effectiveCommitWidth"
        :min="PANEL_LIMITS.commit.min"
        :max="commitMaxWidth"
        @resize="resizeCommitPanel"
        @reset="resetPanelWidth('commit')"
      />

      <aside v-if="panelLayout.visible.commit" class="commit-inspector">
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

        <section class="commit-side">
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
              <button class="ai-action" type="button" :disabled="!snapshot" @click="openReviewPanel">
                {{ t('gitAssistant.ai.reviewCode') }}
              </button>
            </div>
          </section>
          <div v-if="aiLoading" class="ai-progress">
            <span class="ai-progress__dot"></span>
            <span>{{ promptGenerationStep }}</span>
          </div>
        </section>
      </aside>

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
        :mask-closable="true"
      >
        <WorkbenchSheet
          size="wide"
          icon="solar:folder-with-files-linear"
          :title="t('gitAssistant.repo.recentRepoManage')"
          :description="t('gitAssistant.repo.recentRepoManageHint')"
          :close-label="t('gitAssistant.prompt.close')"
          @close="recentRepoManagerOpen = false"
        >
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
        </WorkbenchSheet>
      </NModal>

      <!-- Filterable selects teleport their menus; keep this modal focus policy intact. -->
      <NModal v-model:show="branchSelectorOpen" class="repository-action-modal" :auto-focus="false" :mask-closable="true" :trap-focus="false">
        <WorkbenchSheet
          icon="solar:branching-paths-down-linear"
          :title="t('gitAssistant.repo.manageBranches')"
          :description="snapshot?.branch ? `${t('gitAssistant.repo.branch')}：${snapshot.branch}` : ''"
          :close-label="t('gitAssistant.prompt.close')"
          :busy="branchLoading"
          @close="branchSelectorOpen = false"
        >
          <label class="repository-action-field">
            <span>{{ t('gitAssistant.repo.selectBranch') }}</span>
            <NSelect
              v-model:value="branchSelectionValue"
              :options="branchOptions"
              :loading="branchLoading"
              filterable
              to="body"
              :placeholder="branchLoading ? t('gitAssistant.repo.loadingBranches') : t('gitAssistant.repo.selectBranch')"
              @update:value="value => handleBranchSelection(String(value ?? ''))"
            />
          </label>
          <label class="repository-action-field">
            <span>{{ t('gitAssistant.repo.createBranch') }}</span>
            <div class="repository-action-create">
              <NInput
                v-model:value="newBranchDraft"
                :placeholder="t('gitAssistant.repo.branchNamePlaceholder')"
                @keydown.enter="handleCreateBranch"
              />
              <NButton
                type="primary"
                :disabled="!newBranchDraft.trim() || branchLoading"
                :loading="branchLoading"
                @click="handleCreateBranch"
              >
                {{ t('gitAssistant.repo.createBranch') }}
              </NButton>
            </div>
          </label>
          <template #footer>
            <NButton @click="branchSelectorOpen = false">{{ t('common.dismiss') }}</NButton>
          </template>
        </WorkbenchSheet>
      </NModal>

      <NModal v-model:show="reviewPanelOpen" class="review-modal" :mask-closable="false" :auto-focus="false">
        <GitReviewPanel
          :key="reviewPanelRevision"
          :session="reviewStore.activeSession"
          :history="reviewStore.history"
          :rules="reviewStore.rules"
          :selected-count="selectedFileViews.length"
          :has-model="Boolean(reviewModel)"
          :running="reviewStore.running"
          :loading="reviewStore.loading"
          :error="reviewStore.error"
          @close="reviewPanelOpen = false"
          @start="startCodeReview"
          @cancel="reviewStore.cancel()"
          @open-session="reviewStore.open"
          @open-file="openReviewFindingFile"
          @finding-status="reviewStore.setFindingStatus"
          @save-rule="reviewStore.saveRule"
          @delete-rule="reviewStore.deleteRule"
        />
      </NModal>

      <NModal v-model:show="conflictDialogOpen" class="conflict-modal" :mask-closable="true">
        <WorkbenchSheet
          icon="solar:danger-triangle-linear"
          :title="t('gitAssistant.conflict.title')"
          :description="t('gitAssistant.conflict.dialogHint')"
          :close-label="t('gitAssistant.prompt.close')"
          :busy="conflictLoading"
          @close="conflictDialogOpen = false"
        >
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

          <template #footer>
            <div class="sheet-footer-split">
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
            </div>
          </template>
        </WorkbenchSheet>
      </NModal>

      <!-- Same focus policy as the branch modal for the filterable merge-source select. -->
      <NModal v-model:show="mergeDialogOpen" class="repository-action-modal" :auto-focus="false" :mask-closable="true" :trap-focus="false">
        <WorkbenchSheet
          icon="solar:branching-paths-up-linear"
          :title="t('gitAssistant.repo.mergeBranch')"
          :description="t('gitAssistant.repo.mergeInto', { branch: snapshot?.branch ?? '--' })"
          :close-label="t('gitAssistant.prompt.close')"
          :busy="mergeLoading"
          @close="mergeDialogOpen = false"
        >
          <label class="repository-action-field">
            <span>{{ t('gitAssistant.repo.mergeSource') }}</span>
            <NSelect
              v-model:value="mergeSourceValue"
              :options="mergeSourceOptions"
              filterable
              to="body"
              :placeholder="t('gitAssistant.repo.mergeSourcePlaceholder')"
            />
          </label>
          <label class="repository-action-field">
            <span>{{ t('gitAssistant.repo.mergeMode') }}</span>
            <NSelect v-model:value="mergeMode" :options="mergeModeOptions" to="body" />
          </label>
          <p class="repository-action-hint">{{ t('gitAssistant.repo.mergeCleanWorktreeHint') }}</p>
          <template #footer>
            <NButton @click="mergeDialogOpen = false">{{ t('common.dismiss') }}</NButton>
            <NButton type="primary" :loading="mergeLoading" :disabled="!mergeSourceValue" @click="handleMergeBranch">
              {{ t('gitAssistant.repo.mergeStart') }}
            </NButton>
          </template>
        </WorkbenchSheet>
      </NModal>

      <NModal v-model:show="repositorySetupOpen" class="repository-action-modal" :mask-closable="true">
        <WorkbenchSheet
          :icon="repositorySetupMode === 'clone' ? 'solar:download-square-linear' : 'solar:folder-add-linear'"
          :title="repositorySetupMode === 'clone' ? t('gitAssistant.repo.cloneRepository') : t('gitAssistant.repo.initRepository')"
          :description="t('gitAssistant.repo.repositorySetupHint')"
          :close-label="t('gitAssistant.prompt.close')"
          :busy="repositoryLoading"
          @close="repositorySetupOpen = false"
        >
          <NInput v-if="repositorySetupMode === 'clone'" v-model:value="cloneUrlDraft" :placeholder="t('gitAssistant.repo.cloneUrlPlaceholder')" />
          <div class="repository-path-picker">
            <NInput :value="repositoryPathDraft" readonly :placeholder="t('gitAssistant.repo.repositoryPathPlaceholder')" />
            <NButton @click="pickRepositoryTarget">{{ t('gitAssistant.repo.chooseDirectory') }}</NButton>
          </div>
          <template #footer>
            <NButton @click="repositorySetupOpen = false">{{ t('common.dismiss') }}</NButton>
            <NButton type="primary" :disabled="repositoryLoading || !repositoryPathDraft || (repositorySetupMode === 'clone' && !cloneUrlDraft.trim())" @click="handleRepositorySetup">
              {{ repositorySetupMode === 'clone' ? t('gitAssistant.repo.cloneRepository') : t('gitAssistant.repo.initRepository') }}
            </NButton>
          </template>
        </WorkbenchSheet>
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
            <span>{{ formatHistoryTime(entry.createdAt) }} · {{ historySourceLabel(entry.source) }}</span>
            <strong>{{ entry.title }}</strong>
            <p v-if="entry.body">{{ entry.body }}</p>
            <small>{{ entry.repoName }} · {{ t('gitAssistant.history.fileCount', { count: entry.selectedFileCount }) }}</small>
          </div>
          <button type="button" @click="restoreCommitMessage(entry)">{{ t('gitAssistant.history.restore') }}</button>
        </article>
        <div v-if="!filteredCommitMessageHistory.length" class="history-empty">
          {{ t('gitAssistant.history.empty') }}
        </div>
      </section>
    </WorkbenchDrawer>

    <WorkbenchDrawer
      v-if="repositoryRulesOpen"
      size="wide"
      :title="t('gitAssistant.repositoryRules.title')"
      :description="t('gitAssistant.repositoryRules.description')"
      :close-label="t('gitAssistant.prompt.close')"
      @close="repositoryRulesOpen = false"
    >
      <RepositoryRulesEditor :repo-path="displayRepoPath" />
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
import { computed, defineAsyncComponent, onMounted, onUnmounted, ref, watch } from 'vue'
import { NButton, NCheckbox, NInput, NModal, NSelect } from 'naive-ui'
import { useLocale } from '@/hooks/useLocale'
import {
  openGitFileExternal,
  revertGitFile,
} from '@/services/git/git-service'
import { openGitLogWindow } from '@/services/git/git-log-window'
import { openGitDiffWindow } from '@/services/git/git-diff-window'
import { useAiSettingsStore } from '@/stores/ai-settings'
import { parseGitStatusList } from '@/utils/git-status'
import GitChangeExplorer from './components/GitChangeExplorer.vue'
import GitCommandDialog from './components/GitCommandDialog.vue'
import GitCommitAssistant from './components/GitCommitAssistant.vue'
import GitStatusBar from './components/GitStatusBar.vue'
import GitReviewPanel from './components/GitReviewPanel.vue'
import RepositoryRulesEditor from './components/RepositoryRulesEditor.vue'
import WorkbenchDrawer from '@/components/workbench/WorkbenchDrawer.vue'
import WorkbenchModalPanel from '@/components/workbench/WorkbenchModalPanel.vue'
import WorkbenchSplitHandle from '@/components/workbench/WorkbenchSplitHandle.vue'
import WorkbenchSheet from '@/components/workbench/WorkbenchSheet.vue'
import { GIT_REPO_STORAGE_KEY } from './git-assistant.config'
import { useGitSnapshot, useGitDiff, useGitRemote, useGitCommit, useGitWorkspaceLayout, useGitReview, useGitBranchOptions, useGitRepositorySetup, useGitBranchActions, useGitStaging, useGitConflictDialog } from '@/composables/git-assistant'
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
  currentDiff, diffMode, diffLoading, showDiff, activeFileRaw, loadDiffForFile,
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
const {
  PANEL_LIMITS, workspaceBody, panelLayout, workspaceGridStyle, effectiveCommitWidth, commitMaxWidth,
  leadingHandleLabel, leadingHandleValue, leadingHandleMin, leadingHandleMax,
  togglePanel, resetPanelLayout, resetPanelWidth, resizeCommitPanel, resizeLeadingPanel, resetLeadingPanelWidth,
  observeWorkspaceBody, disconnectWorkspaceObserver,
} = useGitWorkspaceLayout(t)

const keyword = ref('')
const statusFilter = ref<GitAssistantStatusFilter>('all')
const recommendedOnly = ref(false)
const recentRepoManagerOpen = ref(false)
const repositoryRulesOpen = ref(false)
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
      scoreBreakdown: reviewScore?.scoreBreakdown ?? [],
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
const {
  conflictDialogOpen, conflictSelectedPaths,
  openConflictDialog, toggleConflictSelection, handleMarkConflictPathsResolved,
} = useGitConflictDialog({
  getConflictedFilePaths: () => conflictedFiles.value.map(file => file.path),
  markResolved: handleMarkResolved,
})
const {
  reviewStore, reviewScores, reviewScoring, reviewPanelOpen, reviewPanelRevision, reviewScoreProgress,
  reviewModel, selectedFileViews, toggleReviewSelection, setReviewSelection, loadReviewScores,
  openReviewPanel, startCodeReview, openReviewFindingFile, startReviewListeners, stopReviewListeners,
} = useGitReview({
  getRepositoryPath: () => displayRepoPath.value,
  getRepositoryRoot: () => snapshot.value?.repoRoot || displayRepoPath.value,
  getFiles: () => allFiles.value,
  getSnapshotStatus: () => snapshot.value?.status,
  getModel: () => aiSettings.getModelForTask('light-review'),
  reviewSelectedRaws,
  onError: message => { error.value = message },
  openFileDiff: handleOpenDiff,
  errorFallback: () => t('gitAssistant.errorFallback'),
})
const { branchOptions, currentBranchValue, mergeSourceOptions, mergeModeOptions } = useGitBranchOptions(snapshot, t)
const {
  branchLoading, branchSelectorOpen, branchSelectionValue, newBranchDraft,
  mergeDialogOpen, mergeLoading, mergeSourceValue, mergeMode,
  openBranchSelector, openMergeDialog, handleCreateBranch, handleMergeBranch, handleBranchSelection,
} = useGitBranchActions({
  getRepositoryPath: () => displayRepoPath.value,
  getCurrentBranchValue: () => currentBranchValue.value,
  snapshot,
  setError: message => { error.value = message },
  translate: t,
  loadSnapshot: loadSnapshotByPath,
  startGitCommand,
  finishGitCommand,
  failGitCommand,
})
const { handleStageFiles } = useGitStaging({
  getRepositoryPath: () => displayRepoPath.value,
  getFiles: () => allFiles.value,
  setError: message => { error.value = message },
  translate: t,
  loadSnapshot: loadSnapshotByPath,
  startGitCommand,
  finishGitCommand,
  failGitCommand,
})
const {
  repositorySetupOpen, repositorySetupMode, repositoryLoading, repositoryPathDraft, cloneUrlDraft,
  openRepositorySetup, pickRepositoryTarget, handleRepositorySetup,
} = useGitRepositorySetup({
  repoPath,
  setError: message => { error.value = message },
  translate: t,
  loadSnapshot: loadSnapshotByPath,
  finishGitCommand,
})
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

async function handleOpenExternalFile(filePath: string) {
  if (!displayRepoPath.value) return
  try { await openGitFileExternal(displayRepoPath.value, filePath) } catch (err) {
    console.error(err); error.value = err instanceof Error ? err.message : t('gitAssistant.errorFallback')
  }
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

watch(showConflictTools, (active, wasActive) => {
  if (active && !wasActive) openConflictDialog()
  if (!active) conflictDialogOpen.value = false
})

watch(selectedFile, file => {
  if (diffMode.value === 'head') return
  if (!file) { diffMode.value = 'unstaged'; return }
  diffMode.value = file.unstaged ? 'unstaged' : 'staged'
}, { immediate: true })

watch([selectedFile, diffMode], async ([file, mode]) => {
  if (!file || !displayRepoPath.value) { currentDiff.value = ''; return }
  await loadDiffForFile(file, mode)
}, { immediate: true })

// MainLayout keeps this view alive. Route deactivation must not refresh the snapshot:
// doing so invalidates in-flight scoring/AI requests and discards their results.
onMounted(async () => {
  await startReviewListeners()
  loadRecentRepos()
  loadCommitMessageHistory()
  observeWorkspaceBody()
  const saved = localStorage.getItem(GIT_REPO_STORAGE_KEY)
  if (!saved) return
  repoPath.value = saved
  await loadSnapshotByPath(saved)
})

onUnmounted(() => {
  stopReviewListeners()
  disconnectWorkspaceObserver()
})
</script>
<style scoped lang="scss" src="./GitAssistantView.scss"></style>
