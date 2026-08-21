<template>
  <section class="workbench-sheet" :class="[`size-${size}`, { busy }]" role="dialog" aria-modal="true">
    <header class="workbench-sheet__header">
      <span v-if="icon" class="workbench-sheet__icon" aria-hidden="true"><Icon :icon="icon" /></span>
      <div class="workbench-sheet__title">
        <h2>{{ title }}</h2>
        <p v-if="description">{{ description }}</p>
      </div>
      <button class="workbench-sheet__close" type="button" :aria-label="closeLabel" :disabled="busy" @click="$emit('close')">
        <span class="close-glyph" aria-hidden="true">×</span>
      </button>
    </header>
    <main class="workbench-sheet__body"><slot /></main>
    <footer v-if="$slots.footer" class="workbench-sheet__footer"><slot name="footer" /></footer>
  </section>
</template>

<script setup lang="ts">
withDefaults(defineProps<{
  title: string
  description?: string
  closeLabel: string
  icon?: string
  busy?: boolean
  size?: 'compact' | 'normal' | 'wide'
}>(), { description: '', icon: '', busy: false, size: 'normal' })
defineEmits<{ close: [] }>()
</script>

<style scoped lang="scss">
.workbench-sheet {
  animation: workbench-sheet-in var(--lumina-duration-normal) var(--lumina-ease-spring);
  background: color-mix(in srgb, var(--lumina-surface-elevated) 94%, transparent);
  backdrop-filter: saturate(180%) blur(24px);
  border: 0.5px solid var(--lumina-separator-strong);
  border-radius: 12px;
  box-shadow:
    0 0 0 0.5px color-mix(in srgb, var(--lumina-text) 8%, transparent),
    0 2px 8px color-mix(in srgb, #000 10%, transparent),
    0 18px 52px color-mix(in srgb, #000 18%, transparent);
  color: var(--lumina-text);
  display: grid;
  grid-template-rows: auto minmax(0, 1fr) auto;
  max-height: min(760px, calc(100vh - 64px));
  overflow: hidden;
  width: min(560px, calc(100vw - 48px));

  &.size-compact { width: min(420px, calc(100vw - 48px)); }
  &.size-wide { height: min(780px, calc(100vh - 56px)); width: min(980px, calc(100vw - 48px)); }
}

.workbench-sheet__header {
  align-items: center;
  border-bottom: 0.5px solid var(--lumina-separator);
  display: grid;
  gap: 10px;
  grid-template-columns: auto minmax(0, 1fr) auto;
  min-height: 58px;
  padding: 10px 12px 10px 16px;
}

.workbench-sheet__icon {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-primary) 11%, var(--lumina-surface-2));
  border-radius: 8px;
  color: var(--lumina-primary);
  display: inline-flex;
  flex: 0 0 28px;
  height: 32px;
  justify-content: center;
  width: 32px;
  svg { height: 17px; width: 17px; }
}

.workbench-sheet__title {
  min-width: 0;
  h2 { font-size: 15px; font-weight: 600; letter-spacing: -0.01em; margin: 0; }
  p { color: var(--lumina-text-secondary); font-size: 11px; line-height: 1.4; margin: 2px 0 0; }
}

.workbench-sheet__close {
  align-items: center;
  background: transparent;
  border: 0;
  border-radius: 6px;
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: inline-flex;
  height: 28px;
  justify-content: center;
  padding: 0;
  transition: background var(--lumina-duration-fast) var(--lumina-ease-out), color var(--lumina-duration-fast) var(--lumina-ease-out);
  width: 28px;
  &:hover:not(:disabled) { background: var(--lumina-button-secondary-hover); color: var(--lumina-text); }
  &:focus-visible { box-shadow: 0 0 0 3px var(--lumina-accent-ring); outline: none; }
  &:disabled { cursor: default; opacity: .4; }
  .close-glyph { font-size: 20px; font-weight: 300; line-height: 1; transform: translateY(-1px); }
}

.workbench-sheet__body { display: grid; gap: 12px; min-height: 0; overflow: auto; padding: 16px; }
.workbench-sheet__footer { align-items: center; background: color-mix(in srgb, var(--lumina-surface-2) 70%, transparent); border-top: 0.5px solid var(--lumina-separator); display: flex; gap: 8px; justify-content: flex-end; min-height: 52px; padding: 8px 12px; }

@keyframes workbench-sheet-in {
  from { opacity: 0; transform: translateY(8px) scale(.985); }
  to { opacity: 1; transform: translateY(0) scale(1); }
}
</style>
