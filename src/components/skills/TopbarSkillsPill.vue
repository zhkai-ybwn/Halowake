<template>
  <div class="topbar-skills-pill-host" @mousedown.stop @dblclick.stop>
    <button
      type="button"
      class="topbar-skills-pill"
      :class="{
        'has-updates': store.hasUpdates,
        'is-checking': store.checkingUpdates,
      }"
      :title="tooltipText"
      @click="store.openModal"
    >
      <span class="pill-dot" :class="dotClass"></span>
      <Icon
        :icon="store.checkingUpdates ? 'solar:refresh-linear' : 'solar:magic-stick-3-linear'"
        class="pill-icon"
        :class="{ 'spin-icon': store.checkingUpdates }"
      />
      <span class="pill-text">{{ pillLabel }}</span>
    </button>

    <!-- Skills Manager Modal -->
    <SkillsManagerModal />
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { Icon } from '@iconify/vue'
import { useI18n } from 'vue-i18n'
import { useSkillsStore } from '@/stores/skills'
import SkillsManagerModal from './SkillsManagerModal.vue'

const { t } = useI18n()
const store = useSkillsStore()

const dotClass = computed(() => {
  if (store.hasUpdates) return 'warning'
  if (store.skills.length > 0) return 'healthy'
  return 'idle'
})

const pillLabel = computed(() => {
  if (store.hasUpdates) {
    return `Skills (${store.updateCount})`
  }
  return 'Skills'
})

const tooltipText = computed(() => {
  if (store.hasUpdates) {
    return `${t('skills.pillTitle')} - ${t('skills.pillUpdatesAvailable', { count: store.updateCount })}`
  }
  return `${t('skills.pillTitle')} (${store.skills.length})`
})
</script>

<style scoped lang="scss">
.topbar-skills-pill-host {
  align-items: center;
  display: flex;
}

.topbar-skills-pill {
  align-items: center;
  background: var(--lumina-control-bg);
  border: 0.5px solid var(--lumina-separator);
  border-radius: 13px;
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: flex;
  font-size: 11.5px;
  gap: 6px;
  height: 26px;
  padding: 0 9px;
  transition: all var(--lumina-duration-fast) var(--lumina-ease-out);

  &:hover {
    background: var(--lumina-control-hover);
    color: var(--lumina-text);
  }

  &.has-updates {
    border-color: rgba(245, 158, 11, 0.4);
    color: var(--lumina-text);

    .pill-text {
      color: #f59e0b;
      font-weight: 600;
    }
  }
}

.pill-dot {
  border-radius: 50%;
  flex: 0 0 6px;
  height: 6px;
  width: 6px;

  &.healthy {
    background: #34c759;
    box-shadow: 0 0 6px rgba(52, 199, 89, 0.4);
  }

  &.warning {
    background: #ff9500;
    box-shadow: 0 0 6px rgba(255, 149, 0, 0.4);
  }

  &.idle {
    background: var(--lumina-text-tertiary);
  }
}

.pill-icon {
  font-size: 13px;

  &.spin-icon {
    animation: spin 1s linear infinite;
  }
}

.pill-text {
  font-family: var(--lumina-font-mono, monospace);
  font-weight: 550;
  white-space: nowrap;
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
