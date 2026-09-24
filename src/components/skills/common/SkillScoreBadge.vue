<template>
  <div class="skill-score-group">
    <!-- 权威认证徽章 -->
    <span v-if="badge" class="score-badge" :data-badge="badge">
      <Icon :icon="badgeIcon" class="badge-glyph" />
      <span>{{ badgeLabel }}</span>
    </span>

    <!-- 综合量化评分 (Halowake Score) -->
    <span v-if="score !== undefined" class="score-pill" :title="`Halowake 综合评分: ${score}/100`">
      <Icon icon="solar:star-bold" class="score-star-glyph" />
      <span class="score-num">{{ score }}</span>
      <span class="score-scale">分</span>
    </span>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { Icon } from '@iconify/vue'
import { useI18n } from 'vue-i18n'
import type { SkillBadgeType } from '@/types/skills'

const props = defineProps<{
  score?: number
  badge?: SkillBadgeType
}>()

const { t } = useI18n()

const badgeLabel = computed(() => {
  switch (props.badge) {
    case 'verified':
      return t('skills.badgeVerified')
    case 'trending':
      return t('skills.badgeTrending')
    case 'security':
      return t('skills.badgeSecurity')
    case 'community':
      return t('skills.badgeCommunity')
    default:
      return ''
  }
})

const badgeIcon = computed(() => {
  switch (props.badge) {
    case 'verified':
      return 'solar:verified-check-bold'
    case 'trending':
      return 'solar:fire-bold'
    case 'security':
      return 'solar:shield-check-bold'
    case 'community':
      return 'solar:users-group-two-rounded-linear'
    default:
      return 'solar:star-linear'
  }
})
</script>

<style scoped lang="scss">
.skill-score-group {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.score-badge {
  display: inline-flex;
  align-items: center;
  gap: 3.5px;
  padding: 1.5px 6px;
  border-radius: 4px;
  font-size: 11.5px;
  font-weight: 600;
  line-height: 1.2;
}

.badge-glyph {
  font-size: 12px;
  flex-shrink: 0;
}

/* 官方精选 - 琥珀金 */
.score-badge[data-badge='verified'] {
  background: color-mix(in srgb, var(--lumina-warning, #b76500) 12%, transparent);
  color: var(--lumina-warning, #b76500);
}

/* 社区热门 - 烈焰橙红 */
.score-badge[data-badge='trending'] {
  background: color-mix(in srgb, var(--lumina-danger, #d9363e) 12%, transparent);
  color: var(--lumina-danger, #d9363e);
}

/* 安全认证 - 科技绿 */
.score-badge[data-badge='security'] {
  background: color-mix(in srgb, var(--lumina-success, #27853f) 12%, transparent);
  color: var(--lumina-success, #27853f);
}

/* 社区开源 - 柔和次要字色 */
.score-badge[data-badge='community'] {
  background: var(--lumina-surface-secondary);
  color: var(--lumina-text-secondary);
}

.score-pill {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  padding: 1.5px 6px;
  border-radius: 9999px;
  background: var(--lumina-primary-soft);
  color: var(--lumina-primary);
  font-size: 11.5px;
  font-weight: 600;
  line-height: 1.2;
}

.score-star-glyph {
  font-size: 11.5px;
  color: var(--lumina-warning, #b76500);
}

.score-num {
  font-family: var(--lumina-font-mono, monospace);
  font-weight: 700;
}

.score-scale {
  font-size: 11px;
  opacity: 0.85;
  font-weight: 500;
}
</style>
