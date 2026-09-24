<template>
  <div class="skill-explore-container">
    <!-- Explore Toolbar: Stage Tabs & Search Input & Online Sync -->
    <div class="explore-toolbar">
      <div class="macos-search-box">
        <Icon icon="solar:magnifer-linear" class="search-glyph" />
        <input
          v-model="exploreStore.searchQuery"
          type="text"
          class="macos-search-field"
          :placeholder="t('skills.exploreSearchPlaceholder')"
          autocomplete="off"
          spellcheck="false"
        />
        <button
          v-if="exploreStore.searchQuery"
          type="button"
          class="search-clear-glyph"
          @click="exploreStore.searchQuery = ''"
        >
          ×
        </button>
      </div>

      <!-- Segmented SDLC Stages Bar + User Custom Categories -->
      <div class="stage-segmented-bar" role="tablist">
        <button
          type="button"
          role="tab"
          class="stage-tab-btn"
          :class="{ active: exploreStore.selectedStage === 'all' }"
          @click="exploreStore.selectedStage = 'all'"
        >
          <Icon icon="solar:layers-minimalistic-linear" />
          <span>{{ t('skills.stageAll') }}</span>
          <span class="stage-tab-badge">{{ exploreStore.stageCounts.all || 0 }}</span>
        </button>

        <button
          type="button"
          role="tab"
          class="stage-tab-btn"
          :class="{ active: exploreStore.selectedStage === 'requirement' }"
          @click="exploreStore.selectedStage = 'requirement'"
        >
          <Icon icon="solar:clipboard-list-linear" />
          <span>{{ t('skills.stageRequirement') }}</span>
          <span class="stage-tab-badge">{{ exploreStore.stageCounts.requirement || 0 }}</span>
        </button>

        <button
          type="button"
          role="tab"
          class="stage-tab-btn"
          :class="{ active: exploreStore.selectedStage === 'design' }"
          @click="exploreStore.selectedStage = 'design'"
        >
          <Icon icon="solar:palette-linear" />
          <span>{{ t('skills.stageDesign') }}</span>
          <span class="stage-tab-badge">{{ exploreStore.stageCounts.design || 0 }}</span>
        </button>

        <button
          type="button"
          role="tab"
          class="stage-tab-btn"
          :class="{ active: exploreStore.selectedStage === 'coding' }"
          @click="exploreStore.selectedStage = 'coding'"
        >
          <Icon icon="solar:code-2-linear" />
          <span>{{ t('skills.stageCoding') }}</span>
          <span class="stage-tab-badge">{{ exploreStore.stageCounts.coding || 0 }}</span>
        </button>

        <button
          type="button"
          role="tab"
          class="stage-tab-btn"
          :class="{ active: exploreStore.selectedStage === 'testing' }"
          @click="exploreStore.selectedStage = 'testing'"
        >
          <Icon icon="solar:shield-check-linear" />
          <span>{{ t('skills.stageTesting') }}</span>
          <span class="stage-tab-badge">{{ exploreStore.stageCounts.testing || 0 }}</span>
        </button>

        <button
          type="button"
          role="tab"
          class="stage-tab-btn"
          :class="{ active: exploreStore.selectedStage === 'delivery' }"
          @click="exploreStore.selectedStage = 'delivery'"
        >
          <Icon icon="solar:rocket-2-linear" />
          <span>{{ t('skills.stageDelivery') }}</span>
          <span class="stage-tab-badge">{{ exploreStore.stageCounts.delivery || 0 }}</span>
        </button>

        <button
          v-if="exploreStore.starredCount > 0"
          type="button"
          role="tab"
          class="stage-tab-btn stage-tab-btn--star"
          :class="{ active: exploreStore.selectedStage === 'starred' }"
          @click="exploreStore.selectedStage = 'starred'"
        >
          <Icon icon="solar:star-bold" />
          <span>{{ t('skills.stageStarred') }}</span>
          <span class="stage-tab-badge stage-tab-badge--star">{{ exploreStore.starredCount }}</span>
        </button>

        <!-- 动态用户自定义分类标签 -->
        <button
          v-for="customCat in exploreStore.allCustomCategories"
          :key="customCat"
          type="button"
          role="tab"
          class="stage-tab-btn stage-tab-btn--custom"
          :class="{ active: exploreStore.selectedStage === customCat }"
          @click="exploreStore.selectedStage = customCat"
        >
          <Icon icon="solar:tag-linear" />
          <span>#{{ customCat }}</span>
          <span class="stage-tab-badge">{{ exploreStore.stageCounts[customCat] || 0 }}</span>
        </button>
      </div>

      <!-- 数据源/注册表设置快捷入口 -->
      <button
        type="button"
        class="sources-shortcut-btn"
        :title="'管理技能注册表与镜像源'"
        @click="exploreStore.openSourcesModal"
      >
        <Icon icon="solar:database-bold" class="shortcut-icon" />
        <span>注册表源</span>
      </button>
    </div>

    <!-- Main Viewport Scroll Area -->
    <div class="explore-viewport">
      <!-- Section 1: 研发全流程流水线套餐 (仅在查看全部阶段且无搜索时高亮展示) -->
      <section v-if="showPipelinesSection" class="explore-section">
        <div class="section-title-bar">
          <div class="title-with-desc">
            <h3 class="section-heading">
              <Icon icon="solar:fire-bold" class="heading-fire-icon" />
              <span>{{ t('skills.pipelineTitle') }}</span>
            </h3>
            <p class="section-subtext">{{ t('skills.pipelineSubtitle') }}</p>
          </div>
        </div>

        <div class="pipelines-grid">
          <PipelineCard
            v-for="pipeline in exploreStore.pipelines"
            :key="pipeline.id"
            :pipeline="pipeline"
          />
        </div>
      </section>

      <!-- Section 2: 全流程技能矩阵 (按 SDLC 阶段卡片网格呈现) -->
      <section class="explore-section">
        <div class="section-title-bar">
          <div class="title-with-desc">
            <h3 class="section-heading">
              <Icon icon="solar:layers-bold" class="heading-layer-icon" />
              <span>
                {{ currentSectionHeading }}
                <span class="section-count">({{ exploreStore.filteredSkills.length }})</span>
              </span>
            </h3>
          </div>
        </div>

        <!-- 空状态 / 同步中状态 -->
        <div v-if="exploreStore.isSyncing && exploreStore.filteredSkills.length === 0" class="empty-explore-state">
          <Icon icon="solar:restart-linear" class="empty-glyph spin-glyph" />
          <p>正在从远程注册表拉取最新技能数据…</p>
        </div>

        <div v-else-if="exploreStore.filteredSkills.length === 0" class="empty-explore-state">
          <Icon icon="solar:box-minimalistic-linear" class="empty-glyph" />
          <p v-if="exploreStore.searchQuery">{{ t('skills.noMatchingSkills') }}</p>
          <div v-else class="empty-sync-actions">
            <p>本地暂未同步技能库数据</p>
            <div class="empty-btn-group">
              <button
                type="button"
                class="empty-action-btn empty-action-btn--primary"
                :disabled="exploreStore.isSyncing"
                @click="exploreStore.syncRegistry()"
              >
                <Icon icon="solar:restart-linear" :class="{ 'spin-glyph': exploreStore.isSyncing }" />
                <span>立即同步远程技能库</span>
              </button>
              <button
                type="button"
                class="empty-action-btn empty-action-btn--secondary"
                @click="exploreStore.openSourcesModal"
              >
                <Icon icon="solar:settings-linear" />
                <span>配置镜像源</span>
              </button>
            </div>
          </div>
        </div>

        <!-- 技能卡片网格 -->
        <div v-else class="skills-catalog-grid">
          <SkillCard
            v-for="skill in exploreStore.filteredSkills"
            :key="skill.id"
            :skill="skill"
          />
        </div>
      </section>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { Icon } from '@iconify/vue'
import { useI18n } from 'vue-i18n'
import { useSkillExploreStore } from '@/stores/skill-explore'
import PipelineCard from './PipelineCard.vue'
import SkillCard from './SkillCard.vue'

const { t } = useI18n()
const exploreStore = useSkillExploreStore()

const showPipelinesSection = computed(() => {
  return exploreStore.selectedStage === 'all' && !exploreStore.searchQuery.trim()
})


const currentSectionHeading = computed(() => {
  switch (exploreStore.selectedStage) {
    case 'requirement':
      return `阶段 1：${t('skills.stageRequirement')}`
    case 'design':
      return `阶段 2：${t('skills.stageDesign')}`
    case 'coding':
      return `阶段 3：${t('skills.stageCoding')}`
    case 'testing':
      return `阶段 4：${t('skills.stageTesting')}`
    case 'delivery':
      return `阶段 5：${t('skills.stageDelivery')}`
    case 'starred':
      return t('skills.stageStarred')
    default:
      if (exploreStore.allCustomCategories.includes(exploreStore.selectedStage)) {
        return `自定义分类：#${exploreStore.selectedStage}`
      }
      return '全流程精选技能库'
  }
})
</script>

<style scoped lang="scss">
.skill-explore-container {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

.explore-toolbar {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-surface-base) 80%, transparent);
  border-bottom: 0.5px solid var(--lumina-separator);
  display: flex;
  flex-shrink: 0;
  gap: 12px;
  padding: 8px 18px;
}

.macos-search-box {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-fill-quaternary) 75%, transparent);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-md, 7px);
  cursor: text;
  display: flex;
  gap: 6px;
  height: 28px;
  padding: 0 8px;
  transition: all 120ms ease;
  width: 240px;
  flex-shrink: 0;

  &:focus-within {
    background: var(--lumina-surface-elevated);
    border-color: var(--lumina-primary);
    box-shadow: 0 0 0 2px var(--lumina-primary-soft);
  }

  .search-glyph {
    color: var(--lumina-text-tertiary);
    font-size: 13px;
    flex-shrink: 0;
  }

  .macos-search-field {
    background: transparent;
    border: none;
    color: var(--lumina-text);
    flex: 1;
    font-size: 11.5px;
    min-width: 0;
    outline: none;

    &::placeholder {
      color: var(--lumina-text-tertiary);
    }
  }

  .search-clear-glyph {
    background: transparent;
    border: none;
    color: var(--lumina-text-tertiary);
    cursor: pointer;
    font-size: 14px;
    line-height: 1;
    padding: 0;

    &:hover {
      color: var(--lumina-text);
    }
  }
}

.stage-segmented-bar {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-fill-quaternary) 65%, transparent);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-md, 7px);
  display: flex;
  gap: 2px;
  flex: 1;
  overflow-x: auto;
  padding: 2px;

  &::-webkit-scrollbar {
    display: none;
  }
}

.stage-tab-btn {
  align-items: center;
  background: transparent;
  border: none;
  border-radius: 5px;
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: inline-flex;
  font-size: 11px;
  font-weight: 500;
  gap: 4px;
  height: 24px;
  padding: 0 8px;
  transition: all 120ms ease;
  white-space: nowrap;

  &:hover {
    color: var(--lumina-text);
  }

  &.active {
    background: var(--lumina-surface-elevated);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
    color: var(--lumina-primary);
    font-weight: 600;
  }

  &--star.active {
    color: var(--lumina-warning, #b76500);
  }

  &--custom.active {
    color: var(--lumina-primary);
  }
}

.stage-tab-badge {
  background: color-mix(in srgb, var(--lumina-fill-tertiary) 70%, transparent);
  border-radius: 9999px;
  font-size: 9.5px;
  font-weight: 600;
  padding: 0.5px 5px;

  &--star {
    background: color-mix(in srgb, var(--lumina-warning, #b76500) 18%, transparent);
    color: var(--lumina-warning, #b76500);
  }
}

.explore-viewport {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 20px;
  min-height: 0;
  overflow-y: auto;
  padding: 16px 18px;
}

.explore-section {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.section-title-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.title-with-desc {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.section-heading {
  align-items: center;
  color: var(--lumina-text);
  display: flex;
  font-size: 13px;
  font-weight: 700;
  gap: 6px;
  margin: 0;
}

.heading-fire-icon {
  color: var(--lumina-danger, #d9363e);
  font-size: 15px;
}

.heading-layer-icon {
  color: var(--lumina-primary);
  font-size: 15px;
}

.section-subtext {
  color: var(--lumina-text-secondary);
  font-size: 11px;
  margin: 0;
}

.section-count {
  color: var(--lumina-text-tertiary);
  font-size: 11.5px;
  font-weight: 500;
}

.pipelines-grid {
  display: grid;
  gap: 12px;
  grid-template-columns: repeat(3, minmax(0, 1fr));
}

@media (max-width: 1040px) {
  .pipelines-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 680px) {
  .pipelines-grid {
    grid-template-columns: 1fr;
  }
}

.skills-catalog-grid {
  display: grid;
  gap: 12px;
  grid-template-columns: repeat(3, 1fr);
}

@media (max-width: 980px) {
  .skills-catalog-grid {
    grid-template-columns: repeat(2, 1fr);
  }
}

@media (max-width: 650px) {
  .skills-catalog-grid {
    grid-template-columns: 1fr;
  }
}

.empty-explore-state {
  align-items: center;
  color: var(--lumina-text-tertiary);
  display: flex;
  flex-direction: column;
  gap: 8px;
  justify-content: center;
  padding: 40px;

  .empty-glyph {
    font-size: 32px;
  }
}

.sources-shortcut-btn {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-primary) 10%, transparent);
  border: 0.5px solid color-mix(in srgb, var(--lumina-primary) 30%, transparent);
  border-radius: var(--lumina-radius-md, 7px);
  color: var(--lumina-primary);
  cursor: pointer;
  display: flex;
  font-size: 11.5px;
  font-weight: 600;
  gap: 5px;
  height: 28px;
  margin-left: auto;
  padding: 0 10px;
  transition: all 120ms ease;
  flex-shrink: 0;

  &:hover {
    background: var(--lumina-primary);
    border-color: var(--lumina-primary);
    color: #ffffff;

    .source-count-pill {
      background: rgba(255, 255, 255, 0.25);
      color: #ffffff;
    }
  }

  .shortcut-icon {
    font-size: 13px;
  }

  .source-count-pill {
    background: color-mix(in srgb, var(--lumina-primary) 20%, transparent);
    border-radius: 10px;
    font-size: 10px;
    font-weight: 700;
    padding: 0 5px;
    transition: all 120ms ease;
  }
}

.empty-sync-actions {
  align-items: center;
  display: flex;
  flex-direction: column;
  gap: 12px;
  margin-top: 10px;

  p {
    color: var(--lumina-text-secondary);
    font-size: 13px;
    margin: 0;
  }
}

.empty-btn-group {
  display: flex;
  gap: 10px;
}

.empty-action-btn {
  align-items: center;
  border-radius: var(--lumina-radius-sm, 6px);
  cursor: pointer;
  display: inline-flex;
  font-size: 12px;
  font-weight: 550;
  gap: 6px;
  height: 32px;
  padding: 0 14px;
  transition: all 0.15s ease;

  &--primary {
    background: var(--lumina-primary);
    border: 0;
    color: var(--lumina-on-accent, #ffffff);

    &:hover:not(:disabled) {
      opacity: 0.9;
    }
  }

  &--secondary {
    background: var(--lumina-surface-2);
    border: 0.5px solid var(--lumina-separator);
    color: var(--lumina-text);

    &:hover {
      background: var(--lumina-control-hover);
    }
  }

  &:disabled {
    cursor: not-allowed;
    opacity: 0.5;
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
