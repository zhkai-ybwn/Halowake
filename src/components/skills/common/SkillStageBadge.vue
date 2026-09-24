<template>
  <span class="skill-stage-badge" :data-stage="stage" :title="stageLabel">
    <Icon :icon="stageIcon" class="stage-glyph" />
    <span>{{ stageLabel }}</span>
  </span>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { Icon } from '@iconify/vue'
import { useI18n } from 'vue-i18n'
import type { SdlcStage } from '@/types/skills'

const props = defineProps<{
  stage: SdlcStage
}>()

const { t } = useI18n()

const stageLabel = computed(() => {
  switch (props.stage) {
    case 'requirement':
      return t('skills.stageRequirement')
    case 'design':
      return t('skills.stageDesign')
    case 'coding':
      return t('skills.stageCoding')
    case 'testing':
      return t('skills.stageTesting')
    case 'delivery':
      return t('skills.stageDelivery')
    default:
      return props.stage
  }
})

const stageIcon = computed(() => {
  switch (props.stage) {
    case 'requirement':
      return 'solar:clipboard-list-linear'
    case 'design':
      return 'solar:palette-linear'
    case 'coding':
      return 'solar:code-2-linear'
    case 'testing':
      return 'solar:shield-check-linear'
    case 'delivery':
      return 'solar:rocket-2-linear'
    default:
      return 'solar:widget-linear'
  }
})
</script>

<style scoped>
.skill-stage-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 2px 7px;
  border-radius: 9999px;
  font-size: 11.5px;
  font-weight: 600;
  line-height: 1.3;
  letter-spacing: 0.01em;
  user-select: none;
  border: 1px solid transparent;
  transition: all 0.15s ease;
}

.stage-glyph {
  font-size: 12px;
  flex-shrink: 0;
}

/* 需求阶段 - 亮蓝紫 */
.skill-stage-badge[data-stage='requirement'] {
  background: rgba(147, 51, 234, 0.1);
  color: #a855f7;
  border-color: rgba(168, 85, 247, 0.25);
}

/* 设计阶段 - 活力粉/洋红 */
.skill-stage-badge[data-stage='design'] {
  background: rgba(236, 72, 153, 0.1);
  color: #ec4899;
  border-color: rgba(236, 72, 153, 0.25);
}

/* 编码阶段 - 极客绿/翡翠绿 */
.skill-stage-badge[data-stage='coding'] {
  background: rgba(16, 185, 129, 0.1);
  color: #10b981;
  border-color: rgba(16, 185, 129, 0.25);
}

/* 测试阶段 - 琥珀/警戒橙 */
.skill-stage-badge[data-stage='testing'] {
  background: rgba(245, 158, 11, 0.1);
  color: #f59e0b;
  border-color: rgba(245, 158, 11, 0.25);
}

/* 交付阶段 - 科技深蓝 */
.skill-stage-badge[data-stage='delivery'] {
  background: rgba(59, 130, 246, 0.1);
  color: #3b82f6;
  border-color: rgba(59, 130, 246, 0.25);
}
</style>
