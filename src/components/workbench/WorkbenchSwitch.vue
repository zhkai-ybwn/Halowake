<template>
  <div class="workbench-switch" :aria-label="ariaLabel">
    <button
      v-for="item in items"
      :key="item.key"
      class="workbench-switch-item"
      type="button"
      :class="{ active: item.key === activeKey }"
      @click="handleSelect(item.key)"
    >
      {{ item.label }}
    </button>
  </div>
</template>

<script setup lang="ts">
const props = defineProps<{
  activeKey: string
  ariaLabel: string
  items: Array<{
    key: string
    label: string
  }>
}>()

const emit = defineEmits<{
  (e: 'select', key: string): void
}>()

function handleSelect(key: string) {
  if (key !== props.activeKey) {
    emit('select', key)
  }
}
</script>

<style scoped lang="scss">
.workbench-switch {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-surface-2) 70%, var(--lumina-surface-3));
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-sm);
  display: inline-flex;
  flex: 0 0 auto;
  font-size: 11px;
  gap: 2px;
  height: 28px;
  padding: 2px;
  box-shadow: inset 0 0 0 0.5px color-mix(in srgb, var(--lumina-separator) 42%, transparent);
}

.workbench-switch-item {
  background: transparent;
  border: 0;
  border-radius: 6px;
  color: var(--lumina-text-secondary);
  cursor: pointer;
  font-size: 11px;
  font-weight: 650;
  height: 22px;
  padding: 0 10px;
  white-space: nowrap;

  &:hover {
    background: var(--lumina-button-secondary-hover);
    color: var(--lumina-text);
  }

  &.active {
    background: var(--lumina-surface-1);
    box-shadow: var(--lumina-shadow-sm);
    color: var(--lumina-text);
    cursor: default;
  }
}
</style>
