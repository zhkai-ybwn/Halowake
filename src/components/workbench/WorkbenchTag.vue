<template>
  <span class="workbench-tag" :class="toneClass">
    <span v-if="label">{{ label }}</span>
    <strong>{{ value }}</strong>
  </span>
</template>

<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(defineProps<{
  label?: string
  value: string | number
  tone?: 'default' | 'primary' | 'success' | 'warning' | 'danger'
}>(), {
  label: '',
  tone: 'default',
})

const toneClass = computed(() => `tone-${props.tone}`)
</script>

<style scoped lang="scss">
.workbench-tag {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-surface-3) 82%, transparent);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-xs);
  display: inline-flex;
  flex: 0 0 auto;
  font-size: 11px;
  gap: 8px;
  height: 24px;
  min-width: 0;
  padding: 0 8px;
  white-space: nowrap;

  span {
    color: var(--lumina-text-secondary);
  }

  strong {
    color: var(--lumina-text);
    font-weight: 650;
  }

  &.tone-primary {
    background: color-mix(in srgb, var(--lumina-primary-soft) 58%, var(--lumina-surface-2));

    strong {
      color: var(--lumina-primary);
    }
  }

  &.tone-success strong {
    color: var(--lumina-success);
  }

  &.tone-warning strong {
    color: var(--lumina-warning);
  }

  &.tone-danger {
    background: color-mix(in srgb, var(--lumina-danger) 12%, transparent);

    strong {
      color: var(--lumina-danger);
    }
  }
}
</style>
