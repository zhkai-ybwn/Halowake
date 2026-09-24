<template>
  <div
    class="skill-catalog-card"
    :class="{
      'is-installed': isInstalled,
      'is-bundle': skill.isBundle,
      'is-shelf-open': isSubSkillsOpen,
    }"
  >
    <!-- 1. Card Top Bar: Stage & Score & Star -->
    <div class="card-top-bar">
      <div class="top-left-badges">
        <SkillStageBadge :stage="skill.stage" />
        <SkillScoreBadge v-if="skill.halowakeScore" :score="skill.halowakeScore" :badge="skill.badge" />
      </div>

      <button
        type="button"
        class="card-star-btn"
        :class="{ active: skill.isStar }"
        :title="skill.isStar ? t('skills.detailStarredAction') : t('skills.detailStarAction')"
        @click.stop="exploreStore.toggleStar(skill.id)"
      >
        <Icon :icon="skill.isStar ? 'solar:star-bold' : 'solar:star-linear'" />
      </button>
    </div>

    <!-- 2. Header & Author Row -->
    <div class="card-identity-header" @click="exploreStore.openDetail(skill)">
      <div class="identity-primary-row">
        <span class="skill-code-name">{{ skill.name }}</span>
        <span v-if="isInstalled" class="installed-mini-pill">
          <Icon icon="solar:check-circle-bold" />
          <span>{{ t('skills.installedTag') }}</span>
        </span>
      </div>

      <div class="author-row">
        <span class="author-name">
          <Icon v-if="skill.author.verified" icon="solar:verified-check-bold" class="verified-icon" />
          <span>{{ skill.author.name }}</span>
        </span>
      </div>

      <h4 class="skill-chinese-title">{{ skill.titleZh }}</h4>
    </div>

    <!-- 3. Multi-Skill Bundle Banner & Interactive Sub-skills Shelf -->
    <div v-if="skill.isBundle && skill.subSkills?.length" class="bundle-shelf-wrap">
      <div
        class="bundle-header-toggle"
        role="button"
        tabindex="0"
        @click.stop="isSubSkillsOpen = !isSubSkillsOpen"
      >
        <div class="bundle-pill">
          <Icon icon="solar:box-minimalistic-bold" class="bundle-box-icon" />
          <span>多技能合集 · 含 {{ skill.subSkills.length }} 个独立技能</span>
        </div>
        <button type="button" class="toggle-action-btn">
          <span>{{ isSubSkillsOpen ? '收起清单' : '展开单独装配' }}</span>
          <Icon
            icon="solar:alt-arrow-down-linear"
            class="toggle-chevron"
            :class="{ rotated: isSubSkillsOpen }"
          />
        </button>
      </div>

      <!-- Slide-down sub-skills shelf -->
      <div v-if="isSubSkillsOpen" class="subskills-shelf">
        <div
          v-for="sub in skill.subSkills"
          :key="sub.id"
          class="subskill-row"
        >
          <div class="subskill-meta">
            <div class="subskill-name-row">
              <span class="subskill-name">{{ sub.name }}</span>
              <span class="subskill-title">{{ sub.titleZh }}</span>
            </div>
            <p class="subskill-desc">{{ sub.description }}</p>
          </div>
          <button
            type="button"
            class="subskill-install-btn"
            title="单独装配此子技能"
            @click.stop="exploreStore.openSubSkillInstallDialog(skill, sub)"
          >
            <Icon icon="solar:cloud-download-linear" />
            <span>单独装配</span>
          </button>
        </div>
      </div>
    </div>

    <!-- 4. Description -->
    <p class="card-desc" :title="skill.description" @click="exploreStore.openDetail(skill)">
      {{ skill.description }}
    </p>

    <!-- 5. Tags & Stats Row -->
    <div class="card-tags-row">
      <span v-for="tag in customTags" :key="tag" class="micro-custom-tag" :title="`自定义分类: ${tag}`">
        #{{ tag }}
      </span>
      <span v-for="tag in displayTags" :key="tag" class="micro-tag">
        #{{ tag }}
      </span>
      <span
        v-if="formattedStars"
        class="micro-stars-counter"
        :title="`GitHub Stars: ${skill.metrics?.githubStars || skill.stars || 0}`"
      >
        ⭐ {{ formattedStars }}
      </span>
    </div>

    <!-- 6. Card Action Footer -->
    <div class="card-footer">
      <div class="card-agent-dots" :title="`兼容 Agent: ${skill.compatibleAgents.join(', ')}`">
        <span
          v-for="agent in skill.compatibleAgents"
          :key="agent"
          class="agent-dot"
          :class="`agent-dot--${agent}`"
          :title="agent"
        ></span>
      </div>

      <div class="footer-buttons">
        <button
          type="button"
          class="card-action-btn card-action-btn--ghost"
          @click="exploreStore.openDetail(skill)"
        >
          <Icon icon="solar:info-circle-linear" />
          <span>玩法指引</span>
        </button>

        <button
          v-if="isInstalled"
          type="button"
          class="card-action-btn card-action-btn--installed"
          disabled
        >
          <Icon icon="solar:check-read-linear" />
          <span>{{ t('skills.installedTag') }}</span>
        </button>

        <button
          v-else-if="skill.isBundle"
          type="button"
          class="card-action-btn card-action-btn--primary"
          @click="exploreStore.openInstallDialog(skill)"
        >
          <Icon icon="solar:box-minimalistic-linear" />
          <span>合集装配 ({{ skill.subSkills?.length || skill.bundleCount }})</span>
        </button>

        <button
          v-else
          type="button"
          class="card-action-btn card-action-btn--primary"
          @click="exploreStore.openInstallDialog(skill)"
        >
          <Icon icon="solar:cloud-download-linear" />
          <span>{{ t('skills.installActionBtn') }}</span>
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { Icon } from '@iconify/vue'
import { useI18n } from 'vue-i18n'
import { useSkillExploreStore } from '@/stores/skill-explore'
import { useSkillsStore } from '@/stores/skills'
import type { CatalogSkill } from '@/types/skills'
import SkillStageBadge from '../common/SkillStageBadge.vue'
import SkillScoreBadge from '../common/SkillScoreBadge.vue'

const props = defineProps<{
  skill: CatalogSkill
}>()

const { t } = useI18n()
const exploreStore = useSkillExploreStore()
const skillsStore = useSkillsStore()

const isSubSkillsOpen = ref(false)

const isInstalled = computed(() => {
  return exploreStore.isSkillInstalled(props.skill.id, skillsStore.skills)
})

const customTags = computed(() => {
  return exploreStore.userCustomCategories[props.skill.id] || []
})

const displayTags = computed(() => {
  return props.skill.tags.slice(0, 2)
})

function formatStars(count: number): string {
  if (count >= 1000) {
    return (count / 1000).toFixed(1) + 'k'
  }
  return String(count)
}

const formattedStars = computed(() => {
  const count = props.skill.metrics?.githubStars ?? props.skill.stars ?? 0
  if (!count || count <= 0) return ''
  return formatStars(count)
})
</script>

<style scoped lang="scss">
.skill-catalog-card {
  background: var(--lumina-card-bg, #ffffff);
  border: 0.5px solid var(--lumina-card-border);
  border-radius: 10px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 14px;
  position: relative;
  transition: border-color 150ms ease, box-shadow 200ms ease, transform 220ms var(--lumina-ease-spring);
  user-select: none;

  &:hover {
    border-color: color-mix(in srgb, var(--lumina-primary) 35%, var(--lumina-separator));
    box-shadow: 0 4px 18px rgba(0, 0, 0, 0.05), 0 1px 3px rgba(0, 0, 0, 0.03);
    transform: translateY(-2px);
  }
}

.card-top-bar {
  align-items: center;
  display: flex;
  gap: 8px;
  justify-content: space-between;
}

.top-left-badges {
  align-items: center;
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.card-star-btn {
  align-items: center;
  background: none;
  border: none;
  border-radius: 4px;
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: flex;
  font-size: 16px;
  justify-content: center;
  padding: 2px;
  transition: all 120ms ease;

  &:hover {
    color: var(--lumina-warning, #b76500);
    transform: scale(1.15);
  }

  &.active {
    color: var(--lumina-warning, #b76500);
  }
}

.card-identity-header {
  cursor: pointer;
  display: flex;
  flex-direction: column;
}

.identity-primary-row {
  align-items: center;
  display: flex;
  gap: 6px;
  justify-content: space-between;
}

.skill-code-name {
  color: var(--lumina-text);
  font-family: var(--lumina-font-mono, monospace);
  font-size: 13.5px;
  font-weight: 700;
}

.installed-mini-pill {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-success, #27853f) 12%, transparent);
  border-radius: 4px;
  color: var(--lumina-success, #27853f);
  display: inline-flex;
  font-size: 11px;
  font-weight: 600;
  gap: 3px;
  padding: 1px 6px;
}

.author-row {
  align-items: center;
  display: flex;
  margin-top: 2px;
}

.author-name {
  align-items: center;
  color: var(--lumina-text-secondary);
  display: inline-flex;
  font-size: 11.5px;
  font-weight: 500;
  gap: 3.5px;
}

.verified-icon {
  color: var(--lumina-primary);
  font-size: 13px;
}

.skill-chinese-title {
  color: var(--lumina-text);
  font-size: 13.5px;
  font-weight: 600;
  margin: 4px 0 0;
}

/* Multi-skill Bundle Shelf */
.bundle-shelf-wrap {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin: 2px 0;
}

.bundle-header-toggle {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-primary) 6%, var(--lumina-surface-2));
  border: 0.5px solid color-mix(in srgb, var(--lumina-primary) 18%, transparent);
  border-radius: var(--lumina-radius-sm, 6px);
  cursor: pointer;
  display: flex;
  justify-content: space-between;
  padding: 5px 8px;
  transition: all 150ms ease;

  &:hover {
    background: color-mix(in srgb, var(--lumina-primary) 10%, var(--lumina-surface-2));
  }
}

.bundle-pill {
  align-items: center;
  color: var(--lumina-primary);
  display: inline-flex;
  font-size: 11.5px;
  font-weight: 600;
  gap: 5px;
}

.bundle-box-icon {
  font-size: 13px;
}

.toggle-action-btn {
  align-items: center;
  background: none;
  border: none;
  color: var(--lumina-primary);
  cursor: pointer;
  display: inline-flex;
  font-size: 11px;
  font-weight: 600;
  gap: 3px;
  padding: 0;
}

.toggle-chevron {
  font-size: 12px;
  transition: transform 180ms ease;

  &.rotated {
    transform: rotate(180deg);
  }
}

.subskills-shelf {
  background: color-mix(in srgb, var(--lumina-surface-2) 65%, transparent);
  border: 0.5px solid var(--lumina-separator);
  border-radius: 6px;
  display: flex;
  flex-direction: column;
  max-height: 160px;
  overflow-y: auto;
  padding: 4px 8px;
}

.subskill-row {
  align-items: center;
  border-bottom: 0.5px solid var(--lumina-separator);
  display: flex;
  gap: 10px;
  justify-content: space-between;
  padding: 6px 4px;

  &:last-child {
    border-bottom: none;
  }
}

.subskill-meta {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-width: 0;
}

.subskill-name-row {
  align-items: center;
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.subskill-name {
  color: var(--lumina-text);
  font-family: var(--lumina-font-mono, monospace);
  font-size: 12px;
  font-weight: 600;
}

.subskill-title {
  color: var(--lumina-text-secondary);
  font-size: 11.5px;
  font-weight: 500;
}

.subskill-desc {
  color: var(--lumina-text-secondary);
  font-size: 11.5px;
  line-height: 1.4;
  margin: 2px 0 0;
}

.subskill-install-btn {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-primary) 12%, transparent);
  border: 0.5px solid color-mix(in srgb, var(--lumina-primary) 28%, transparent);
  border-radius: 4px;
  color: var(--lumina-primary);
  cursor: pointer;
  display: inline-flex;
  flex-shrink: 0;
  font-size: 11px;
  font-weight: 600;
  gap: 3px;
  height: 22px;
  padding: 0 7px;
  transition: all 120ms ease;
  white-space: nowrap;

  &:hover {
    background: var(--lumina-primary);
    color: #ffffff;
  }

  &:active {
    transform: scale(0.96);
  }
}

.card-desc {
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: -webkit-box;
  font-size: 12px;
  height: 36px;
  line-height: 1.5;
  margin: 0;
  overflow: hidden;
}

.card-tags-row {
  align-items: center;
  display: flex;
  gap: 6px;
}

.micro-custom-tag {
  background: var(--lumina-primary-soft);
  border-radius: 3px;
  color: var(--lumina-primary);
  font-size: 11.5px;
  font-weight: 600;
  padding: 1.5px 6px;
}

.micro-tag {
  background: var(--lumina-surface-secondary);
  border-radius: 3px;
  color: var(--lumina-text-secondary);
  font-size: 11.5px;
  padding: 1.5px 6px;
}

.micro-stars-counter {
  color: var(--lumina-text-secondary);
  font-size: 11.5px;
  font-weight: 600;
  margin-left: auto;
}

.card-footer {
  align-items: center;
  border-top: 0.5px solid var(--lumina-separator);
  display: flex;
  gap: 8px;
  justify-content: space-between;
  margin-top: auto;
  padding-top: 8px;
}

.card-agent-dots {
  align-items: center;
  display: flex;
  gap: 4px;
}

.agent-dot {
  background: var(--lumina-text-tertiary);
  border-radius: 50%;
  display: inline-block;
  height: 6px;
  width: 6px;

  &--antigravity {
    background: #39786f;
  }

  &--claude {
    background: #d97706;
  }

  &--cursor {
    background: #2563eb;
  }

  &--trae {
    background: #7c3aed;
  }
}

.footer-buttons {
  align-items: center;
  display: flex;
  gap: 6px;
}

.card-action-btn {
  align-items: center;
  border-radius: var(--lumina-radius-sm, 6px);
  cursor: pointer;
  display: inline-flex;
  flex-shrink: 0;
  font-size: 11.5px;
  font-weight: 500;
  gap: 4px;
  height: 26px;
  padding: 0 10px;
  transition: all 120ms ease;
  white-space: nowrap;

  &:active:not(:disabled) {
    transform: scale(0.98);
  }

  &--ghost {
    background: transparent;
    border: 0.5px solid var(--lumina-separator);
    color: var(--lumina-text-secondary);

    &:hover {
      background: var(--lumina-control-hover);
      color: var(--lumina-text);
    }
  }

  &--primary {
    background: var(--lumina-primary);
    border: 0.5px solid transparent;
    color: #ffffff;
    font-weight: 600;

    &:hover {
      filter: brightness(1.08);
      transform: translateY(-1px);
    }
  }

  &--installed {
    background: color-mix(in srgb, var(--lumina-success, #27853f) 12%, transparent);
    border: 0.5px solid color-mix(in srgb, var(--lumina-success, #27853f) 20%, transparent);
    color: var(--lumina-success, #27853f);
    cursor: default;
  }
}
</style>
