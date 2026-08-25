<template>
  <section class="workbench-modal-panel" :class="`size-${size}`">
    <button class="workbench-modal-close" type="button" :aria-label="closeLabel" @click="$emit('close')">
      <span class="close-glyph" aria-hidden="true">×</span>
    </button>
    <slot />
  </section>
</template>

<script setup lang="ts">
withDefaults(defineProps<{
  closeLabel: string
  size?: 'normal' | 'wide' | 'diff' | 'log'
}>(), {
  size: 'normal',
})

defineEmits<{
  (e: 'close'): void
}>()
</script>

<style scoped lang="scss">
.workbench-modal-panel {
  background: color-mix(in srgb, var(--lumina-surface-elevated) 94%, transparent);
  backdrop-filter: saturate(180%) blur(24px);
  border: 0.5px solid var(--lumina-separator-strong);
  border-radius: 12px;
  box-shadow: 0 0 0 0.5px color-mix(in srgb, var(--lumina-text) 8%, transparent), 0 2px 8px rgb(0 0 0 / 10%), 0 18px 52px rgb(0 0 0 / 18%);
  animation: workbench-modal-in var(--lumina-duration-normal) var(--lumina-ease-spring);
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  max-height: min(720px, calc(100vh - 96px));
  overflow: hidden;
  position: relative;
  width: min(760px, calc(100vw - 72px));

  &.size-wide {
    height: min(820px, calc(100vh - 72px));
    width: min(1560px, calc(100vw - 44px));
  }

  &.size-diff {
    display: block;
    height: min(820px, calc(100vh - 76px));
    width: min(1480px, calc(100vw - 72px));
  }

  &.size-log {
    grid-template-rows: minmax(0, 1fr);
    height: min(760px, calc(100vh - 76px));
    width: min(1180px, calc(100vw - 72px));
  }
}

@keyframes workbench-modal-in {
  from {
    opacity: 0;
    transform: scale(0.975) translateY(6px);
  }
  to {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
}

.workbench-modal-close {
  align-items: center;
  background: transparent;
  border: 0;
  border-radius: var(--lumina-radius-sm);
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: flex;
  height: 28px;
  justify-content: center;
  padding: 0;
  position: absolute;
  right: 12px;
  top: 12px;
  width: 28px;
  z-index: 20;

  &:hover {
    background: var(--lumina-button-secondary-hover);
    color: var(--lumina-text);
  }

  &:focus-visible {
    box-shadow: 0 0 0 3px var(--lumina-accent-ring);
    outline: none;
  }

  .close-glyph {
    font-size: 20px;
    font-weight: 300;
    line-height: 1;
    transform: translateY(-1px);
  }
}
</style>
