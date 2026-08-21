<template>
  <kbd class="workbench-shortcut" :aria-label="keys">{{ keys }}</kbd>
</template>

<script setup lang="ts">
defineProps<{
  keys: string
}>()
</script>

<style scoped lang="scss">
.workbench-shortcut {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-control-bg) 84%, transparent);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-xs);
  color: currentcolor;
  display: inline-flex;
  font-family: var(--lumina-font-sans);
  font-size: 10px;
  font-weight: 500;
  height: 18px;
  justify-content: center;
  line-height: 1;
  min-width: 18px;
  opacity: 0.58;
  padding: 0 5px;
  white-space: nowrap;
}
</style>
