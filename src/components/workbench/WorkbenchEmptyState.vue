<template>
  <section class="workbench-empty-state">
    <span class="workbench-empty-icon" aria-hidden="true"><Icon :icon="icon" /></span>
    <h3>{{ title }}</h3>
    <p v-if="description">{{ description }}</p>
    <div v-if="$slots.actions" class="workbench-empty-actions"><slot name="actions" /></div>
  </section>
</template>

<script setup lang="ts">
withDefaults(defineProps<{
  description?: string
  icon?: string
  title: string
}>(), {
  description: '',
  icon: 'solar:inbox-line-linear',
})
</script>

<style scoped lang="scss">
.workbench-empty-state {
  align-items: center;
  align-self: stretch;
  color: var(--lumina-text-secondary);
  display: flex;
  flex: 1;
  flex-direction: column;
  justify-content: center;
  min-height: 180px;
  padding: 32px;
  text-align: center;
}

.workbench-empty-icon {
  align-items: center;
  background: var(--lumina-control-bg);
  border-radius: var(--lumina-radius-lg);
  color: var(--lumina-text-tertiary);
  display: flex;
  height: 44px;
  justify-content: center;
  margin-bottom: 14px;
  width: 44px;

  svg {
    height: 22px;
    width: 22px;
  }
}

h3 {
  color: var(--lumina-text);
  font-size: 15px;
  font-weight: 600;
  margin: 0;
}

p {
  font-size: 12px;
  line-height: 1.5;
  margin: 5px 0 0;
  max-width: 360px;
}

.workbench-empty-actions {
  display: flex;
  gap: 8px;
  margin-top: 16px;
}
</style>
