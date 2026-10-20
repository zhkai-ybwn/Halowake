<template>
  <NDrawer
    v-model:show="exploreStore.detailDrawerVisible"
    :width="500"
    placement="right"
    :trap-focus="false"
    :auto-focus="false"
  >
    <NDrawerContent :native-scrollbar="true" closable @close="exploreStore.closeDetail">
      <!-- 头部：单技能模式 OR 流水线全流程玩法模式 -->
      <template #header>
        <!-- A. 流水线玩法指引头部 -->
        <div v-if="pipeline" class="drawer-header-content">
          <div class="header-icon-pill">
            <Icon :icon="pipeline.icon" />
          </div>
          <div class="header-title-box">
            <h3 class="drawer-skill-title">{{ pipeline.title }}</h3>
            <span class="drawer-subtitle">{{ t('skills.review.detail.pipelineSubtitle') }}</span>
          </div>
        </div>

        <!-- B. 单技能详情头部 -->
        <div v-else-if="skill" class="drawer-header-content">
          <SkillStageBadge :stage="skill.stage" />
          <div class="header-title-box">
            <h3 class="drawer-skill-title">{{ skill.name }}</h3>
            <span class="drawer-subtitle">{{ skill.titleZh }}</span>
          </div>
        </div>
      </template>

      <!-- ================= 模式 1: 流水线全流程实战指引 (Pipeline Playbook) ================= -->
      <div v-if="pipeline" class="drawer-body-wrap">
        <div class="drawer-section">
          <div class="scenario-banner">
            <Icon icon="solar:target-bold" class="target-icon" />
            <div class="scenario-content">
              <span class="scenario-title">{{ t('skills.review.detail.scenario') }}</span>
              <p class="scenario-text">{{ pipeline.targetScenario }}</p>
            </div>
          </div>
          <p class="pipeline-desc-text">{{ pipeline.description }}</p>
        </div>

        <!-- 全流程步骤协同玩法拆解 -->
        <div class="drawer-section">
          <div class="section-title-row">
            <Icon icon="solar:route-linear" class="sec-icon" />
            <h5 class="sec-heading">{{ t('skills.review.detail.playbook') }}</h5>
          </div>

          <div class="playbook-steps-list">
            <div
              v-for="(step, idx) in pipeline.playbook || []"
              :key="step.skillId"
              class="playbook-step-card"
            >
              <div class="step-card-header">
                <div class="step-badge-num">{{ idx + 1 }}</div>
                <SkillStageBadge :stage="step.stage" />
                <span class="step-role-title">{{ step.role }} ({{ step.skillName }})</span>
              </div>

              <div class="step-card-body">
                <p class="step-how-to-act">{{ step.howToAct }}</p>
                <div class="step-prompt-box">
                  <div class="prompt-header">
                    <span class="prompt-label">{{ t('skills.review.detail.prompt') }}</span>
                    <button
                      type="button"
                      class="copy-mini-btn"
                      @click="handleCopy(step.promptExample)"
                    >
                      <Icon icon="solar:copy-linear" />
                      <span>{{ t('skills.review.detail.copy') }}</span>
                    </button>
                  </div>
                  <pre class="prompt-snippet">{{ step.promptExample }}</pre>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- ================= 模式 2: 单技能详情与指引 ================= -->
      <div v-else-if="skill" class="drawer-body-wrap">
        <!-- 描述与功能说明 -->
        <div class="drawer-section">
          <p class="skill-full-desc">{{ skill.description }}</p>
        </div>

        <!-- 评分与信誉指标卡片 -->
        <!-- 评分与信誉指标卡片 (仅当存在客观真实指标时渲染) -->
        <div v-if="(isCommunityIndex && skill.stars > 0) || (!isCommunityIndex && (skill.metrics || skill.halowakeScore))" class="metrics-dashboard-card">
          <div class="dashboard-header">
            <span class="dashboard-title">{{ isCommunityIndex ? t('skills.review.detail.indexPopularity') : t('skills.detailScore') }}</span>
            <SkillScoreBadge v-if="skill.halowakeScore" :score="skill.halowakeScore" :badge="skill.badge" />
          </div>

          <div v-if="skill.metrics" class="metrics-grid">
            <div v-if="skill.metrics.githubStars !== undefined" class="metric-item">
              <span class="metric-label">GitHub Stars</span>
              <span class="metric-val">⭐ {{ formatNumber(skill.metrics.githubStars) }}</span>
            </div>
            <div v-if="!isCommunityIndex && skill.metrics.forks !== undefined" class="metric-item">
              <span class="metric-label">Forks</span>
              <span class="metric-val">{{ formatNumber(skill.metrics.forks) }}</span>
            </div>
            <div v-if="!isCommunityIndex && skill.metrics.lastCommitDaysAgo !== undefined" class="metric-item">
              <span class="metric-label">{{ t('skills.review.detail.lastUpdated') }}</span>
              <span class="metric-val highlight-green">{{ t('skills.review.detail.daysAgo', { count: skill.metrics.lastCommitDaysAgo }) }}</span>
            </div>
            <div v-if="!isCommunityIndex && skill.metrics.issueCloseRate !== undefined" class="metric-item">
              <span class="metric-label">{{ t('skills.review.detail.issueRate') }}</span>
              <span class="metric-val">{{ skill.metrics.issueCloseRate }}%</span>
            </div>
          </div>

          <div v-if="skill.metrics?.securityAuditPassed" class="security-status-row">
            <Icon icon="solar:shield-check-bold" class="security-glyph" />
            <span>{{ t('skills.review.detail.security') }}</span>
          </div>
        </div>

        <!-- 自定义分类与用户标签管理 -->
        <div class="drawer-section">
          <div class="section-title-row">
            <Icon icon="solar:tag-linear" class="sec-icon" />
            <h5 class="sec-heading">{{ t('skills.review.detail.myTags') }}</h5>
          </div>
          <div class="custom-tags-editor">
            <div class="existing-tags-row">
              <span
                v-for="tag in currentSkillCustomTags"
                :key="tag"
                class="user-tag-pill"
              >
                <span>#{{ tag }}</span>
                <button
                  type="button"
                  class="remove-tag-btn"
                  @click="exploreStore.removeSkillCustomCategory(skill.id, tag)"
                >
                  ×
                </button>
              </span>
              <span v-if="currentSkillCustomTags.length === 0" class="no-tags-hint">
                {{ t('skills.review.detail.noTags') }}
              </span>
            </div>

            <div class="add-tag-input-row">
              <input
                v-model="newTagInput"
                type="text"
                class="tag-input-field"
                :placeholder="t('skills.review.detail.tagName')"
                @keydown.enter.prevent="handleAddTag"
              />
              <button
                type="button"
                class="add-tag-btn"
                :disabled="!newTagInput.trim()"
                @click="handleAddTag"
              >
                {{ t('skills.review.detail.addCategory') }}
              </button>
            </div>
          </div>
        </div>

        <!-- 如何触发与使用指引 -->
        <div class="drawer-section">
          <div class="section-title-row">
            <Icon icon="solar:magic-stick-3-linear" class="sec-icon" />
            <h5 class="sec-heading">{{ t('skills.detailHowToUse') }}</h5>
          </div>
          <div class="instruction-box">
            <div v-if="skill.usageGuide" class="usage-summary">
              <p class="usage-provenance">{{ t('skills.review.detail.provenance', { date: new Date(skill.usageGuide.extractedAt).toLocaleDateString(locale) }) }}</p>
              <p v-if="skill.usageGuide.summary !== skill.usageGuide.trigger" class="usage-overview">{{ skill.usageGuide.summary }}</p>
              <div v-if="skill.usageGuide.trigger" class="usage-field">
                <strong>{{ t('skills.review.detail.trigger') }}</strong>
                <p>{{ skill.usageGuide.trigger }}</p>
              </div>
              <div v-if="Array.isArray(skill.usageGuide.steps) && skill.usageGuide.steps.length" class="usage-field">
                <strong>{{ t('skills.review.detail.steps') }}</strong>
                <ol><li v-for="(step, index) in skill.usageGuide.steps" :key="index">{{ step }}</li></ol>
              </div>
              <div v-if="skill.usageGuide.example" class="usage-field">
                <strong>{{ t('skills.review.detail.example') }}</strong>
                <p>{{ skill.usageGuide.example }}</p>
              </div>
            </div>
            <p v-else-if="sourceGuideLoading" class="instruction-text">{{ t('skills.review.detail.loadingGuide') }}</p>
            <template v-else-if="sourceGuide">
              <div v-if="sourceGuides.length > 1" class="source-guide-switch">
                <button
                  v-for="guide in sourceGuides"
                  :key="guide.kind"
                  type="button"
                  :class="{ active: selectedGuideKind === guide.kind }"
                  :aria-pressed="selectedGuideKind === guide.kind"
                  @click="selectedGuideKind = guide.kind"
                >{{ guide.kind }}</button>
              </div>
              <p class="instruction-text">{{ t('skills.review.detail.guideSource', { path: sourceGuide.path }) }}{{ sourceGuide.kind === 'README' && !sourceGuides.some(guide => guide.kind === 'SKILL.md') ? t('skills.review.detail.repositoryReadme') : '' }}</p>
              <div class="source-guide-preview" @click="handleGuideLinkClick" v-html="renderedGuide.html" />
              <p v-if="renderedGuide.truncated" class="instruction-text">{{ t('skills.review.detail.guideTruncated') }}</p>
            </template>
            <p v-else class="instruction-text">{{ sourceGuideError || t('skills.review.detail.noGuide') }}</p>
            <button v-if="!skill.usageGuide && !sourceGuideLoading && sourceGuideError" type="button" class="source-retry" @click="loadGuides(skill)">{{ t('skills.review.detail.retryGuide') }}</button>
            <p v-if="!skill.usageGuide && !sourceGuide && !sourceGuideLoading && !isUsageUnknown" class="instruction-text">{{ t('skills.review.detail.indexHint', { hint: skill.howToUse }) }}</p>
          </div>
        </div>

        <div class="drawer-section">
          <div class="section-title-row">
            <Icon icon="solar:link-round-linear" class="sec-icon" />
            <h5 class="sec-heading">{{ t('skills.review.detail.sourceAddresses') }}</h5>
          </div>
          <button type="button" class="source-address" :title="skill.gitUrl" @click="openSourceUrl(skill.gitUrl)">{{ skill.gitUrl }}</button>
          <button v-if="sourceDirectoryUrl" type="button" class="source-address" :title="sourceDirectoryUrl" @click="openSourceUrl(sourceDirectoryUrl)">{{ t('skills.review.detail.skillDirectory', { url: sourceDirectoryUrl }) }}</button>
          <button v-if="skill.usageGuide" type="button" class="source-address" :title="skill.usageGuide.sourceUrl" @click="openSourceUrl(skill.usageGuide.sourceUrl)">SKILL.md：{{ skill.usageGuide.sourceUrl }}</button>
          <button v-for="guide in sourceGuides" :key="guide.url" type="button" class="source-address" :title="guide.url" @click="openSourceUrl(guide.url)">{{ guide.kind }}：{{ guide.url }}</button>
        </div>

        <!-- 推荐提示词示例 -->
        <div v-if="skill.promptExample" class="drawer-section">
          <div class="section-title-row">
            <Icon icon="solar:chat-round-line-linear" class="sec-icon" />
            <h5 class="sec-heading">{{ t('skills.detailPromptExample') }}</h5>
            <button type="button" class="copy-pill-btn" @click="handleCopy(skill.promptExample)">
              <Icon icon="solar:copy-linear" />
              <span>{{ copied ? t('skills.review.detail.copied') : t('skills.review.detail.copy') }}</span>
            </button>
          </div>
          <pre class="prompt-code-block">{{ skill.promptExample }}</pre>
        </div>

        <!-- 最佳协同搭档 (Recommended With) -->
        <div v-if="skill.recommendedWith && skill.recommendedWith.length > 0" class="drawer-section">
          <div class="section-title-row">
            <Icon icon="solar:link-round-linear" class="sec-icon" />
            <h5 class="sec-heading">{{ t('skills.detailRecommendedWith') }}</h5>
          </div>
          <div class="companion-pills">
            <button
              v-for="comp in companionSkills"
              :key="comp.id"
              type="button"
              class="companion-chip"
              @click="exploreStore.openDetail(comp)"
            >
              <SkillStageBadge :stage="comp.stage" />
              <span class="comp-name">{{ comp.name }}</span>
              <span class="comp-score">★ {{ comp.halowakeScore }}</span>
            </button>
          </div>
        </div>

      </div>

      <!-- 抽屉底部动作栏 -->
      <template #footer>
        <!-- A. 流水线模式底部动作 -->
        <div v-if="pipeline" class="drawer-footer-actions">
          <div class="pipeline-status-text">
            <span>{{ t('skills.review.detail.pipelineCount', { count: pipeline.skills.length }) }}</span>
          </div>
          <button
            type="button"
            class="action-btn action-btn--primary"
            @click="exploreStore.openPipelineInstallDialog(pipeline)"
          >
            <Icon icon="solar:rocket-2-linear" />
            <span>{{ t('skills.review.detail.installPipeline') }}</span>
          </button>
        </div>

        <!-- B. 单技能模式底部动作 -->
        <div v-else-if="skill" class="drawer-footer-actions">
          <button
            type="button"
            class="star-toggle-btn"
            :class="{ active: skill.isStar }"
            @click="exploreStore.toggleStar(skill.id)"
          >
            <Icon :icon="skill.isStar ? 'solar:star-bold' : 'solar:star-linear'" />
            <span>{{ skill.isStar ? t('skills.detailStarredAction') : t('skills.detailStarAction') }}</span>
          </button>

          <button
            v-if="isInstalled"
            type="button"
            class="action-btn action-btn--installed"
            :title="installedTargetSummary ? `${t('skills.installedTag')}: ${installedTargetSummary}` : t('skills.installedTag')"
            @click="exploreStore.openInstallDialog(skill)"
          >
            <Icon icon="solar:check-circle-bold" />
            <span>{{ t('skills.installedTag') }}<template v-if="installedTargets.labels.length === 1"> · {{ installedTargets.labels[0] }}</template></span>
          </button>

          <button
            v-else
            type="button"
            class="action-btn action-btn--primary"
            @click="exploreStore.openInstallDialog(skill)"
          >
            <Icon icon="solar:cloud-download-linear" />
            <span>{{ t('skills.installActionBtn') }}</span>
          </button>
        </div>
      </template>
    </NDrawerContent>
  </NDrawer>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { NDrawer, NDrawerContent, useMessage } from 'naive-ui'
import { Icon } from '@iconify/vue'
import { useI18n } from 'vue-i18n'
import { useSkillExploreStore } from '@/stores/skill-explore'
import { useSkillsStore } from '@/stores/skills'
import { openExternalUrl } from '@/services/app-service'
import { loadSkillSourceGuides, type SkillSourceGuide } from '@/services/skills/skill-source-guide'
import { renderSkillMarkdownPreview } from '@/services/skills/skill-markdown-preview'
import SkillStageBadge from '../common/SkillStageBadge.vue'
import SkillScoreBadge from '../common/SkillScoreBadge.vue'

const { locale, t } = useI18n()
const message = useMessage()
const exploreStore = useSkillExploreStore()
const skillsStore = useSkillsStore()

const skill = computed(() => exploreStore.activeDetailSkill)
const pipeline = computed(() => exploreStore.activeDetailPipeline)
const sourceDirectoryUrl = computed(() => {
  if (!skill.value?.subPath || !/^https:\/\/github\.com\/[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+\/?$/.test(skill.value.gitUrl)) return ''
  const path = skill.value.subPath.split('/').filter(Boolean)
  if (path.some(part => part === '.' || part === '..')) return ''
  return `${skill.value.gitUrl.replace(/\/$/, '')}/tree/HEAD/${path.map(encodeURIComponent).join('/')}`
})
const isCommunityIndex = computed(() => skill.value?.sourceId === 'agent-skills-corpus')
const isUsageUnknown = computed(() => {
  const guidance = skill.value?.howToUse || ''
  return isCommunityIndex.value || !guidance || guidance.startsWith('在对应 Agent 会话或工程中启用此技能') ||
    guidance.startsWith('本技能为多技能集合包') || guidance.startsWith('未收录触发条件')
})
const copied = ref(false)
const newTagInput = ref('')
const sourceGuides = ref<SkillSourceGuide[]>([])
const selectedGuideKind = ref<SkillSourceGuide['kind']>('SKILL.md')
const sourceGuide = computed(() => sourceGuides.value.find(guide => guide.kind === selectedGuideKind.value) || sourceGuides.value[0] || null)
const sourceGuideLoading = ref(false)
const sourceGuideError = ref('')
const renderedGuide = computed(() => sourceGuide.value
  ? renderSkillMarkdownPreview(sourceGuide.value)
  : { html: '', truncated: false })
let guideRequest = 0

async function loadGuides(current: typeof skill.value) {
  const request = ++guideRequest
  sourceGuides.value = []
  selectedGuideKind.value = 'SKILL.md'
  sourceGuideError.value = ''
  sourceGuideLoading.value = Boolean(current)
  if (!current) return
  if (current.usageGuide) {
    sourceGuideLoading.value = false
    return
  }
  try {
    const guides = await loadSkillSourceGuides(current)
    if (request === guideRequest) sourceGuides.value = guides
  } catch (error) {
    if (request === guideRequest) sourceGuideError.value = t('skills.review.detail.guideError', { error: error instanceof Error ? error.message : String(error) })
  } finally {
    if (request === guideRequest) sourceGuideLoading.value = false
  }
}

watch(skill, current => { void loadGuides(current) })

const installedTargets = computed(() => {
  if (!skill.value) {
    return { isInstalled: false, count: 0, agentTypes: [], labels: [], skills: [] }
  }
  return exploreStore.getSkillInstalledTargets(skill.value, skillsStore.skills)
})

const installedTargetSummary = computed(() => installedTargets.value.labels.join(' / '))

const isInstalled = computed(() => {
  if (!skill.value) return false
  return exploreStore.isSkillInstalled(skill.value, skillsStore.skills)
})

const currentSkillCustomTags = computed(() => {
  if (!skill.value) return []
  return exploreStore.getSkillCustomCategories(skill.value)
})

const companionSkills = computed(() => {
  if (!skill.value?.recommendedWith) return []
  return exploreStore.catalogSkills.filter(s => skill.value?.recommendedWith.includes(s.id))
})

function formatNumber(num: number): string {
  if (num >= 1000) {
    return (num / 1000).toFixed(1) + 'k'
  }
  return String(num)
}

function handleAddTag() {
  const val = newTagInput.value.trim()
  if (!val || !skill.value) return
  exploreStore.addSkillCustomCategory(skill.value.id, val)
  newTagInput.value = ''
  message.success(t('skills.review.detail.categoryAdded', { tag: val }))
}

async function handleCopy(text: string) {
  try {
    await navigator.clipboard.writeText(text)
    copied.value = true
    message.success(t('skills.review.detail.copyDone'))
    setTimeout(() => {
      copied.value = false
    }, 2000)
  } catch {
    message.error(t('skills.review.detail.copyFailed'))
  }
}

async function openSourceUrl(url: string) {
  try {
    const parsed = new URL(url)
    if (parsed.protocol !== 'https:' || parsed.hostname !== 'github.com') throw new Error('来源地址无效')
    await openExternalUrl(parsed.href)
  } catch {
    message.error(t('skills.review.detail.sourceOpenFailed'))
  }
}

async function handleGuideLinkClick(event: MouseEvent) {
  const target = event.target
  if (!(target instanceof Element)) return
  const link = target.closest('a[href]')
  if (!link) return
  event.preventDefault()
  const href = link.getAttribute('href')
  if (!href) return
  try {
    const parsed = new URL(href)
    if (parsed.protocol !== 'https:') throw new Error('文档链接无效')
    await openExternalUrl(parsed.href)
  } catch {
    message.error(t('skills.review.detail.linkOpenFailed'))
  }
}
</script>

<style scoped lang="scss">
.drawer-header-content {
  align-items: center;
  display: flex;
  gap: 10px;
}

.source-guide-preview {
  color: var(--lumina-text);
  font-size: 12px;
  line-height: 1.6;
  margin: 8px 0;
  max-height: 440px;
  overflow: auto;
  overflow-wrap: anywhere;
}
.source-guide-preview :deep(:first-child) { margin-top: 0; }
.source-guide-preview :deep(:last-child) { margin-bottom: 0; }
.source-guide-preview :deep(h1),
.source-guide-preview :deep(h2),
.source-guide-preview :deep(h3),
.source-guide-preview :deep(h4) { color: var(--lumina-text); line-height: 1.35; margin: 16px 0 7px; }
.source-guide-preview :deep(h1) { font-size: 17px; }
.source-guide-preview :deep(h2) { font-size: 15px; }
.source-guide-preview :deep(h3),
.source-guide-preview :deep(h4) { font-size: 13px; }
.source-guide-preview :deep(p),
.source-guide-preview :deep(ul),
.source-guide-preview :deep(ol),
.source-guide-preview :deep(blockquote) { margin: 7px 0; }
.source-guide-preview :deep(ul),
.source-guide-preview :deep(ol) { padding-left: 20px; }
.source-guide-preview :deep(pre) {
  background: var(--lumina-surface-elevated);
  border: 1px solid var(--lumina-separator);
  border-radius: 6px;
  overflow-x: auto;
  padding: 9px 10px;
}
.source-guide-preview :deep(code) { font-family: var(--lumina-font-mono, monospace); font-size: 11px; }
.source-guide-preview :deep(:not(pre) > code) { background: var(--lumina-control-hover); border-radius: 3px; padding: 1px 4px; }
.source-guide-preview :deep(a) { color: var(--lumina-primary); cursor: pointer; text-decoration: underline; }
.source-guide-preview :deep(img) { height: auto; max-width: 100%; }
.source-guide-preview :deep(table) { border-collapse: collapse; display: block; max-width: 100%; overflow-x: auto; }
.source-guide-preview :deep(th),
.source-guide-preview :deep(td) { border: 1px solid var(--lumina-separator); padding: 4px 7px; }
.source-guide-preview :deep(blockquote) { border-left: 2px solid var(--lumina-separator); color: var(--lumina-text-secondary); padding-left: 10px; }
.source-guide-switch {
  display: flex;
  gap: 4px;
  margin-bottom: 8px;
}
.source-guide-switch button {
  background: transparent;
  border: 1px solid var(--lumina-separator);
  border-radius: 5px;
  color: var(--lumina-text-secondary);
  cursor: pointer;
  font-size: 11px;
  padding: 3px 8px;
}
.source-guide-switch button.active {
  background: var(--lumina-primary-soft);
  border-color: var(--lumina-primary);
  color: var(--lumina-primary);
}
.source-address {
  background: transparent;
  border: 0;
  color: var(--lumina-primary);
  cursor: pointer;
  font-size: 12px;
  padding: 0;
  text-align: left;
}
.source-address { overflow-wrap: anywhere; text-decoration: underline; }
.source-retry {
  background: transparent;
  border: 0;
  color: var(--lumina-primary);
  cursor: pointer;
  font-size: 12px;
  margin-top: 6px;
  padding: 0;
}

.header-icon-pill {
  align-items: center;
  background: var(--lumina-primary-soft);
  border-radius: var(--lumina-radius-md, 8px);
  color: var(--lumina-primary);
  display: flex;
  font-size: 20px;
  height: 36px;
  justify-content: center;
  width: 36px;
}

.header-title-box {
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.drawer-skill-title {
  color: var(--lumina-text);
  font-size: 14.5px;
  font-weight: 700;
  margin: 0;
}

.drawer-subtitle {
  color: var(--lumina-text-secondary);
  font-size: 11px;
}

.drawer-body-wrap {
  display: flex;
  flex-direction: column;
  gap: 18px;
  padding: 10px 0 24px;
}

.drawer-section {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.scenario-banner {
  align-items: flex-start;
  background: var(--lumina-surface-secondary);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-md, 8px);
  display: flex;
  gap: 10px;
  padding: 10px 12px;

  .target-icon {
    color: var(--lumina-primary);
    font-size: 18px;
    margin-top: 1px;
  }

  .scenario-content {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .scenario-title {
    color: var(--lumina-text);
    font-size: 11.5px;
    font-weight: 600;
  }

  .scenario-text {
    color: var(--lumina-text-secondary);
    font-size: 11.5px;
    line-height: 1.4;
    margin: 0;
  }
}

.pipeline-desc-text {
  color: var(--lumina-text-secondary);
  font-size: 12px;
  line-height: 1.5;
  margin: 0;
}

.playbook-steps-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.playbook-step-card {
  background: var(--lumina-surface-secondary);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-md, 8px);
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px;
}

.step-card-header {
  align-items: center;
  display: flex;
  gap: 8px;
}

.step-badge-num {
  align-items: center;
  background: var(--lumina-primary);
  border-radius: 50%;
  color: #fff;
  display: flex;
  font-size: 10px;
  font-weight: 700;
  height: 18px;
  justify-content: center;
  width: 18px;
}

.step-role-title {
  color: var(--lumina-text);
  font-size: 12px;
  font-weight: 600;
}

.step-how-to-act {
  color: var(--lumina-text-secondary);
  font-size: 11.5px;
  line-height: 1.45;
  margin: 0;
}

.step-prompt-box {
  background: var(--lumina-surface-elevated);
  border: 0.5px solid var(--lumina-separator);
  border-radius: 6px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-top: 6px;
  padding: 8px 10px;
}

.prompt-header {
  align-items: center;
  display: flex;
  justify-content: space-between;
}

.prompt-label {
  color: var(--lumina-text-tertiary);
  font-size: 10.5px;
  font-weight: 500;
}

.copy-mini-btn {
  align-items: center;
  background: transparent;
  border: 0.5px solid var(--lumina-separator);
  border-radius: 3px;
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: inline-flex;
  font-size: 10.5px;
  gap: 3px;
  padding: 1px 5px;
  transition: all 120ms ease;

  &:hover {
    background: var(--lumina-control-hover);
    color: var(--lumina-text);
  }
}

.prompt-snippet {
  color: var(--lumina-text);
  font-family: var(--lumina-font-mono, monospace);
  font-size: 11px;
  line-height: 1.4;
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
}

.skill-full-desc {
  color: var(--lumina-text-secondary);
  font-size: 12px;
  line-height: 1.55;
  margin: 0;
}

.metrics-dashboard-card {
  background: var(--lumina-surface-secondary);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-md, 8px);
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 12px 14px;
}

.dashboard-header {
  align-items: center;
  display: flex;
  justify-content: space-between;
}

.dashboard-title {
  color: var(--lumina-text);
  font-size: 12px;
  font-weight: 600;
}

.metrics-grid {
  display: grid;
  gap: 8px;
  grid-template-columns: repeat(4, 1fr);
}

.metric-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.metric-label {
  color: var(--lumina-text-tertiary);
  font-size: 10px;
}

.metric-val {
  color: var(--lumina-text);
  font-family: var(--lumina-font-mono, monospace);
  font-size: 12px;
  font-weight: 600;
}

.highlight-green {
  color: var(--lumina-success, #27853f);
}

.security-status-row {
  align-items: center;
  border-top: 0.5px dashed var(--lumina-separator);
  color: var(--lumina-success, #27853f);
  display: flex;
  font-size: 11px;
  gap: 6px;
  padding-top: 4px;

  .security-glyph {
    font-size: 14px;
    flex-shrink: 0;
  }
}

.section-title-row {
  align-items: center;
  display: flex;
  gap: 6px;

  .sec-icon {
    color: var(--lumina-primary);
    font-size: 14px;
  }

  .sec-heading {
    color: var(--lumina-text);
    flex: 1;
    font-size: 12px;
    font-weight: 600;
    margin: 0;
  }
}

.custom-tags-editor {
  background: var(--lumina-surface-secondary);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-md, 8px);
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 10px 12px;
}

.existing-tags-row {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 6px;
}

.user-tag-pill {
  align-items: center;
  background: var(--lumina-surface-elevated);
  border: 0.5px solid var(--lumina-separator);
  border-radius: 4px;
  color: var(--lumina-primary);
  display: inline-flex;
  font-size: 11px;
  font-weight: 500;
  gap: 4px;
  padding: 1.5px 6px;

  .remove-tag-btn {
    background: none;
    border: none;
    color: var(--lumina-text-tertiary);
    cursor: pointer;
    font-size: 12px;
    padding: 0;

    &:hover {
      color: var(--lumina-danger, #d9363e);
    }
  }
}

.no-tags-hint {
  color: var(--lumina-text-tertiary);
  font-size: 11px;
}

.add-tag-input-row {
  display: flex;
  gap: 8px;
}

.tag-input-field {
  background: var(--lumina-surface-elevated);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-sm, 6px);
  color: var(--lumina-text);
  flex: 1;
  font-size: 11.5px;
  outline: none;
  padding: 0 8px;
  height: 28px;

  &::placeholder {
    color: var(--lumina-text-tertiary);
  }
}

.add-tag-btn {
  background: var(--lumina-control-bg);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-sm, 6px);
  color: var(--lumina-text);
  cursor: pointer;
  font-size: 11px;
  font-weight: 500;
  padding: 0 10px;
  white-space: nowrap;

  &:hover:not(:disabled) {
    background: var(--lumina-control-hover);
  }

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
}

.instruction-box {
  background: var(--lumina-surface-secondary);
  border-left: 3px solid var(--lumina-primary);
  border-radius: 0 var(--lumina-radius-sm, 6px) var(--lumina-radius-sm, 6px) 0;
  padding: 8px 12px;
}

.usage-summary { display: flex; flex-direction: column; gap: 9px; }
.usage-provenance { color: var(--lumina-text-tertiary); font-size: 11px; margin: 0; }
.usage-overview { color: var(--lumina-text); font-size: 12px; line-height: 1.55; margin: 0; }
.usage-field strong { color: var(--lumina-text); font-size: 11px; }
.usage-field p { color: var(--lumina-text-secondary); font-size: 12px; line-height: 1.55; margin: 3px 0 0; }
.usage-field ol { color: var(--lumina-text-secondary); font-size: 12px; line-height: 1.55; margin: 4px 0 0; padding-left: 18px; }
.usage-field li + li { margin-top: 4px; }

.instruction-text {
  color: var(--lumina-text);
  font-size: 12px;
  line-height: 1.5;
  margin: 0;
}

.copy-pill-btn {
  align-items: center;
  background: transparent;
  border: 0.5px solid var(--lumina-separator);
  border-radius: 4px;
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: inline-flex;
  font-size: 11px;
  gap: 4px;
  padding: 2px 7px;
  transition: all 120ms ease;

  &:hover {
    background: var(--lumina-control-hover);
  }
}

.prompt-code-block {
  background: var(--lumina-surface-secondary);
  border-radius: var(--lumina-radius-sm, 6px);
  color: var(--lumina-text);
  font-family: var(--lumina-font-mono, monospace);
  font-size: 11px;
  line-height: 1.45;
  margin: 0;
  padding: 10px 12px;
  white-space: pre-wrap;
  word-break: break-all;
}

.companion-pills {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.companion-chip {
  align-items: center;
  background: var(--lumina-surface-secondary);
  border: 0.5px solid var(--lumina-separator);
  border-radius: 6px;
  cursor: pointer;
  display: flex;
  gap: 8px;
  padding: 6px 10px;
  text-align: left;
  transition: all 120ms ease;

  &:hover {
    border-color: var(--lumina-primary);
  }

  .comp-name {
    color: var(--lumina-text);
    flex: 1;
    font-size: 12px;
    font-weight: 500;
  }

  .comp-score {
    color: var(--lumina-warning, #b76500);
    font-size: 11px;
    font-weight: 600;
  }
}

.drawer-footer-actions {
  align-items: center;
  display: flex;
  justify-content: space-between;
  width: 100%;
}

.pipeline-status-text {
  color: var(--lumina-text-secondary);
  font-size: 11.5px;
}

.star-toggle-btn {
  align-items: center;
  background: transparent;
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-sm, 6px);
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: inline-flex;
  font-size: 12px;
  gap: 6px;
  padding: 6px 12px;
  transition: all 120ms ease;

  &.active {
    background: var(--lumina-primary-soft);
    border-color: var(--lumina-primary);
    color: var(--lumina-primary);
  }
}

.action-btn {
  align-items: center;
  border-radius: var(--lumina-radius-sm, 6px);
  cursor: pointer;
  display: inline-flex;
  font-size: 12px;
  font-weight: 500;
  gap: 6px;
  padding: 6px 16px;
  transition: all 120ms ease;

  &--primary {
    background: var(--lumina-primary);
    border: 0.5px solid transparent;
    color: #ffffff;
    font-weight: 600;

    &:hover {
      filter: brightness(1.08);
    }
  }

  &--installed {
    background: color-mix(in srgb, var(--lumina-success, #27853f) 12%, transparent);
    border: 0.5px solid color-mix(in srgb, var(--lumina-success, #27853f) 25%, transparent);
    color: var(--lumina-success, #27853f);
    cursor: default;
  }
}
</style>
