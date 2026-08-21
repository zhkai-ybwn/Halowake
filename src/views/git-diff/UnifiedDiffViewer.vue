<template>
  <section class="unified-diff-viewer">
    <aside
      class="locator-bar"
      aria-label="Diff locator"
      @pointerdown="startLocatorDrag"
      @pointermove="dragLocator"
      @pointerup="stopLocatorDrag"
      @pointercancel="stopLocatorDrag"
    >
      <div class="locator-bar__track">
        <div v-for="lane in locatorLanes" :key="lane.key" class="locator-bar__lane">
          <i
            v-for="(marker, index) in lane.markers"
            :key="`${marker.top}-${index}`"
            class="locator-bar__marker"
            :class="`locator-bar__marker--${marker.kind}`"
            :style="{ top: `${marker.top}%`, height: `${marker.height}%` }"
          ></i>
        </div>
        <i
          class="locator-bar__viewport"
          :style="{ top: `${locatorViewport.top}%`, height: `${locatorViewport.height}%` }"
        ></i>
        <i class="locator-bar__edge" :style="{ top: `${locatorViewport.top}%` }"></i>
        <i class="locator-bar__edge" :style="{ top: `${locatorViewport.bottom}%` }"></i>
      </div>
    </aside>

    <section ref="codePane" class="code-pane" @scroll="syncViewport">
      <template v-for="hunk in hunks" :key="hunk.id">
        <div
          v-for="line in hunk.lines"
          :key="line.id"
          class="diff-line"
          :class="`diff-line--${line.kind}`"
          :data-locator-index="locatorIndex(line.id)"
        >
          <template v-if="line.kind === 'note'">
            <span class="diff-line__note">{{ line.text }}</span>
          </template>
          <template v-else>
            <span class="diff-line__sign">{{ lineSign(line.kind) }}</span>
            <span class="diff-line__number">{{ displayLineNumber(line) }}</span>
            <code class="diff-line__content">{{ line.text }}</code>
          </template>
        </div>
      </template>
    </section>
  </section>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { parseUnifiedDiff, type UnifiedDiffLine, type UnifiedDiffLineKind } from './unified-diff'

const props = defineProps<{ diff: string }>()

type LocatorKind = 'added' | 'deleted'

interface LocatorMarker {
  top: number
  height: number
  kind: LocatorKind
}

const codePane = ref<HTMLElement | null>(null)
const viewport = ref({ top: 0, bottom: 100 })
let locatorDragging = false

const hunks = computed(() => parseUnifiedDiff(props.diff))
const codeLines = computed(() => hunks.value.flatMap(hunk => hunk.lines.filter(line => line.kind !== 'hunk' && line.kind !== 'note')))
const lineIndexes = computed(() => new Map(codeLines.value.map((line, index) => [line.id, index])))
function buildLocatorMarkers(): LocatorMarker[] {
  const lines = codeLines.value
  const groups: Array<{ start: number; end: number; kind: LocatorKind }> = []

  for (let index = 0; index < lines.length;) {
    const kind = lines[index].kind
    if (kind !== 'added' && kind !== 'deleted') {
      index++
      continue
    }
    const start = index
    while (index < lines.length && lines[index].kind === kind) index++
    groups.push({ start, end: index, kind })
  }

  const total = Math.max(lines.length, 1)
  return groups.map(group => ({
    top: (group.start / total) * 100,
    height: Math.max(((group.end - group.start) / total) * 100, 0.4),
    kind: group.kind,
  }))
}

const locatorLanes = computed(() => [
  { key: 'source', markers: buildLocatorMarkers() },
  { key: 'result', markers: [] as LocatorMarker[] },
  { key: 'target', markers: [] as LocatorMarker[] },
])

const locatorViewport = computed(() => {
  const sourceHeight = viewport.value.bottom - viewport.value.top
  const height = Math.min(Math.max(sourceHeight, 4), 14)
  const scrollableSourceHeight = Math.max(100 - sourceHeight, 1)
  const progress = Math.min(viewport.value.top / scrollableSourceHeight, 1)
  const top = progress * (100 - height)
  return { top, height, bottom: top + height }
})

function locatorIndex(id: string) {
  return lineIndexes.value.get(id) ?? ''
}

function lineSign(kind: UnifiedDiffLineKind) {
  if (kind === 'added') return '+'
  if (kind === 'deleted') return '−'
  return ''
}

function displayLineNumber(line: UnifiedDiffLine) {
  return line.newLine ?? line.oldLine ?? ''
}

function syncViewport() {
  const pane = codePane.value
  const total = codeLines.value.length
  if (!pane || total === 0) {
    viewport.value = { top: 0, bottom: 100 }
    return
  }

  const paneRect = pane.getBoundingClientRect()
  const rows = [...pane.querySelectorAll<HTMLElement>('[data-locator-index]')]
  const topIndex = rows.findIndex(row => row.getBoundingClientRect().bottom > paneRect.top)
  const bottomIndex = rows.findLastIndex(row => row.getBoundingClientRect().top < paneRect.bottom)
  const start = Math.max(topIndex, 0)
  const end = Math.max(bottomIndex + 1, start + 1)
  viewport.value = { top: (start / total) * 100, bottom: Math.min((end / total) * 100, 100) }
}

function scrollFromLocator(event: PointerEvent) {
  const pane = codePane.value
  const target = event.currentTarget as HTMLElement
  if (!pane) return
  const bounds = target.getBoundingClientRect()
  const ratio = Math.min(Math.max((event.clientY - bounds.top) / bounds.height, 0), 1)
  pane.scrollTop = ratio * Math.max(pane.scrollHeight - pane.clientHeight, 0)
}

function startLocatorDrag(event: PointerEvent) {
  locatorDragging = true
  ;(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId)
  scrollFromLocator(event)
}

function dragLocator(event: PointerEvent) {
  if (locatorDragging) scrollFromLocator(event)
}

function stopLocatorDrag(event: PointerEvent) {
  locatorDragging = false
  const target = event.currentTarget as HTMLElement
  if (target.hasPointerCapture(event.pointerId)) target.releasePointerCapture(event.pointerId)
}

function refreshViewport() {
  nextTick(() => requestAnimationFrame(syncViewport))
}

watch(() => props.diff, refreshViewport, { immediate: true })
onMounted(() => window.addEventListener('resize', refreshViewport))
onUnmounted(() => window.removeEventListener('resize', refreshViewport))
</script>

<style scoped lang="scss">
.unified-diff-viewer {
  --diff-added-color: color-mix(in srgb, var(--lumina-success) 18%, var(--lumina-surface-1));
  --diff-deleted-color: color-mix(in srgb, var(--lumina-danger) 18%, var(--lumina-surface-1));

  display: grid;
  grid-template-columns: 28px minmax(0, 1fr);
  height: 100%;
  min-height: 0;
}

.locator-bar {
  background: color-mix(in srgb, var(--lumina-surface-2) 92%, var(--lumina-text-secondary));
  border-right: 0.5px solid var(--lumina-separator);
  cursor: pointer;
  padding: 0;
  user-select: none;
}

.locator-bar__track {
  background: color-mix(in srgb, var(--lumina-text-secondary) 6%, var(--lumina-surface-1));
  border-left: 0.5px solid color-mix(in srgb, var(--lumina-text-secondary) 24%, transparent);
  border-right: 0.5px solid color-mix(in srgb, var(--lumina-text-secondary) 24%, transparent);
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  height: 100%;
  overflow: hidden;
  position: relative;
}

.locator-bar__lane {
  border-right: 0.5px solid color-mix(in srgb, var(--lumina-text-secondary) 18%, transparent);
  position: relative;
}

.locator-bar__lane:last-of-type {
  border-right: 0;
}

.locator-bar__marker,
.locator-bar__edge,
.locator-bar__viewport {
  display: block;
  left: 0;
  pointer-events: none;
  position: absolute;
  right: 0;
}

.locator-bar__marker {
  min-height: 3px;
}
.locator-bar__marker--added { background: var(--diff-added-color); }
.locator-bar__marker--deleted { background: var(--diff-deleted-color); }

.locator-bar__edge {
  background: color-mix(in srgb, var(--lumina-text-secondary) 56%, transparent);
  height: 1px;
  z-index: 3;
}

.locator-bar__viewport {
  background: color-mix(in srgb, var(--lumina-text-secondary) 26%, transparent);
  border: 0.5px solid color-mix(in srgb, var(--lumina-text-secondary) 44%, transparent);
  min-height: 8px;
  z-index: 2;
}

.code-pane {
  background: var(--lumina-surface-1);
  min-height: 0;
  overflow: auto;
  scrollbar-color: color-mix(in srgb, var(--lumina-primary) 28%, transparent) transparent;
}

.code-pane::-webkit-scrollbar {
  height: 10px;
  width: 10px;
}

.code-pane::-webkit-scrollbar-thumb {
  background: color-mix(in srgb, var(--lumina-primary) 28%, transparent);
  background-clip: padding-box;
  border: 2px solid transparent;
  border-radius: 999px;
}

.code-pane::-webkit-scrollbar-thumb:hover {
  background: color-mix(in srgb, var(--lumina-primary) 42%, transparent);
}

.diff-line {
  display: grid;
  grid-template-columns: 28px 60px minmax(max-content, 1fr);
  min-height: 20px;
  min-width: max-content;
}

.diff-line--normal { background: var(--lumina-surface-1); }
.diff-line--added { background: var(--diff-added-color); }
.diff-line--deleted { background: var(--diff-deleted-color); }
.diff-line--note { background: color-mix(in srgb, var(--lumina-text-secondary) 8%, var(--lumina-surface-1)); }

.diff-line__sign,
.diff-line__number,
.diff-line__content,
.diff-line__note {
  font: 12px/20px SFMono-Regular, Consolas, 'Liberation Mono', Menlo, monospace;
}

.diff-line__sign {
  color: var(--lumina-text-secondary);
  font-size: 15px;
  font-weight: 700;
  text-align: center;
}

.diff-line--added .diff-line__sign { color: var(--lumina-success); }
.diff-line--deleted .diff-line__sign { color: var(--lumina-danger); }

.diff-line__number {
  background: color-mix(in srgb, var(--lumina-surface-2) 90%, var(--lumina-text-secondary));
  border-right: 0.5px solid var(--lumina-separator);
  color: var(--lumina-text-secondary);
  font-variant-numeric: tabular-nums;
  padding-right: 8px;
  text-align: right;
}

.diff-line__content {
  color: var(--lumina-text);
  padding: 0 12px;
  white-space: pre;
}

.diff-line__note {
  color: var(--lumina-text-secondary);
  grid-column: 1 / -1;
  padding: 0 12px;
}
</style>
