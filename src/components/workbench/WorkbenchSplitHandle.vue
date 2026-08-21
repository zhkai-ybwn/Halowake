<template>
  <div
    class="split-handle"
    :class="{ 'is-dragging': dragging }"
    role="separator"
    aria-orientation="vertical"
    :aria-label="label"
    :aria-valuemin="min"
    :aria-valuemax="max"
    :aria-valuenow="Math.round(value)"
    tabindex="0"
    @dblclick="$emit('reset')"
    @keydown="handleKeydown"
    @pointerdown="startDrag"
  >
    <span aria-hidden="true"></span>
  </div>
</template>

<script setup lang="ts">
import { onUnmounted, ref } from 'vue'

defineProps<{
  label: string
  value: number
  min: number
  max: number
}>()

const emit = defineEmits<{
  (e: 'resize', delta: number): void
  (e: 'reset'): void
}>()

const dragging = ref(false)
let previousX = 0

function startDrag(event: PointerEvent) {
  if (event.button !== 0) return
  event.preventDefault()
  previousX = event.clientX
  dragging.value = true
  document.body.classList.add('is-resizing-workbench')
  window.addEventListener('pointermove', handlePointerMove)
  window.addEventListener('pointerup', stopDrag)
  window.addEventListener('pointercancel', stopDrag)
  window.addEventListener('blur', stopDrag)
}

function handlePointerMove(event: PointerEvent) {
  const delta = event.clientX - previousX
  if (!delta) return
  previousX = event.clientX
  emit('resize', delta)
}

function stopDrag() {
  dragging.value = false
  document.body.classList.remove('is-resizing-workbench')
  window.removeEventListener('pointermove', handlePointerMove)
  window.removeEventListener('pointerup', stopDrag)
  window.removeEventListener('pointercancel', stopDrag)
  window.removeEventListener('blur', stopDrag)
}

function handleKeydown(event: KeyboardEvent) {
  if (event.key === 'ArrowLeft') {
    event.preventDefault()
    emit('resize', event.shiftKey ? -32 : -12)
  } else if (event.key === 'ArrowRight') {
    event.preventDefault()
    emit('resize', event.shiftKey ? 32 : 12)
  } else if (event.key === 'Home') {
    event.preventDefault()
    emit('reset')
  }
}

onUnmounted(stopDrag)
</script>

<style scoped lang="scss">
.split-handle {
  cursor: col-resize;
  display: flex;
  justify-content: center;
  min-height: 0;
  outline: none;
  position: relative;
  touch-action: none;
  width: 7px;
  z-index: 4;

  span {
    background: var(--lumina-separator);
    height: 100%;
    transition: background var(--lumina-motion-fast), width var(--lumina-motion-fast);
    width: 1px;
  }

  &:hover span,
  &:focus-visible span,
  &.is-dragging span {
    background: var(--lumina-primary);
    width: 2px;
  }

  &:focus-visible {
    box-shadow: inset 0 0 0 2px var(--lumina-accent-ring);
  }
}

:global(body.is-resizing-workbench) {
  cursor: col-resize !important;
  user-select: none !important;
}
</style>
