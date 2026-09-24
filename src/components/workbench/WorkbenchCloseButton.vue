<template>
  <button
    class="workbench-close-button"
    :class="[`size-${size}`, { disabled }]"
    type="button"
    :aria-label="computedLabel"
    :title="title || computedLabel"
    :disabled="disabled"
    @click="handleClick"
  >
    <span class="close-glyph" aria-hidden="true">×</span>
  </button>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

const props = withDefaults(
  defineProps<{
    disabled?: boolean
    label?: string
    size?: 'sm' | 'md'
    title?: string
  }>(),
  {
    disabled: false,
    label: '',
    size: 'md',
    title: '',
  },
)

const emit = defineEmits<{
  (e: 'click', event: MouseEvent): void
  (e: 'close', event: MouseEvent): void
}>()

const { t } = useI18n({ useScope: 'global' })
const computedLabel = computed(() => props.label || t('common.close', '关闭'))

function handleClick(event: MouseEvent) {
  if (props.disabled) return
  emit('click', event)
  emit('close', event)
}
</script>

<style scoped lang="scss">
.workbench-close-button {
  align-items: center;
  background: transparent;
  border: 0;
  border-radius: var(--lumina-radius-sm);
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: inline-flex;
  flex: 0 0 var(--lumina-control-height);
  height: var(--lumina-control-height);
  justify-content: center;
  padding: 0;
  transition:
    background var(--lumina-duration-fast) var(--lumina-ease-out),
    color var(--lumina-duration-fast) var(--lumina-ease-out),
    transform var(--lumina-duration-fast) var(--lumina-ease-out);
  width: var(--lumina-control-height);

  &:hover:not(:disabled) {
    background: var(--lumina-button-secondary-hover);
    color: var(--lumina-text);
  }

  &:active:not(:disabled) {
    transform: scale(0.94);
  }

  &:focus-visible {
    box-shadow: 0 0 0 3px var(--lumina-accent-ring);
    outline: none;
  }

  &:disabled {
    cursor: not-allowed;
    opacity: 0.4;
  }

  &.size-sm {
    flex: 0 0 24px;
    height: 24px;
    width: 24px;

    .close-glyph {
      font-size: 18px;
    }
  }

  .close-glyph {
    font-family: var(--lumina-font-sans);
    font-size: 20px;
    font-weight: 300;
    line-height: 1;
    transform: translateY(-1px);
    user-select: none;
  }
}
</style>
