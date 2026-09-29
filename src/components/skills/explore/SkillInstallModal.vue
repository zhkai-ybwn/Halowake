<template>
  <NModal
    v-model:show="exploreStore.installModalVisible"
    :mask-closable="!isInstalling"
    :close-on-esc="!isInstalling"
    transform-origin="center"
  >
    <div class="install-sheet-modal" role="dialog" :aria-label="modalTitle">
      <!-- Sheet Header -->
      <header class="sheet-header">
        <div class="header-icon-box">
          <Icon
            :icon="isPipeline ? 'solar:rocket-2-linear' : 'solar:box-minimalistic-linear'"
            class="header-icon"
          />
        </div>
        <div class="header-text">
          <h3 class="sheet-title">{{ modalTitle }}</h3>
          <p class="sheet-target-name">
            <span class="highlight">{{ targetName }}</span>
            <span v-if="isPipeline" class="pipeline-count-tag">
              {{ t('skills.pipelineSkillsCount', { count: pipelineSkillsCount }) }}
            </span>
          </p>
        </div>
        <button
          type="button"
          class="sheet-close-btn"
          :disabled="isInstalling"
          @click="exploreStore.closeInstallDialog"
        >
          <Icon icon="solar:close-circle-linear" />
        </button>
      </header>

      <div v-if="exploreStore.pendingSkill" class="install-source-note">
        <Icon icon="solar:info-circle-linear" />
        <span>
          {{ exploreStore.pendingSkill.sourceId === 'agent-skills-corpus' ? t('skills.review.install.unreviewedSource') : t('skills.review.install.source') }}
          {{ exploreStore.pendingSkill.gitUrl }}<template v-if="exploreStore.pendingSkill.subPath">/{{ exploreStore.pendingSkill.subPath }}</template>
        </span>
      </div>

      <!-- Sheet Form Body -->
      <div class="sheet-body">
        <!-- 单独子技能模式提示横幅 -->
        <div v-if="exploreStore.pendingSubSkill && exploreStore.pendingSkill" class="subskill-banner">
          <Icon icon="solar:box-minimalistic-bold" class="banner-icon" />
          <div class="banner-text">
            <span class="banner-title">{{ t('skills.review.install.subskillMode') }}</span>
            <span class="banner-sub">
              {{ t('skills.review.install.subskillHint', { bundle: exploreStore.pendingSkill.titleZh, skill: exploreStore.pendingSubSkill.titleZh }) }}
            </span>
          </div>
        </div>

        <!-- 多技能合集时的子技能勾选清单 (仅在非单装模式且为Bundle时展示) -->
        <div
          v-else-if="exploreStore.pendingSkill?.isBundle && exploreStore.pendingSkill.subSkills?.length"
          class="form-section bundle-selection-section"
        >
          <div class="section-label-row">
            <label class="section-title">
              {{ t('skills.review.install.bundleSelection', { selected: exploreStore.selectedBundleSubSkillIds.length, total: exploreStore.pendingSkill.subSkills.length }) }}
            </label>
            <div class="bundle-quick-actions">
              <button type="button" class="quick-link-btn" @click="selectAllBundleSubs">{{ t('skills.review.install.selectAll') }}</button>
              <span class="divider">·</span>
              <button type="button" class="quick-link-btn" @click="clearBundleSubs">{{ t('skills.review.install.clear') }}</button>
            </div>
          </div>

          <div class="subskills-checklist">
            <label
              v-for="sub in exploreStore.pendingSkill.subSkills"
              :key="sub.id"
              class="subskill-check-item"
              :class="{ checked: isSubChecked(sub.id) }"
            >
              <input
                type="checkbox"
                class="hidden-input"
                :checked="isSubChecked(sub.id)"
                @change="toggleSubChecked(sub.id)"
              />
              <div class="check-box-icon">
                <Icon :icon="isSubChecked(sub.id) ? 'solar:check-square-bold' : 'solar:square-dashed-linear'" />
              </div>
              <div class="sub-check-info">
                <div class="sub-check-names">
                  <span class="sub-check-code">{{ sub.name }}</span>
                  <span class="sub-check-title">{{ sub.titleZh }}</span>
                </div>
                <p class="sub-check-desc">{{ sub.description }}</p>
              </div>
            </label>
          </div>
        </div>

        <!-- 核心 3 种目标模式选择 -->
        <div class="form-section">
          <div class="section-label-row">
            <label class="section-title">{{ t('skills.review.install.destination') }}</label>
          </div>

          <div class="target-modes-grid">
            <!-- 模式 1: 放入当前项目内 -->
            <div
              class="target-mode-card"
              :class="{ active: localPref.targetMode === 'project' }"
              @click="localPref.targetMode = 'project'"
            >
              <div class="card-radio">
                <span v-if="localPref.targetMode === 'project'" class="radio-dot"></span>
              </div>
              <div class="card-info">
                <div class="card-title">
                  <Icon icon="solar:folder-with-files-linear" class="title-glyph" />
                  <span>{{ t('skills.review.install.project') }}</span>
                </div>
                <p class="card-desc">{{ t('skills.review.install.projectHint') }}</p>
              </div>
            </div>

            <!-- 模式 2: 对应特定 Agent -->
            <div
              class="target-mode-card"
              :class="{ active: localPref.targetMode === 'specific_agent' }"
              @click="localPref.targetMode = 'specific_agent'"
            >
              <div class="card-radio">
                <span v-if="localPref.targetMode === 'specific_agent'" class="radio-dot"></span>
              </div>
              <div class="card-info">
                <div class="card-title">
                  <Icon icon="solar:devices-linear" class="title-glyph" />
                  <span>{{ t('skills.review.install.agent') }}</span>
                </div>
                <p class="card-desc">{{ t('skills.review.install.agentHint') }}</p>
              </div>
            </div>

            <!-- 模式 3: 全局通用 -->
            <div
              class="target-mode-card"
              :class="{ active: localPref.targetMode === 'global' }"
              @click="localPref.targetMode = 'global'"
            >
              <div class="card-radio">
                <span v-if="localPref.targetMode === 'global'" class="radio-dot"></span>
              </div>
              <div class="card-info">
                <div class="card-title">
                  <Icon icon="solar:global-linear" class="title-glyph" />
                  <span>{{ t('skills.review.install.global') }}</span>
                </div>
                <p class="card-desc">{{ t('skills.review.install.globalHint') }}</p>
              </div>
            </div>
          </div>
        </div>

        <!-- 模式 1 补充设置：项目路径与 Git 策略 -->
        <div v-if="localPref.targetMode === 'project'" class="sub-config-box">
          <div class="project-path-display">
            <span class="path-label">{{ t('skills.review.install.projectPath') }}</span>
            <span class="path-value" :title="activeProjectCwd">
              {{ activeProjectCwd || t('skills.review.install.noProject') }}
            </span>
          </div>

          <div class="gitignore-choice-row">
            <label class="gitignore-label">
              <input v-model="localPref.addToGitignore" type="checkbox" class="lumina-checkbox" />
              <span class="checkbox-text-group">
                <span class="checkbox-title">{{ t('skills.review.install.gitignore') }}</span>
                <span class="checkbox-desc">{{ t('skills.review.install.gitignoreHint') }}</span>
              </span>
            </label>
          </div>
        </div>

        <!-- 模式 2 补充设置：多 Agent 勾选与全选智能校验 -->
        <div v-if="localPref.targetMode === 'specific_agent'" class="sub-config-box">
          <div class="agent-select-header">
            <span class="path-label">{{ t('skills.review.install.chooseAgents') }}</span>
          </div>

          <div class="agents-checkbox-row">
            <label
              v-for="agent in availableAgents"
              :key="agent.key"
              class="agent-check-pill"
              :class="{ checked: isAgentSelected(agent.key) }"
            >
              <input
                type="checkbox"
                class="hidden-input"
                :checked="isAgentSelected(agent.key)"
                @change="toggleAgent(agent.key)"
              />
              <Icon :icon="agent.icon" class="agent-icon" />
              <span>{{ agent.name }}</span>
            </label>
          </div>

          <!-- 全选 Agent 智能提示校验 -->
          <div v-if="isAllAgentsSelected" class="all-agents-hint-banner">
            <Icon icon="solar:lightbulb-linear" class="hint-icon" />
            <div class="hint-text">
              <span>{{ t('skills.review.install.allAgentsHint') }}</span>
            </div>
            <button
              type="button"
              class="switch-global-btn"
              @click="localPref.targetMode = 'global'"
            >
              {{ t('skills.review.install.setGlobal') }}
            </button>
          </div>
        </div>

        <!-- 模式 3 补充说明：全局目录 -->
        <div v-if="localPref.targetMode === 'global'" class="sub-config-box">
          <div class="project-path-display">
            <span class="path-label">{{ t('skills.review.install.globalPath') }}</span>
            <span class="path-value">~/.agents/skills/{{ targetId }}</span>
          </div>
          <p class="global-hint-text">
            {{ t('skills.review.install.globalPathHint') }}
          </p>
        </div>

        <!-- 偏好记忆勾选 -->
        <div class="remember-row">
          <label class="remember-label">
            <input v-model="localPref.remember" type="checkbox" class="lumina-checkbox" />
            <span>{{ t('skills.review.install.remember') }}</span>
          </label>
        </div>
      </div>

      <div v-if="exploreStore.installProgress" class="install-feedback" :class="{ 'install-feedback--error': exploreStore.installError }" role="status" aria-live="polite">
        <div class="install-feedback__heading">
          <Icon :icon="exploreStore.installError ? 'solar:danger-circle-linear' : 'solar:download-minimalistic-linear'" />
          <div>
            <strong>{{ exploreStore.installError ? t('skills.review.install.incomplete') : t('skills.review.install.progress', { current: exploreStore.installCurrent, total: exploreStore.installTotal }) }}</strong>
            <span>{{ exploreStore.installProgress.message }}</span>
          </div>
          <small v-if="exploreStore.installProgress.percent !== null && !exploreStore.installError">{{ exploreStore.installProgress.percent }}%</small>
        </div>
        <div v-if="isInstalling" class="install-feedback__track" :class="{ 'install-feedback__track--indeterminate': exploreStore.installProgress.percent === null }">
          <span :style="exploreStore.installProgress.percent === null ? undefined : { width: `${exploreStore.installProgress.percent}%` }"></span>
        </div>
        <p v-if="exploreStore.installError" class="install-feedback__error">{{ exploreStore.installError }}</p>
      </div>

      <!-- Sheet Footer Actions -->
      <footer class="sheet-footer">
        <button
          type="button"
          class="sheet-btn sheet-btn--ghost"
          :disabled="isInstalling"
          @click="exploreStore.closeInstallDialog"
        >
          {{ t('common.cancel', '取消') }}
        </button>

        <button
          type="button"
          class="sheet-btn sheet-btn--primary"
          :disabled="isInstalling || isConfirmDisabled"
          @click="handleConfirm"
        >
          <Icon
            :icon="isInstalling ? 'solar:refresh-linear' : 'solar:cloud-download-linear'"
            :class="{ 'spin-glyph': isInstalling }"
          />
          <span>{{ isInstalling ? t('skills.installingBtn') : t('skills.installConfirmBtn') }}</span>
        </button>
      </footer>
    </div>
  </NModal>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { NModal, useMessage } from 'naive-ui'
import { Icon } from '@iconify/vue'
import { useI18n } from 'vue-i18n'
import { useSkillExploreStore } from '@/stores/skill-explore'
import { useSkillsStore } from '@/stores/skills'
import { GIT_REPO_STORAGE_KEY } from '@/constants/git'
import type { InstallPreference } from '@/types/skills'

const { t } = useI18n()
const message = useMessage()
const exploreStore = useSkillExploreStore()
const skillsStore = useSkillsStore()

const localPref = ref<InstallPreference>({ ...exploreStore.installPreference })

watch(
  () => exploreStore.installModalVisible,
  visible => {
    if (visible) {
      localPref.value = { ...exploreStore.installPreference }
    }
  }
)

const activeProjectCwd = computed(() => {
  return localStorage.getItem(GIT_REPO_STORAGE_KEY) || ''
})

const isPipeline = computed(() => !!exploreStore.pendingPipeline)

const targetId = computed(() => {
  if (exploreStore.pendingPipeline) return exploreStore.pendingPipeline.id
  if (exploreStore.pendingSkill) return exploreStore.pendingSkill.id
  return 'skill'
})

const targetName = computed(() => {
  if (exploreStore.pendingPipeline) {
    return exploreStore.pendingPipeline.title
  }
  if (exploreStore.pendingSubSkill && exploreStore.pendingSkill) {
    return `${exploreStore.pendingSubSkill.titleZh} (${exploreStore.pendingSubSkill.name})`
  }
  if (exploreStore.pendingSkill) {
    return `${exploreStore.pendingSkill.name} (${exploreStore.pendingSkill.titleZh})`
  }
  return ''
})

const modalTitle = computed(() => {
  if (isPipeline.value) return t('skills.review.install.pipelineTitle')
  if (exploreStore.pendingSubSkill) return t('skills.review.install.subskillTitle')
  if (exploreStore.pendingSkill?.isBundle) return t('skills.review.install.bundleTitle')
  return t('skills.installDialogTitle')
})

const pipelineSkillsCount = computed(() => {
  return exploreStore.pendingPipeline?.skills.length || 0
})

const isInstalling = computed(() => exploreStore.installingIds.length > 0)

const availableAgents = [
  { key: 'antigravity', name: 'Antigravity', icon: 'solar:magic-stick-3-linear' },
  { key: 'claude', name: 'Claude Code', icon: 'solar:code-square-linear' },
  { key: 'codex', name: 'Codex', icon: 'solar:code-linear' },
  { key: 'cursor', name: 'Cursor', icon: 'solar:code-file-linear' },
  { key: 'trae', name: 'Trae', icon: 'solar:laptop-minimalistic-linear' },
]

const isAllAgentsSelected = computed(() => {
  return availableAgents.every(a => localPref.value.targetAgents.includes(a.key))
})

const isConfirmDisabled = computed(() => {
  if (localPref.value.targetMode === 'project' && !activeProjectCwd.value) {
    return true
  }
  if (localPref.value.targetMode === 'specific_agent' && localPref.value.targetAgents.length === 0) {
    return true
  }
  if (exploreStore.pendingSkill?.isBundle && !exploreStore.pendingSubSkill && exploreStore.selectedBundleSubSkillIds.length === 0) {
    return true
  }
  return false
})

function isSubChecked(subId: string): boolean {
  return exploreStore.selectedBundleSubSkillIds.includes(subId)
}

function toggleSubChecked(subId: string) {
  const cur = exploreStore.selectedBundleSubSkillIds
  if (cur.includes(subId)) {
    exploreStore.selectedBundleSubSkillIds = cur.filter(id => id !== subId)
  } else {
    exploreStore.selectedBundleSubSkillIds.push(subId)
  }
}

function selectAllBundleSubs() {
  if (exploreStore.pendingSkill?.subSkills) {
    exploreStore.selectedBundleSubSkillIds = exploreStore.pendingSkill.subSkills.map(s => s.id)
  }
}

function clearBundleSubs() {
  exploreStore.selectedBundleSubSkillIds = []
}

function isAgentSelected(agentKey: string): boolean {
  return localPref.value.targetAgents.includes(agentKey)
}

function toggleAgent(agentKey: string) {
  const cur = localPref.value.targetAgents
  if (cur.includes(agentKey)) {
    if (cur.length > 1) {
      localPref.value.targetAgents = cur.filter(a => a !== agentKey)
    }
  } else {
    localPref.value.targetAgents.push(agentKey)
  }
}

async function handleConfirm() {
  exploreStore.updateInstallPreference(localPref.value)
  const currentCwd = activeProjectCwd.value || undefined

  const res = await exploreStore.executeInstall(currentCwd)
  if (res.success) {
    const scan = await skillsStore.scan(currentCwd)
    if (skillsStore.errorMessage) {
      message.warning(t('skills.review.install.scanFailed', { error: skillsStore.errorMessage }))
    } else {
      const scannedPaths = new Set(skillsStore.skills.map(skill => skill.path.replace(/\\/g, '/').toLowerCase()))
      const missedPaths = res.installedPaths.filter(path => !scannedPaths.has(path.replace(/\\/g, '/').toLowerCase()))
      if (missedPaths.length) {
        message.warning(t('skills.review.install.pathsMissed', { count: missedPaths.length }))
      } else {
        message.success(t('skills.review.install.scanned', { message: res.message, count: scan.total }))
      }
    }
    const installedPathSet = new Set(res.installedPaths.map(path => path.replace(/\\/g, '/').toLowerCase()))
    const matchingNames = new Set(skillsStore.skills
      .filter(skill => installedPathSet.has(skill.path.replace(/\\/g, '/').toLowerCase()))
      .map(skill => skill.name))
    skillsStore.revealInstalledSkills(matchingNames.size === 1 ? [...matchingNames][0] : '')
    exploreStore.activeMainTab = 'installed'
  } else {
    message.error(t('skills.review.install.failedHint'))
  }
}
</script>

<style scoped lang="scss">
.install-feedback {
  background: var(--lumina-surface-secondary);
  border-top: 0.5px solid var(--lumina-separator);
  display: grid;
  gap: 8px;
  padding: 12px 20px;
}
.install-feedback__heading {
  align-items: center;
  display: flex;
  gap: 9px;

  > svg { color: var(--lumina-primary); font-size: 18px; flex-shrink: 0; }
  > div { display: grid; flex: 1; gap: 2px; min-width: 0; }
  strong { font-size: 12px; }
  span { color: var(--lumina-text-secondary); font-size: 11px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  small { color: var(--lumina-primary); font-size: 11px; font-variant-numeric: tabular-nums; }
}
.install-feedback__track {
  background: var(--lumina-fill-tertiary);
  border-radius: 999px;
  height: 4px;
  overflow: hidden;

  span { background: var(--lumina-primary); border-radius: inherit; display: block; height: 100%; transition: width 180ms ease; }
}
.install-feedback__track--indeterminate span { animation: install-progress 1.4s ease-in-out infinite; width: 35%; }
.install-feedback--error .install-feedback__heading > svg { color: var(--lumina-danger); }
.install-feedback__error {
  color: var(--lumina-danger);
  font-size: 11px;
  line-height: 1.5;
  margin: 0;
  max-height: 90px;
  overflow: auto;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}
@keyframes install-progress {
  from { transform: translateX(-110%); }
  to { transform: translateX(300%); }
}

.install-source-note {
  align-items: center;
  border-bottom: 0.5px solid var(--lumina-separator);
  color: var(--lumina-text-secondary);
  display: flex;
  font-size: 11px;
  gap: 7px;
  overflow-wrap: anywhere;
  padding: 8px 18px;
}

.install-sheet-modal {
  background: var(--lumina-surface-elevated, #ffffff);
  border: 0.5px solid var(--lumina-separator-strong);
  border-radius: var(--lumina-radius-lg, 12px);
  box-shadow: var(--lumina-shadow-lg, 0 24px 64px rgba(0, 0, 0, 0.28));
  color: var(--lumina-text);
  display: flex;
  flex-direction: column;
  max-width: 94vw;
  overflow: hidden;
  width: 580px;
}

.sheet-header {
  align-items: center;
  border-bottom: 0.5px solid var(--lumina-separator);
  display: flex;
  gap: 12px;
  padding: 16px 20px;
}

.header-icon-box {
  align-items: center;
  background: var(--lumina-primary-soft);
  border-radius: var(--lumina-radius-md, 8px);
  color: var(--lumina-primary);
  display: flex;
  flex-shrink: 0;
  font-size: 20px;
  height: 38px;
  justify-content: center;
  width: 38px;
}

.header-text {
  flex: 1;
  min-width: 0;
}

.sheet-title {
  color: var(--lumina-text);
  font-size: 14.5px;
  font-weight: 700;
  margin: 0;
}

.sheet-target-name {
  align-items: center;
  color: var(--lumina-text-secondary);
  display: flex;
  font-size: 12px;
  gap: 6px;
  margin: 2px 0 0;
}

.highlight {
  color: var(--lumina-primary);
  font-weight: 600;
}

.pipeline-count-tag {
  background: var(--lumina-primary-soft);
  border-radius: 4px;
  color: var(--lumina-primary);
  font-size: 11.5px;
  font-weight: 600;
  padding: 1px 6px;
}

.sheet-close-btn {
  background: none;
  border: none;
  border-radius: 4px;
  color: var(--lumina-text-tertiary);
  cursor: pointer;
  font-size: 18px;
  padding: 4px;
  transition: opacity 120ms ease;

  &:hover:not(:disabled) {
    color: var(--lumina-text);
  }
}

.sheet-body {
  display: flex;
  flex-direction: column;
  gap: 14px;
  max-height: 64vh;
  overflow-y: auto;
  padding: 16px 20px;
}

/* Sub-skill Banner */
.subskill-banner {
  align-items: flex-start;
  background: color-mix(in srgb, var(--lumina-primary) 10%, var(--lumina-surface-2));
  border: 0.5px solid color-mix(in srgb, var(--lumina-primary) 30%, transparent);
  border-radius: var(--lumina-radius-md, 8px);
  display: flex;
  gap: 10px;
  padding: 10px 12px;

  .banner-icon {
    color: var(--lumina-primary);
    flex-shrink: 0;
    font-size: 18px;
    margin-top: 1px;
  }

  .banner-text {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 2px;
  }

  .banner-title {
    color: var(--lumina-primary);
    font-size: 12.5px;
    font-weight: 600;
  }

  .banner-sub {
    color: var(--lumina-text-secondary);
    font-size: 11.5px;
    line-height: 1.4;
  }
}

/* Bundle Selection Section */
.bundle-selection-section {
  .bundle-quick-actions {
    align-items: center;
    display: flex;
    gap: 6px;

    .divider {
      color: var(--lumina-text-tertiary);
      font-size: 12px;
    }
  }

  .quick-link-btn {
    background: none;
    border: none;
    color: var(--lumina-primary);
    cursor: pointer;
    font-size: 11.5px;
    font-weight: 600;
    padding: 0;

    &:hover {
      text-decoration: underline;
    }
  }

  .subskills-checklist {
    background: var(--lumina-surface-secondary);
    border: 0.5px solid var(--lumina-separator);
    border-radius: var(--lumina-radius-md, 8px);
    display: flex;
    flex-direction: column;
    max-height: 180px;
    overflow-y: auto;
    padding: 6px 10px;
  }

  .subskill-check-item {
    align-items: flex-start;
    border-bottom: 0.5px solid var(--lumina-separator);
    cursor: pointer;
    display: flex;
    gap: 10px;
    padding: 8px 4px;
    transition: background 120ms ease;
    user-select: none;

    &:last-child {
      border-bottom: none;
    }

    &:hover {
      background: color-mix(in srgb, var(--lumina-primary) 5%, transparent);
    }

    &.checked .check-box-icon {
      color: var(--lumina-primary);
    }
  }

  .check-box-icon {
    color: var(--lumina-text-tertiary);
    flex-shrink: 0;
    font-size: 16px;
    margin-top: 1px;
    transition: color 120ms ease;
  }

  .sub-check-info {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
  }

  .sub-check-names {
    align-items: center;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .sub-check-code {
    color: var(--lumina-text);
    font-family: var(--lumina-font-mono, monospace);
    font-size: 12px;
    font-weight: 600;
  }

  .sub-check-title {
    color: var(--lumina-text-secondary);
    font-size: 12px;
    font-weight: 500;
  }

  .sub-check-desc {
    color: var(--lumina-text-secondary);
    font-size: 11.5px;
    line-height: 1.4;
    margin: 2px 0 0;
  }
}

.form-section {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.section-label-row {
  align-items: center;
  display: flex;
  justify-content: space-between;
}

.section-title {
  color: var(--lumina-text);
  font-size: 12px;
  font-weight: 600;
}

.target-modes-grid {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.target-mode-card {
  align-items: flex-start;
  background: var(--lumina-surface-secondary);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-md, 8px);
  cursor: pointer;
  display: flex;
  gap: 10px;
  padding: 10px 12px;
  transition: all 120ms ease;
  user-select: none;

  &:hover {
    border-color: var(--lumina-primary);
  }

  &.active {
    background: var(--lumina-primary-soft);
    border-color: var(--lumina-primary);

    .card-radio {
      border-color: var(--lumina-primary);
    }
  }
}

.card-radio {
  align-items: center;
  border: 1.5px solid var(--lumina-text-tertiary);
  border-radius: 50%;
  display: flex;
  flex-shrink: 0;
  height: 14px;
  justify-content: center;
  margin-top: 3px;
  width: 14px;
}

.radio-dot {
  background: var(--lumina-primary);
  border-radius: 50%;
  height: 6px;
  width: 6px;
}

.card-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  flex: 1;
}

.card-title {
  align-items: center;
  color: var(--lumina-text);
  display: flex;
  font-size: 12.5px;
  font-weight: 600;
  gap: 6px;

  .title-glyph {
    color: var(--lumina-primary);
    font-size: 14px;
  }
}

.card-desc {
  color: var(--lumina-text-secondary);
  font-size: 11.5px;
  line-height: 1.4;
  margin: 0;
}

.sub-config-box {
  background: color-mix(in srgb, var(--lumina-surface-elevated) 90%, transparent);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-md, 8px);
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 12px 14px;
}

.project-path-display {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 11.5px;

  .path-label {
    color: var(--lumina-text-secondary);
    font-weight: 500;
    flex-shrink: 0;
  }

  .path-value {
    color: var(--lumina-text);
    font-family: var(--lumina-font-mono, monospace);
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
}

.gitignore-choice-row {
  border-top: 0.5px dashed var(--lumina-separator);
  padding-top: 8px;
}

.gitignore-label {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  cursor: pointer;
  user-select: none;
}

.checkbox-text-group {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.checkbox-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--lumina-text);
}

.checkbox-desc {
  font-size: 11px;
  color: var(--lumina-text-secondary);
  line-height: 1.35;
}

.agents-checkbox-row {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.agent-check-pill {
  align-items: center;
  background: var(--lumina-surface-secondary);
  border: 0.5px solid var(--lumina-separator);
  border-radius: 6px;
  cursor: pointer;
  display: inline-flex;
  font-size: 11.5px;
  gap: 6px;
  padding: 5px 10px;
  transition: all 120ms ease;
  user-select: none;

  &:hover {
    border-color: var(--lumina-primary);
  }

  &.checked {
    background: var(--lumina-primary-soft);
    border-color: var(--lumina-primary);
    color: var(--lumina-primary);
    font-weight: 600;
  }

  .agent-icon {
    font-size: 14px;
  }
}

.all-agents-hint-banner {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-warning, #f59e0b) 12%, transparent);
  border: 0.5px solid color-mix(in srgb, var(--lumina-warning, #f59e0b) 30%, transparent);
  border-radius: 6px;
  color: var(--lumina-text);
  display: flex;
  gap: 8px;
  margin-top: 4px;
  padding: 8px 10px;

  .hint-icon {
    color: var(--lumina-warning, #f59e0b);
    font-size: 16px;
    flex-shrink: 0;
  }

  .hint-text {
    flex: 1;
    font-size: 11px;
    line-height: 1.35;
  }

  .switch-global-btn {
    background: var(--lumina-surface-elevated);
    border: 0.5px solid var(--lumina-separator);
    border-radius: 4px;
    color: var(--lumina-primary);
    cursor: pointer;
    font-size: 11.5px;
    font-weight: 600;
    padding: 3px 8px;
    white-space: nowrap;

    &:hover {
      background: var(--lumina-primary);
      color: #fff;
    }
  }
}

.global-hint-text {
  color: var(--lumina-text-secondary);
  font-size: 11.5px;
  line-height: 1.4;
  margin: 0;
}

.hidden-input {
  display: none;
}

.remember-row {
  padding-top: 4px;
}

.remember-label {
  align-items: center;
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: inline-flex;
  font-size: 11.5px;
  gap: 6px;
  user-select: none;
}

.sheet-footer {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-surface-elevated) 75%, transparent);
  border-top: 0.5px solid var(--lumina-separator);
  display: flex;
  gap: 10px;
  justify-content: flex-end;
  padding: 12px 20px;
}

.sheet-btn {
  align-items: center;
  border-radius: var(--lumina-radius-sm, 6px);
  cursor: pointer;
  display: inline-flex;
  font-size: 12px;
  font-weight: 500;
  gap: 6px;
  height: 30px;
  padding: 0 14px;
  transition: all 120ms ease;

  &--ghost {
    background: transparent;
    border: 0.5px solid var(--lumina-separator);
    color: var(--lumina-text);

    &:hover:not(:disabled) {
      background: var(--lumina-control-hover);
    }
  }

  &--primary {
    background: var(--lumina-primary);
    border: 0.5px solid transparent;
    color: #ffffff;
    font-weight: 600;

    &:hover:not(:disabled) {
      filter: brightness(1.08);
    }
  }

  &:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
}

.spin-glyph {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}
</style>
