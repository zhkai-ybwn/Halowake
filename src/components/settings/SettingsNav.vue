<template>
  <nav class="settings-nav">
    <section v-for="section in sections" :key="section.key" class="settings-nav__section">
      <div class="settings-nav__section-title">{{ section.label }}</div>
      <button
        v-for="item in section.items"
        :key="item.key"
        class="settings-nav__item"
        :class="{ active: item.key === modelValue }"
        type="button"
        @click="$emit('update:modelValue', item.key)"
      >
        <Icon :icon="item.icon" />
        <span>{{ item.label }}</span>
      </button>
    </section>
  </nav>
</template>

<script setup lang="ts">
defineProps<{
  modelValue: string
  sections: Array<{
    key: string
    label: string
    items: Array<{ key: string; label: string; icon: string }>
  }>
}>()

defineEmits<{
  (e: 'update:modelValue', value: string): void
}>()
</script>

<style scoped lang="scss">
.settings-nav {
  background: var(--lumina-sidebar-bg);
  backdrop-filter: var(--lumina-vibrancy);
  border-right: 1px solid var(--lumina-separator);
  display: flex;
  flex-direction: column;
  gap: 18px;
  padding: 16px 8px;
  width: 214px;
}

.settings-nav__section {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.settings-nav__section-title {
  color: var(--lumina-text-secondary);
  font-size: 11px;
  font-weight: 600;
  padding: 0 12px 4px;
}

.settings-nav__item {
  align-items: center;
  background: transparent;
  border: 0;
  border-radius: var(--lumina-radius-sm);
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: flex;
  gap: 10px;
  min-height: 30px;
  padding: 0 10px;
  text-align: left;
  transition: background var(--lumina-motion-fast), color var(--lumina-motion-fast);

  &:hover {
    background: var(--lumina-button-secondary-hover);
    color: var(--lumina-text);
  }

  &.active {
    background: var(--lumina-control-active);
    color: var(--lumina-text);
  }

  svg {
    flex: 0 0 auto;
    height: 16px;
    width: 16px;
  }

  span {
    font-size: 13px;
  }
}
</style>
