<template>
  <div class="pipeline-feature-card" :class="{ 'is-complete': installStatus.isFullyInstalled }">
    <!-- Header -->
    <div class="pipeline-card-header">
      <div class="pipeline-icon-box">
        <Icon :icon="pipeline.icon" class="pipeline-icon" />
      </div>
      <div class="pipeline-title-group">
        <div class="title-with-badge">
          <h4 class="pipeline-main-title">{{ pipeline.title }}</h4>
          <span class="pipeline-badge-pill">{{ pipeline.badge }}</span>
        </div>
        <p class="pipeline-subtitle">{{ pipeline.subtitle }}</p>
      </div>
    </div>

    <!-- Target Scenario Description -->
    <p class="pipeline-scenario-desc">
      <Icon icon="solar:target-linear" class="scenario-icon" />
      <span>{{ pipeline.targetScenario }}</span>
    </p>

    <!-- Pipeline Stages Sequence (Breadcrumb style) -->
    <div class="pipeline-steps-flow">
      <template v-for="(skillId, index) in pipeline.skills" :key="skillId">
        <button
          type="button"
          class="step-skill-chip"
          :class="{
            'step-installed': isSkillInstalled(skillId),
          }"
          :title="getSkillTitle(skillId)"
          @click="openSkillDetail(skillId)"
        >
          <Icon
            :icon="isSkillInstalled(skillId) ? 'solar:check-circle-bold' : 'solar:widget-linear'"
            class="chip-glyph"
          />
          <span class="chip-name">{{ getSkillShortName(skillId) }}</span>
        </button>

        <Icon
          v-if="index < pipeline.skills.length - 1"
          icon="solar:alt-arrow-right-linear"
          class="step-arrow"
        />
      </template>
    </div>

    <!-- Pipeline Footer -->
    <div class="pipeline-card-footer">
      <div class="pipeline-progress-section">
        <div class="progress-info-row">
          <span class="progress-label">装配状态</span>
          <span class="progress-count" :class="{ 'is-complete': installStatus.isFullyInstalled }">
            {{ installStatus.installed }} / {{ installStatus.total }} 技能
          </span>
        </div>
        <div class="progress-track">
          <div
            class="progress-fill"
            :style="{ width: `${(installStatus.installed / installStatus.total) * 100}%` }"
          ></div>
        </div>
      </div>

      <div class="footer-actions-group">
        <!-- 查看全流程协同玩法指引按钮 -->
        <button
          type="button"
          class="pipeline-action-btn pipeline-action-btn--ghost"
          title="查看该流水线端到端协同实战指引与推荐 Prompt"
          @click="exploreStore.openPipelinePlaybook(pipeline)"
        >
          <Icon icon="solar:route-linear" />
          <span>玩法指引</span>
        </button>

        <button
          v-if="installStatus.isFullyInstalled"
          type="button"
          class="pipeline-action-btn pipeline-action-btn--complete"
          disabled
        >
          <Icon icon="solar:check-read-linear" />
          <span>{{ t('skills.pipelineFullyInstalled') }}</span>
        </button>

        <button
          v-else
          type="button"
          class="pipeline-action-btn pipeline-action-btn--primary"
          @click="exploreStore.openPipelineInstallDialog(pipeline)"
        >
          <Icon icon="solar:rocket-2-linear" />
          <span>{{ t('skills.pipelineInstallAll') }} ({{ pipeline.skills.length }})</span>
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { Icon } from '@iconify/vue'
import { useI18n } from 'vue-i18n'
import { useSkillExploreStore } from '@/stores/skill-explore'
import { useSkillsStore } from '@/stores/skills'
import type { SkillPipeline } from '@/types/skills'

const props = defineProps<{
  pipeline: SkillPipeline
}>()

const { t } = useI18n()
const exploreStore = useSkillExploreStore()
const skillsStore = useSkillsStore()

const installStatus = computed(() => {
  return exploreStore.getPipelineInstallStatus(props.pipeline, skillsStore.skills)
})

function isSkillInstalled(skillId: string): boolean {
  return exploreStore.isSkillInstalled(skillId, skillsStore.skills)
}

function getSkillShortName(skillId: string): string {
  const s = exploreStore.catalogSkills.find(sk => sk.id === skillId)
  return s ? s.name : skillId
}

function getSkillTitle(skillId: string): string {
  const s = exploreStore.catalogSkills.find(sk => sk.id === skillId)
  return s ? `${s.name} - ${s.titleZh}` : skillId
}

function openSkillDetail(skillId: string) {
  const s = exploreStore.catalogSkills.find(sk => sk.id === skillId)
  if (s) {
    exploreStore.openDetail(s)
  }
}
</script>

<style scoped lang="scss">
.pipeline-feature-card {
  background: var(--lumina-card-bg, #ffffff);
  border: 0.5px solid var(--lumina-card-border);
  border-radius: var(--lumina-radius-lg, 12px);
  display: flex;
  flex-direction: column;
  gap: 12px;
  overflow: hidden;
  padding: 16px;
  position: relative;
  transition: all 140ms ease;

  &:hover {
    border-color: var(--lumina-primary);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.08);
  }
}

.pipeline-card-header {
  align-items: flex-start;
  display: flex;
  gap: 12px;
}

.pipeline-icon-box {
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

.pipeline-title-group {
  display: flex;
  flex-direction: column;
  flex: 1;
  gap: 2px;
  min-width: 0;
}

.title-with-badge {
  align-items: center;
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.pipeline-main-title {
  color: var(--lumina-text);
  font-size: 13.5px;
  font-weight: 700;
  margin: 0;
}

.pipeline-badge-pill {
  background: var(--lumina-primary-soft);
  border-radius: 9999px;
  color: var(--lumina-primary);
  font-size: 10.5px;
  font-weight: 600;
  padding: 1px 6px;
}

.pipeline-subtitle {
  color: var(--lumina-text-secondary);
  font-size: 11px;
  margin: 0;
}

.pipeline-scenario-desc {
  align-items: center;
  background: var(--lumina-surface-secondary);
  border-radius: var(--lumina-radius-sm, 6px);
  color: var(--lumina-text-secondary);
  display: flex;
  font-size: 11.5px;
  gap: 6px;
  margin: 0;
  padding: 6px 10px;

  .scenario-icon {
    color: var(--lumina-primary);
    font-size: 14px;
    flex-shrink: 0;
  }
}

.pipeline-steps-flow {
  align-items: center;
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  padding: 4px 0;
}

.step-skill-chip {
  align-items: center;
  background: var(--lumina-surface-secondary);
  border: 0.5px solid var(--lumina-separator);
  border-radius: 5px;
  color: var(--lumina-text);
  cursor: pointer;
  display: inline-flex;
  font-size: 11px;
  font-weight: 500;
  gap: 5px;
  padding: 3px 8px;
  transition: all 120ms ease;

  &:hover {
    border-color: var(--lumina-primary);
    color: var(--lumina-primary);
  }

  .chip-glyph {
    font-size: 12px;
  }

  &.step-installed {
    background: color-mix(in srgb, var(--lumina-success, #27853f) 12%, transparent);
    border-color: color-mix(in srgb, var(--lumina-success, #27853f) 28%, transparent);
    color: var(--lumina-success, #27853f);

    .chip-glyph {
      color: var(--lumina-success, #27853f);
    }
  }
}

.step-arrow {
  color: var(--lumina-text-tertiary);
  font-size: 11px;
}

.pipeline-card-footer {
  border-top: 0.5px solid var(--lumina-separator);
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin-top: auto;
  padding-top: 12px;
}

.pipeline-progress-section {
  display: flex;
  flex-direction: column;
  gap: 5px;
}

.progress-info-row {
  align-items: center;
  display: flex;
  justify-content: space-between;
}

.progress-label {
  color: var(--lumina-text-tertiary);
  font-size: 10.5px;
  font-weight: 500;
}

.progress-count {
  color: var(--lumina-text-secondary);
  font-family: var(--lumina-font-mono, monospace);
  font-size: 10.5px;
  font-weight: 600;

  &.is-complete {
    color: var(--lumina-success, #27853f);
  }
}

.progress-track {
  background: var(--lumina-surface-secondary);
  border-radius: 9999px;
  height: 4px;
  overflow: hidden;
  width: 100%;
}

.progress-fill {
  background: var(--lumina-primary);
  border-radius: 9999px;
  height: 100%;
  transition: width 0.3s ease;
}

.pipeline-feature-card.is-complete .progress-fill {
  background: var(--lumina-success, #27853f);
}

.footer-actions-group {
  align-items: center;
  display: flex;
  gap: 8px;
  width: 100%;
}

.pipeline-action-btn {
  align-items: center;
  border-radius: var(--lumina-radius-sm, 6px);
  cursor: pointer;
  display: inline-flex;
  font-size: 11.5px;
  font-weight: 600;
  gap: 6px;
  height: 30px;
  justify-content: center;
  padding: 0 12px;
  transition: all 120ms ease;
  white-space: nowrap;

  &--ghost {
    background: transparent;
    border: 0.5px solid var(--lumina-separator);
    color: var(--lumina-text-secondary);
    flex: 0 0 auto;

    &:hover {
      background: var(--lumina-control-hover);
      color: var(--lumina-text);
    }
  }

  &--primary {
    background: var(--lumina-primary);
    border: 0.5px solid transparent;
    color: #ffffff;
    flex: 1 1 auto;
    min-width: 0;

    &:hover {
      filter: brightness(1.08);
    }
  }

  &--complete {
    background: color-mix(in srgb, var(--lumina-success, #27853f) 12%, transparent);
    border: 0.5px solid color-mix(in srgb, var(--lumina-success, #27853f) 25%, transparent);
    color: var(--lumina-success, #27853f);
    cursor: default;
    flex: 1 1 auto;
  }
}
</style>
