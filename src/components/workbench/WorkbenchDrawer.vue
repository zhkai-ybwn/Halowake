<template>
  <aside class="workbench-drawer" :class="[`size-${size}`, { fixed }]">
    <header>
      <div>
        <h3>{{ title }}</h3>
        <p v-if="description">{{ description }}</p>
      </div>
      <WorkbenchCloseButton :label="closeLabel" @close="$emit('close')" />
    </header>
    <section class="workbench-drawer-body">
      <slot />
    </section>
  </aside>
</template>

<script setup lang="ts">
import WorkbenchCloseButton from '@/components/workbench/WorkbenchCloseButton.vue'

withDefaults(defineProps<{
  closeLabel?: string
  description?: string
  fixed?: boolean
  size?: 'narrow' | 'normal' | 'wide' | 'log'
  title: string
}>(), {
  description: '',
  fixed: false,
  size: 'normal',
})

defineEmits<{
  (e: 'close'): void
}>()
</script>

<style scoped lang="scss">
.workbench-drawer {
  background: color-mix(in srgb, var(--lumina-surface-elevated) 86%, transparent);
  border-left: 0.5px solid var(--lumina-separator-strong);
  backdrop-filter: saturate(180%) blur(24px);
  bottom: 0;
  box-shadow: var(--lumina-workbench-drawer-shadow);
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  position: absolute;
  right: 0;
  top: 0;
  width: min(520px, 92vw);
  z-index: 20;
  animation: workbench-drawer-in var(--lumina-duration-normal) var(--lumina-ease-out);

  &.fixed {
    position: fixed;
    top: 36px;
  }

  &.size-narrow {
    width: min(420px, 92vw);
  }

  &.size-wide {
    width: min(1120px, 72vw);
  }

  &.size-log {
    width: min(620px, 92vw);
  }

  > header {
    align-items: start;
    background: transparent;
    border-bottom: 0.5px solid var(--lumina-separator);
    display: flex;
    gap: 12px;
    justify-content: space-between;
    min-height: 54px;
    padding: 10px 12px;

    h3 {
      font-size: 15px;
      margin: 0 0 3px;
    }

    p {
      color: var(--lumina-text-secondary);
      font-size: 11px;
      line-height: 1.45;
      margin: 0;
    }
  }
}

@keyframes workbench-drawer-in {
  from {
    opacity: 0;
    transform: translateX(24px);
  }
  to {
    opacity: 1;
    transform: translateX(0);
  }
}

.workbench-drawer-body {
  min-height: 0;
  overflow: auto;
}
</style>
