<template>
  <div class="workbench-identity" :title="title || value">
    <span>{{ label }}</span>
    <strong>{{ value }}</strong>
    <slot />
  </div>
</template>

<script setup lang="ts">
defineProps<{
  label: string
  value: string
  title?: string
}>()
</script>

<style scoped lang="scss">
.workbench-identity {
  align-items: center;
  background: var(--lumina-surface-3);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-sm);
  display: inline-flex;
  flex: 0 0 230px;
  gap: 8px;
  height: 28px;
  min-width: 0;
  overflow: hidden;
  padding: 0 4px 0 10px;

  span {
    color: var(--lumina-text-secondary);
    flex: 0 0 auto;
    font-size: 11px;
  }

  strong {
    color: var(--lumina-text);
    flex: 1;
    font-size: 12px;
    font-weight: 500;
    min-width: 0;
  }

  span,
  strong {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
}
</style>
