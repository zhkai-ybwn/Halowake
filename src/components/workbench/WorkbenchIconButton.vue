<template>
  <button
    class="workbench-icon-button"
    :class="{ active }"
    type="button"
    :aria-label="label"
    :title="shortcut ? `${label}  ${shortcut}` : label"
    :disabled="disabled"
  >
    <Icon :icon="icon" />
  </button>
</template>

<script setup lang="ts">
withDefaults(defineProps<{
  active?: boolean
  disabled?: boolean
  icon: string
  label: string
  shortcut?: string
}>(), {
  active: false,
  disabled: false,
  shortcut: '',
})
</script>

<style scoped lang="scss">
.workbench-icon-button {
  align-items: center;
  background: transparent;
  border: 0.5px solid transparent;
  border-radius: var(--lumina-radius-sm);
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: inline-flex;
  flex: 0 0 auto;
  height: var(--lumina-control-height);
  justify-content: center;
  padding: 0;
  transition:
    background var(--lumina-duration-fast) var(--lumina-ease-out),
    color var(--lumina-duration-fast) var(--lumina-ease-out),
    transform var(--lumina-duration-fast) var(--lumina-ease-out);
  width: var(--lumina-control-height);

  &:hover:not(:disabled) {
    background: var(--lumina-control-hover);
    color: var(--lumina-text);
  }

  &:active:not(:disabled) {
    transform: scale(0.94);
  }

  &.active {
    background: var(--lumina-primary-soft);
    color: var(--lumina-primary);
  }

  &:disabled {
    cursor: not-allowed;
    opacity: 0.42;
  }

  svg {
    height: 17px;
    width: 17px;
  }
}
</style>
