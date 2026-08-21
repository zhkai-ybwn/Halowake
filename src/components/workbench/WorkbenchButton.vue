<template>
  <button class="workbench-button" :class="[variantClass, `size-${size}`]" :type="type" :disabled="disabled">
    <span class="workbench-button-content"><slot /></span>
    <WorkbenchShortcutHint v-if="shortcut" :keys="shortcut" />
  </button>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import WorkbenchShortcutHint from './WorkbenchShortcutHint.vue'

const props = withDefaults(defineProps<{
  disabled?: boolean
  type?: 'button' | 'submit' | 'reset'
  variant?: 'secondary' | 'primary' | 'danger' | 'ghost'
  size?: 'default' | 'large'
  shortcut?: string
}>(), {
  disabled: false,
  type: 'button',
  variant: 'secondary',
  size: 'default',
  shortcut: '',
})

const variantClass = computed(() => `variant-${props.variant}`)
</script>

<style scoped lang="scss">
.workbench-button {
  align-items: center;
  background: var(--lumina-button-secondary-bg);
  border: 0.5px solid var(--lumina-separator-strong);
  border-radius: var(--lumina-radius-sm);
  color: var(--lumina-text);
  cursor: pointer;
  display: inline-flex;
  flex: 0 0 auto;
  font-size: 12px;
  font-weight: 500;
  gap: 8px;
  height: var(--lumina-control-height);
  justify-content: center;
  padding: 0 var(--lumina-control-padding-x);
  transition:
    background var(--lumina-duration-fast) var(--lumina-ease-out),
    border-color var(--lumina-duration-fast) var(--lumina-ease-out),
    color var(--lumina-duration-fast) var(--lumina-ease-out),
    transform var(--lumina-duration-fast) var(--lumina-ease-out);

  &:hover:not(:disabled) {
    background: var(--lumina-button-secondary-hover);
  }

  &:active:not(:disabled) {
    transform: scale(0.98);
  }

  &:focus-visible {
    box-shadow: 0 0 0 3px var(--lumina-accent-ring);
    outline: none;
  }

  &:disabled {
    cursor: not-allowed;
    opacity: 0.56;
  }
}

.workbench-button-content {
  align-items: center;
  display: inline-flex;
  gap: 6px;
}

.size-large {
  font-size: 13px;
  height: var(--lumina-control-height-lg);
}

.variant-primary {
  background: var(--lumina-primary);
  border-color: var(--lumina-primary);
  color: var(--lumina-on-accent);

  &:hover:not(:disabled) {
    background: var(--lumina-primary-hover);
    border-color: var(--lumina-primary-hover);
  }
}

.variant-danger {
  color: var(--lumina-danger);

  &:hover:not(:disabled) {
    border-color: color-mix(in srgb, var(--lumina-danger) 45%, var(--lumina-card-border));
  }
}

.variant-ghost {
  background: transparent;
  border-color: transparent;
  color: var(--lumina-text-secondary);
}
</style>
