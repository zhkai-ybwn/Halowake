<template>
  <article class="skill-row" @click="exploreStore.openDetail(skill)">
    <!-- 1. 职能/阶段个性化图标 -->
    <span
      class="skill-row-icon"
      :style="{
        color: visual.color,
        backgroundColor: visual.bg,
        borderColor: visual.border,
      }"
      aria-hidden="true"
    >
      <Icon :icon="visual.icon" />
    </span>

    <!-- 2. 主体信息 -->
    <div class="skill-row-main">
      <div class="skill-row-title">
        <strong :title="skill.name">{{ skill.name }}</strong>
        <span v-if="skill.titleZh && skill.titleZh !== skill.name" class="skill-row-zh-title">{{ skill.titleZh }}</span>
        <span v-if="skill.isBundle" class="skill-row-badge skill-row-badge--bundle">
          {{ t('skills.review.bundleCount', { count: skill.bundleCount || skill.subSkills?.length || 0 }) }}
        </span>
        <span v-else-if="skill.badge === 'verified'" class="skill-row-badge skill-row-badge--verified">
          {{ t('skills.review.featured') }}
        </span>
        <span v-else-if="skill.sourceId === 'agent-skills-corpus'" class="skill-row-badge skill-row-badge--community">
          {{ t('skills.review.communityIndex') }}
        </span>
      </div>

      <p class="skill-row-description" :title="skill.description">
        {{ skill.description }}
      </p>

      <div class="skill-row-meta">
        <span
          v-if="matchReason"
          class="skill-match-reason"
          :class="`skill-match-reason--${matchReason.tier}`"
          :title="matchReason.summary"
        >
          <Icon :icon="matchReason.tier === 'exact' ? 'solar:verified-check-bold' : 'solar:compass-linear'" />
          <span>{{ matchReason.summary }}</span>
        </span>
        <span class="skill-author">{{ skill.author.name }}</span>
        <span class="meta-dot" aria-hidden="true">·</span>
        <span class="skill-category">{{ skillCategoryLabel(skill.category) }}</span>
        <template v-if="skill.subPath">
          <span class="meta-dot" aria-hidden="true">·</span>
          <span class="skill-row-path" :title="skill.subPath">{{ skill.subPath }}</span>
        </template>
      </div>
    </div>

    <!-- 3. 流行度热度指示器 (避免与收藏星星混淆) -->
    <span
      v-if="stars > 0"
      class="skill-row-popularity"
      :title="t('skills.review.stars', { count: stars }) + (skill.sourceId === 'agent-skills-corpus' ? t('skills.review.indexSnapshot') : '')"
    >
      <Icon icon="solar:fire-bold" class="popularity-fire" />
      <span>{{ formatStars(stars) }}</span>
    </span>

    <!-- 4. 动作区 -->
    <div class="skill-row-actions" @click.stop>
      <button
        type="button"
        class="skill-row-star"
        :class="{ active: skill.isStar }"
        :aria-label="skill.isStar ? t('skills.review.unstar') : t('skills.review.star')"
        :title="skill.isStar ? t('skills.review.unstar') : t('skills.review.star')"
        :aria-pressed="Boolean(skill.isStar)"
        @click.stop="exploreStore.toggleStar(skill.id)"
      >
        <Icon :icon="skill.isStar ? 'solar:star-bold' : 'solar:star-linear'" />
      </button>

      <button
        v-if="isInstalled"
        type="button"
        class="skill-row-installed-pill"
        :title="installedTargetSummary ? `${t('skills.installedTag')}: ${installedTargetSummary}` : t('skills.installedTag')"
        @click.stop="exploreStore.openInstallDialog(skill)"
      >
        <Icon icon="solar:check-circle-bold" />
        <span>{{ t('skills.installedTag') }}<template v-if="installedTargets.labels.length === 1"> · {{ installedTargets.labels[0] }}</template></span>
      </button>
      <button
        v-else
        type="button"
        class="skill-row-install"
        @click.stop="exploreStore.openInstallDialog(skill)"
      >
        <Icon :icon="skill.isBundle ? 'solar:box-minimalistic-linear' : 'solar:cloud-download-linear'" />
        <span>{{ skill.isBundle ? t('skills.review.assembleBundle') : t('skills.installActionBtn') }}</span>
      </button>
    </div>

    <!-- 5. 详情展开暗示 -->
    <div class="skill-row-chevron" aria-hidden="true">
      <Icon icon="solar:alt-arrow-right-linear" />
    </div>
  </article>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { Icon } from '@iconify/vue'
import { useI18n } from 'vue-i18n'
import { useSkillExploreStore } from '@/stores/skill-explore'
import { useSkillsStore } from '@/stores/skills'
import type { CatalogSkill } from '@/types/skills'
import { skillCategoryLabel } from '@/utils/skill-category-label'
import { getSkillVisuals } from '@/utils/skill-visuals'

const props = defineProps<{ skill: CatalogSkill }>()
const { t } = useI18n()
const exploreStore = useSkillExploreStore()
const skillsStore = useSkillsStore()

const visual = computed(() =>
  getSkillVisuals({
    name: props.skill.name,
    description: props.skill.description,
    stage: props.skill.stage,
    category: props.skill.category,
  })
)

const installedTargets = computed(() =>
  exploreStore.getSkillInstalledTargets(props.skill, skillsStore.skills)
)
const installedTargetSummary = computed(() => installedTargets.value.labels.join(' / '))
const isInstalled = computed(() => exploreStore.isSkillInstalled(props.skill, skillsStore.skills))
const matchReason = computed(() => exploreStore.getSkillMatchReason(props.skill.id))
const stars = computed(() => props.skill.metrics?.githubStars ?? props.skill.stars ?? 0)
const formatStars = (count: number) => (count >= 1000 ? `${(count / 1000).toFixed(1)}k` : String(count))
</script>

<style scoped lang="scss">
.skill-row {
  align-items: center;
  border-bottom: 0.5px solid var(--lumina-separator);
  cursor: pointer;
  display: flex;
  gap: 14px;
  min-height: 68px;
  padding: 10px 18px;
  position: relative;
  transition: all 140ms ease;

  &:hover {
    background: color-mix(in srgb, var(--lumina-primary-soft) 25%, var(--lumina-surface-elevated));
  }

  &:last-child {
    border-bottom: 0;
  }
}

.skill-row-icon {
  align-items: center;
  border: 1px solid transparent;
  border-radius: 9px;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.04);
  display: flex;
  flex-shrink: 0;
  font-size: 17px;
  height: 36px;
  justify-content: center;
  transition: transform 140ms ease;
  width: 36px;
}

.skill-row:hover .skill-row-icon {
  transform: scale(1.05);
}

.skill-row-main {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 3px;
  min-width: 0;
  text-align: left;
}

.skill-row-title {
  align-items: center;
  display: flex;
  flex-wrap: wrap;
  gap: 7px;
  min-width: 0;

  strong {
    color: var(--lumina-text);
    font-size: 13px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .skill-row-zh-title {
    color: var(--lumina-text-secondary);
    font-size: 11.5px;
    font-weight: 500;
  }
}

.skill-row-badge {
  border-radius: 4px;
  flex-shrink: 0;
  font-size: 10px;
  font-weight: 500;
  padding: 1.5px 5px;

  &--bundle {
    background: color-mix(in srgb, var(--lumina-primary) 14%, transparent);
    color: var(--lumina-primary);
  }

  &--verified {
    background: color-mix(in srgb, var(--lumina-success, #27853f) 14%, transparent);
    color: var(--lumina-success, #27853f);
  }

  &--community {
    background: var(--lumina-fill-quaternary);
    color: var(--lumina-text-secondary);
  }
}

.skill-row-description {
  color: var(--lumina-text-secondary);
  display: -webkit-box;
  font-size: 11.5px;
  line-height: 1.45;
  margin: 1px 0 3px;
  overflow: hidden;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 1;
}

.skill-row-meta {
  align-items: center;
  color: var(--lumina-text-tertiary);
  display: flex;
  font-size: 10.5px;
  gap: 5px;

  .meta-dot {
    opacity: 0.6;
  }

  .skill-author {
    color: var(--lumina-text-secondary);
    font-weight: 500;
  }

  .skill-row-path {
    font-family: var(--lumina-font-mono, monospace);
    font-size: 10px;
    max-width: 220px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
}

.skill-match-reason {
  align-items: center;
  border-radius: 4px;
  display: inline-flex;
  font-size: 10px;
  font-weight: 600;
  gap: 3px;
  margin-right: 4px;
  max-width: 340px;
  overflow: hidden;
  padding: 1px 6px;
  text-overflow: ellipsis;
  white-space: nowrap;

  &--exact {
    background: color-mix(in srgb, var(--lumina-success, #10b981) 12%, transparent);
    border: 0.5px solid color-mix(in srgb, var(--lumina-success, #10b981) 28%, transparent);
    color: var(--lumina-success, #10b981);
  }

  &--general {
    background: color-mix(in srgb, #f59e0b 12%, transparent);
    border: 0.5px solid color-mix(in srgb, #f59e0b 28%, transparent);
    color: #d97706;
  }
}

.skill-row-popularity {
  align-items: center;
  background: color-mix(in srgb, #f59e0b 10%, transparent);
  border-radius: 9999px;
  color: #d97706;
  display: inline-flex;
  flex-shrink: 0;
  font-size: 11px;
  font-weight: 600;
  gap: 3px;
  padding: 2px 7px;

  .popularity-fire {
    font-size: 12px;
  }
}

.skill-row-actions {
  align-items: center;
  display: flex;
  flex-shrink: 0;
  gap: 7px;
}

.skill-row-star {
  align-items: center;
  background: transparent;
  border: 0.5px solid transparent;
  border-radius: 6px;
  color: var(--lumina-text-tertiary);
  cursor: pointer;
  display: inline-flex;
  font-size: 16px;
  height: 28px;
  justify-content: center;
  transition: all 120ms ease;
  width: 28px;

  &:hover {
    background: var(--lumina-control-hover);
    color: var(--lumina-text);
  }

  &.active {
    color: #eab308;

    &:hover {
      color: #ca8a04;
    }
  }
}

.skill-row-install {
  align-items: center;
  background: var(--lumina-primary-soft);
  border: 0.5px solid color-mix(in srgb, var(--lumina-primary) 30%, transparent);
  border-radius: 9999px;
  color: var(--lumina-primary);
  cursor: pointer;
  display: inline-flex;
  font-size: 11.5px;
  font-weight: 600;
  gap: 4px;
  height: 26px;
  padding: 0 13px;
  transition: all 140ms ease;

  svg {
    font-size: 13px;
  }

  &:hover {
    background: var(--lumina-primary);
    border-color: var(--lumina-primary);
    box-shadow: 0 2px 6px color-mix(in srgb, var(--lumina-primary) 30%, transparent);
    color: #ffffff;
  }
}

.skill-row-installed-pill {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-fill-quaternary) 80%, transparent);
  border: 0.5px solid var(--lumina-separator);
  border-radius: 9999px;
  color: var(--lumina-text-secondary);
  display: inline-flex;
  font-size: 11px;
  font-weight: 500;
  gap: 4px;
  height: 26px;
  padding: 0 11px;

  svg {
    color: var(--lumina-success, #10b981);
    font-size: 13px;
  }
}

.skill-row-chevron {
  align-items: center;
  color: var(--lumina-text-tertiary);
  display: flex;
  flex-shrink: 0;
  font-size: 15px;
  justify-content: center;
  opacity: 0.22;
  transform: translateX(-3px);
  transition: all 140ms ease;
}

.skill-row:hover .skill-row-chevron {
  color: var(--lumina-primary);
  opacity: 0.85;
  transform: translateX(0);
}

@media (max-width: 650px) {
  .skill-row {
    gap: 9px;
  }

  .skill-row-icon,
  .skill-row-popularity,
  .skill-row-path {
    display: none;
  }
}
</style>
