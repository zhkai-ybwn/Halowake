<template>
  <section class="workbench-topbar">
    <div class="workbench-topbar-main">
      <slot />
    </div>
    <div v-if="$slots.actions" class="workbench-topbar-actions">
      <slot name="actions" />
    </div>
  </section>
</template>

<style scoped lang="scss">
.workbench-topbar {
  align-items: center;
  background: var(--lumina-workbench-top-bg);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-lg);
  box-shadow: var(--lumina-shadow-sm);
  backdrop-filter: var(--lumina-vibrancy);
  display: flex;
  flex: 0 0 auto;
  gap: 8px;
  min-height: 42px;
  overflow: hidden;
  padding: 6px 8px;
}

.workbench-topbar-main,
.workbench-topbar-actions {
  align-items: center;
  display: flex;
  gap: 8px;
  min-width: 0;
}

.workbench-topbar-main {
  flex: 1;
  overflow: hidden;
}

.workbench-topbar-actions {
  flex: 0 0 auto;
}
</style>
