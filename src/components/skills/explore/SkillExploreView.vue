<template>
  <div class="skill-explore-container">
    <!-- Explore Toolbar: 2-Tier Hierarchy matching Installed tab -->
    <div class="explore-toolbar-panel">
      <!-- Tier 1: Search (Left) + Filters & View Mode (Right) -->
      <div class="explore-primary-row">
        <div class="macos-search-box">
          <Icon icon="solar:magnifer-linear" class="search-glyph" />
          <input
            ref="searchInputRef"
            v-model="exploreStore.searchQuery"
            type="text"
            class="macos-search-field"
            :placeholder="t('skills.exploreSearchPlaceholder')"
            autocomplete="off"
            spellcheck="false"
            :aria-label="t('skills.review.catalogSearchAria')"
          />
          <kbd class="search-shortcut">{{ isMac ? '⌘F' : 'Ctrl+F' }}</kbd>
          <button
            v-if="exploreStore.searchQuery"
            type="button"
            class="search-clear-glyph"
            :aria-label="t('skills.review.clearSearch')"
            @click="exploreStore.searchQuery = ''"
          >
            ×
          </button>
        </div>

        <div class="explore-controls-right">
          <label class="catalog-select-label">
            <Icon icon="solar:tag-linear" class="control-glyph" />
            <select v-model="selectedCategory" :aria-label="t('skills.review.categoryFilterAria')">
              <option value="all">{{ t('skills.review.allCategories') }}</option>
              <option v-for="category in availableCategories" :key="category.name" :value="category.name">
                {{ skillCategoryLabel(category.name) }} ({{ category.count }})
              </option>
            </select>
          </label>
          <label class="catalog-select-label">
            <Icon icon="solar:sort-vertical-linear" class="control-glyph" />
            <select v-model="sortBy" :aria-label="t('skills.review.sortAria')">
              <option value="relevance">{{ exploreStore.searchQuery.trim() ? t('skills.review.bestMatch') : t('skills.review.popularFirst') }}</option>
              <option value="popular">{{ t('skills.review.popularity') }}</option>
              <option value="name">{{ t('skills.review.nameAscending') }}</option>
            </select>
          </label>
          <div class="catalog-view-switch" role="group" :aria-label="t('skills.review.displayMode')">
            <button type="button" :class="{ active: viewMode === 'list' }" :aria-pressed="viewMode === 'list'" :aria-label="t('skills.review.listView')" @click="viewMode = 'list'"><Icon icon="solar:list-linear" /></button>
            <button type="button" :class="{ active: viewMode === 'grid' }" :aria-pressed="viewMode === 'grid'" :aria-label="t('skills.review.gridView')" @click="viewMode = 'grid'"><Icon icon="solar:widget-2-linear" /></button>
          </div>
        </div>
      </div>

      <!-- Tier 2: Segmented SDLC Stages Bar + User Custom Categories -->
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
    </div>

    <div class="section-title-bar catalog-title-bar">
      <div class="title-with-desc">
        <h3 class="section-heading">
          <Icon icon="solar:layers-bold" class="heading-layer-icon" />
          <span>
            {{ currentSectionHeading }}
            <span class="section-count">({{ sortedSkills.length }})</span>
          </span>
        </h3>
      </div>
      <div class="catalog-heading-hint">
        <Icon icon="solar:info-circle-linear" />
        <span>{{ t('skills.review.rowClickHint') }}</span>
      </div>
    </div>

    <!-- Main Viewport Scroll Area -->
    <div ref="viewportRef" class="explore-viewport">
      <p v-if="exploreStore.syncStatusMessage" class="catalog-sync-status" role="status">
        {{ exploreStore.syncStatusMessage }}
        <button type="button" :aria-label="t('skills.review.dismissStatus')" @click="exploreStore.syncStatusMessage = ''">×</button>
      </p>
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
        <!-- 空状态 / 同步中状态 -->
        <div v-if="exploreStore.isSyncing && sortedSkills.length === 0" class="empty-explore-state">
          <Icon icon="solar:restart-linear" class="empty-glyph spin-glyph" />
          <p>{{ t('skills.review.syncing') }}</p>
        </div>

        <div v-else-if="sortedSkills.length === 0" class="empty-explore-state">
          <Icon icon="solar:box-minimalistic-linear" class="empty-glyph" />
          <template v-if="exploreStore.catalogSkills.length > 0">
            <p>{{ t('skills.noMatchingSkills') }}</p>
            <button type="button" class="empty-action-btn empty-action-btn--secondary" @click="clearDiscoveryFilters">{{ t('skills.review.clearFilters') }}</button>
          </template>
          <div v-else class="empty-sync-actions">
            <p>{{ t('skills.review.noLocalCatalog') }}</p>
            <div class="empty-btn-group">
              <button
                type="button"
                class="empty-action-btn empty-action-btn--primary"
                :disabled="exploreStore.isSyncing"
                @click="exploreStore.syncRegistry()"
              >
                <Icon icon="solar:restart-linear" :class="{ 'spin-glyph': exploreStore.isSyncing }" />
                <span>{{ t('skills.review.syncNow') }}</span>
              </button>
            </div>
          </div>
        </div>

        <!-- 技能卡片网格 -->
        <div v-else-if="viewMode === 'list'" class="skills-catalog-list">
          <SkillCompactRow v-for="skill in visibleSkills" :key="skill.id" :skill="skill" />
        </div>
        <div v-else class="skills-catalog-grid">
          <SkillCard
            v-for="skill in visibleSkills"
            :key="skill.id"
            :skill="skill"
          />
        </div>
      </section>
    </div>
    <nav v-if="sortedSkills.length > 0" class="catalog-pagination" :aria-label="t('skills.review.pagination')">
      <div class="page-summary">
        <strong>{{ sortedSkills.length }}</strong>
        <span>{{ t('skills.review.skillsUnit') }}</span>
        <span class="page-summary-divider" aria-hidden="true"></span>
        <span>{{ t('skills.review.showing', { start: (currentPage - 1) * pageSize + 1, end: Math.min(currentPage * pageSize, sortedSkills.length) }) }}</span>
      </div>
      <div class="page-actions">
        <div class="page-controls">
          <button type="button" :disabled="currentPage === 1" :aria-label="t('skills.review.previousPage')" @click="goToPage(currentPage - 1)"><Icon icon="solar:alt-arrow-left-linear" /></button>
          <template v-for="(item, index) in pageButtons" :key="`${item}-${index}`">
            <span v-if="item === '…'" class="page-ellipsis">…</span>
            <button v-else type="button" :class="{ active: item === currentPage }" :aria-label="t('skills.review.pageNumber', { page: item })" :aria-current="item === currentPage ? 'page' : undefined" @click="goToPage(Number(item))">{{ item }}</button>
          </template>
          <button type="button" :disabled="currentPage === pageCount" :aria-label="t('skills.review.nextPage')" @click="goToPage(currentPage + 1)"><Icon icon="solar:alt-arrow-right-linear" /></button>
        </div>
        <div v-if="pageCount > 7" class="page-jump">
          <label>{{ t('skills.review.jumpTo') }} <input v-model="jumpPage" type="number" min="1" :max="pageCount" :aria-label="t('skills.review.jumpAria')" @keydown.enter="submitJumpPage" /> {{ t('skills.review.page') }}</label>
          <button type="button" @click="submitJumpPage">{{ t('skills.review.jump') }}</button>
        </div>
        <label class="page-size">{{ t('skills.review.perPage') }} <select v-model.number="pageSize" :aria-label="t('skills.review.perPageAria')"><option :value="24">24</option><option :value="48">48</option><option :value="96">96</option></select> {{ t('skills.review.items') }}</label>
      </div>
    </nav>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { Icon } from '@iconify/vue'
import { useI18n } from 'vue-i18n'
import { useSkillExploreStore } from '@/stores/skill-explore'
import { useSkillsStore } from '@/stores/skills'
import PipelineCard from './PipelineCard.vue'
import SkillCard from './SkillCard.vue'
import SkillCompactRow from './SkillCompactRow.vue'
import { skillCategoryLabel } from '@/utils/skill-category-label'

const { t } = useI18n()
const exploreStore = useSkillExploreStore()
const skillsStore = useSkillsStore()
const searchInputRef = ref<HTMLInputElement | null>(null)
const viewportRef = ref<HTMLElement | null>(null)
const selectedCategory = ref('all')
const sortBy = ref<'relevance' | 'popular' | 'name'>('relevance')
const viewMode = ref<'list' | 'grid'>('list')
const currentPage = ref(1)
const pageSize = ref(24)
const jumpPage = ref('')
const isMac = navigator.platform.toLowerCase().includes('mac')

const availableCategories = computed(() => {
  const counts = new Map<string, number>()
  for (const skill of exploreStore.filteredSkills) {
    counts.set(skill.category, (counts.get(skill.category) || 0) + 1)
  }
  return [...counts].map(([name, count]) => ({ name, count })).sort((a, b) => b.count - a.count || a.name.localeCompare(b.name))
})

function matchScore(skill: (typeof exploreStore.filteredSkills)[number], terms: string[]): number {
  const name = skill.name.toLowerCase()
  const title = skill.titleZh.toLowerCase()
  const category = `${skill.category} ${skillCategoryLabel(skill.category)}`.toLowerCase()
  const tags = skill.tags.join(' ').toLowerCase()
  return terms.reduce((score, term) => score +
    (name === term ? 120 : name.startsWith(term) ? 80 : name.includes(term) ? 60 : 0) +
    (title.includes(term) ? 30 : 0) +
    (category.includes(term) ? 20 : 0) +
    (tags.includes(term) ? 15 : 0), 0)
}

const sortedSkills = computed(() => {
  const skills = exploreStore.filteredSkills.filter(skill => selectedCategory.value === 'all' || skill.category === selectedCategory.value)
  const terms = exploreStore.searchQuery.trim().toLowerCase().split(/\s+/).filter(Boolean)
  return [...skills].sort((a, b) => {
    if (sortBy.value === 'name') return a.name.localeCompare(b.name)
    if (sortBy.value === 'relevance' && terms.length) {
      const difference = matchScore(b, terms) - matchScore(a, terms)
      if (difference) return difference
    }
    return (b.metrics?.githubStars ?? b.stars ?? 0) - (a.metrics?.githubStars ?? a.stars ?? 0) || a.name.localeCompare(b.name)
  })
})
const pageCount = computed(() => Math.max(1, Math.ceil(sortedSkills.value.length / pageSize.value)))
const visibleSkills = computed(() => sortedSkills.value.slice((currentPage.value - 1) * pageSize.value, currentPage.value * pageSize.value))
const pageButtons = computed<(number | '…')[]>(() => {
  const pages = new Set([1, pageCount.value])
  for (let page = Math.max(1, currentPage.value - 2); page <= Math.min(pageCount.value, currentPage.value + 2); page++) pages.add(page)
  const sorted = [...pages].sort((a, b) => a - b)
  return sorted.flatMap((page, index) => {
    if (index === 0) return [page]
    const gap = page - sorted[index - 1]
    return gap === 2 ? [page - 1, page] : gap > 2 ? ['…' as const, page] : [page]
  })
})

function goToPage(page: number) {
  currentPage.value = Math.min(Math.max(1, page), pageCount.value)
  viewportRef.value?.scrollTo({ top: 0, behavior: 'instant' })
}

function submitJumpPage() {
  const page = Number(jumpPage.value)
  if (Number.isInteger(page) && page >= 1) goToPage(page)
  jumpPage.value = ''
}

function clearDiscoveryFilters() {
  exploreStore.searchQuery = ''
  selectedCategory.value = 'all'
  exploreStore.selectedStage = 'all'
}

watch(() => [exploreStore.searchQuery, exploreStore.selectedStage, selectedCategory.value, sortBy.value, pageSize.value], () => {
  currentPage.value = 1
  viewportRef.value?.scrollTo({ top: 0 })
})
watch(pageCount, count => { if (currentPage.value > count) currentPage.value = count })

function handleSearchShortcut(event: KeyboardEvent) {
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'f' && skillsStore.modalVisible && exploreStore.activeMainTab === 'explore') {
    event.preventDefault()
    void nextTick(() => searchInputRef.value?.focus())
  }
}

onMounted(() => window.addEventListener('keydown', handleSearchShortcut))
onUnmounted(() => window.removeEventListener('keydown', handleSearchShortcut))

const showPipelinesSection = computed(() => {
  return exploreStore.pipelines.length > 0 && exploreStore.selectedStage === 'all' && !exploreStore.searchQuery.trim()
})


const currentSectionHeading = computed(() => {
  switch (exploreStore.selectedStage) {
    case 'requirement':
      return t('skills.review.stageHeading', { number: 1, name: t('skills.stageRequirement') })
    case 'design':
      return t('skills.review.stageHeading', { number: 2, name: t('skills.stageDesign') })
    case 'coding':
      return t('skills.review.stageHeading', { number: 3, name: t('skills.stageCoding') })
    case 'testing':
      return t('skills.review.stageHeading', { number: 4, name: t('skills.stageTesting') })
    case 'delivery':
      return t('skills.review.stageHeading', { number: 5, name: t('skills.stageDelivery') })
    case 'starred':
      return t('skills.stageStarred')
    default:
      if (exploreStore.allCustomCategories.includes(exploreStore.selectedStage)) {
        return t('skills.review.customHeading', { name: exploreStore.selectedStage })
      }
      return t('skills.review.catalogHeading')
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

.explore-toolbar-panel {
  background: color-mix(in srgb, var(--lumina-surface-base) 80%, transparent);
  border-bottom: 0.5px solid var(--lumina-separator);
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  gap: 8px;
  padding: 10px 18px;
}

.explore-primary-row {
  align-items: center;
  display: flex;
  gap: 12px;
  justify-content: space-between;
}

.explore-controls-right {
  align-items: center;
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.control-glyph {
  color: var(--lumina-text-tertiary);
  font-size: 12px;
}

.search-shortcut {
  color: var(--lumina-text-tertiary);
  flex-shrink: 0;
  font-size: 10px;
  font-weight: 500;
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
  padding: 0 10px;
  transition: all 120ms ease;
  width: 350px;
  max-width: 48vw;
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

.catalog-sync-status {
  align-items: center;
  color: var(--lumina-text-secondary);
  display: flex;
  font-size: 11px;
  gap: 8px;
  margin: 0;

  button {
    background: transparent;
    border: 0;
    color: inherit;
    cursor: pointer;
    font-size: 16px;
    line-height: 1;
    padding: 2px 4px;
  }
}

.section-title-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 10px;
}

.catalog-title-bar {
  align-items: center;
  background: var(--lumina-surface-base);
  border-bottom: 0.5px solid var(--lumina-separator);
  display: flex;
  flex-shrink: 0;
  justify-content: space-between;
  min-height: 38px;
  padding: 6px 18px;
}

.catalog-heading-hint {
  align-items: center;
  color: var(--lumina-text-tertiary);
  display: flex;
  font-size: 11px;
  gap: 5px;

  svg {
    font-size: 13px;
  }
}

.catalog-controls { align-items: center; display: flex; flex-wrap: wrap; gap: 8px; }
.catalog-select-label { align-items: center; color: var(--lumina-text-secondary); display: flex; font-size: 11px; gap: 5px; }
.catalog-select-label select, .page-size select {
  background: var(--lumina-surface-elevated);
  border: 0.5px solid var(--lumina-separator);
  border-radius: 6px;
  color: var(--lumina-text);
  font-size: 11px;
  font-weight: 500;
  height: 26px;
  max-width: 180px;
  padding: 0 6px;
  transition: all 120ms ease;

  &:hover {
    background: var(--lumina-control-hover);
  }
}
.catalog-view-switch {
  background: var(--lumina-fill-quaternary);
  border: 0.5px solid var(--lumina-separator);
  border-radius: 6px;
  display: flex;
  padding: 2px;

  button {
    align-items: center;
    background: transparent;
    border: 0;
    border-radius: 4px;
    color: var(--lumina-text-secondary);
    cursor: pointer;
    display: flex;
    height: 24px;
    justify-content: center;
    width: 27px;

    &.active { background: var(--lumina-surface-elevated); color: var(--lumina-primary); box-shadow: 0 1px 3px rgba(0, 0, 0, .08); }
  }
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

.skills-catalog-list {
  background: var(--lumina-surface-elevated);
  border: 0.5px solid var(--lumina-separator);
  border-radius: 9px;
  overflow: hidden;
}

.catalog-pagination {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-surface-elevated) 88%, transparent);
  border-top: 0.5px solid var(--lumina-separator);
  color: var(--lumina-text-secondary);
  display: flex;
  flex-shrink: 0;
  flex-wrap: wrap;
  font-size: 11px;
  gap: 6px 16px;
  justify-content: space-between;
  min-height: 38px;
  padding: 5px 18px;
}
.page-summary, .page-actions { align-items: center; display: flex; gap: 8px; }
.page-summary { font-variant-numeric: tabular-nums; white-space: nowrap; }
.page-summary strong { color: var(--lumina-text); font-size: 12px; }
.page-summary-divider { background: var(--lumina-separator); height: 14px; width: 1px; }
.page-actions { flex-wrap: wrap; gap: 8px 14px; justify-content: flex-end; }
.page-controls {
  align-items: center;
  background: var(--lumina-surface-base);
  border: 0.5px solid var(--lumina-separator);
  border-radius: 7px;
  display: flex;
  gap: 2px;
  padding: 2px;
}
.page-controls button, .page-jump button {
  align-items: center;
  background: transparent;
  border: 0;
  border-radius: 5px;
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: inline-flex;
  font-size: 11px;
  height: 25px;
  min-width: 25px;
  padding: 0 5px;
  justify-content: center;
  line-height: 1;

  :deep(svg) { display: block; height: 14px; width: 14px; }

  &:hover:not(:disabled) { background: var(--lumina-control-hover); color: var(--lumina-text); }
  &.active { background: var(--lumina-primary-soft); color: var(--lumina-primary); font-weight: 700; }
  &:disabled { cursor: default; opacity: .4; }
}
.page-ellipsis { padding: 0 4px; }
.page-jump { align-items: center; display: flex; gap: 3px; white-space: nowrap; }
.page-jump input {
  background: var(--lumina-surface-elevated);
  border: 0.5px solid var(--lumina-separator);
  border-radius: 5px;
  color: var(--lumina-text);
  font-size: 11px;
  height: 24px;
  text-align: center;
  width: 40px;
}
.page-size { align-items: center; display: flex; gap: 4px; white-space: nowrap; }
.page-size select { height: 24px; }

@media (max-width: 780px) {
  .catalog-pagination { justify-content: center; }
  .page-actions { justify-content: center; }
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
