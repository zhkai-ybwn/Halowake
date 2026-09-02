import { computed, ref, watch } from 'vue'

type GitWorkspacePanel = 'changes' | 'diff' | 'commit'

interface GitWorkspaceLayout {
  visible: Record<GitWorkspacePanel, boolean>
  widths: {
    changes: number
    commit: number
  }
}

const GIT_WORKSPACE_LAYOUT_STORAGE_KEY = 'lumina.gitAssistant.workspaceLayout.v1'
const SPLIT_HANDLE_WIDTH = 7
const DIFF_MIN_WIDTH = 220
const PANEL_LIMITS = {
  changes: { min: 280, default: 340 },
  commit: { min: 300, default: 320 },
} as const
const DEFAULT_PANEL_LAYOUT: GitWorkspaceLayout = {
  visible: { changes: true, diff: true, commit: true },
  widths: { changes: PANEL_LIMITS.changes.default, commit: PANEL_LIMITS.commit.default },
}

export function useGitWorkspaceLayout(t: (key: string) => string) {
  const workspaceBody = ref<HTMLElement | null>(null)
  const workspaceWidth = ref(0)
  const panelLayout = ref<GitWorkspaceLayout>(loadPanelLayout())
  let workspaceResizeObserver: ResizeObserver | null = null

  const visiblePanelCount = computed(() => Object.values(panelLayout.value.visible).filter(Boolean).length)
  const storedCommitWidth = computed(() => Math.max(panelLayout.value.widths.commit, PANEL_LIMITS.commit.min))
  const changesMaxWidth = computed(() => {
    const visible = panelLayout.value.visible
    const reservedDiff = visible.diff ? DIFF_MIN_WIDTH : 0
    const reservedCommit = visible.commit
      ? (visible.diff ? storedCommitWidth.value : PANEL_LIMITS.commit.min)
      : 0
    const handles = Math.max(0, visiblePanelCount.value - 1) * SPLIT_HANDLE_WIDTH
    return Math.max(
      PANEL_LIMITS.changes.min,
      (workspaceWidth.value || 1280) - reservedDiff - reservedCommit - handles,
    )
  })
  const effectiveChangesWidth = computed(() => clamp(
    panelLayout.value.widths.changes,
    PANEL_LIMITS.changes.min,
    changesMaxWidth.value,
  ))
  const commitMaxWidth = computed(() => {
    const visible = panelLayout.value.visible
    const reservedDiff = visible.diff ? DIFF_MIN_WIDTH : 0
    const reservedChanges = visible.changes
      ? (visible.diff ? effectiveChangesWidth.value : PANEL_LIMITS.changes.min)
      : 0
    const handles = Math.max(0, visiblePanelCount.value - 1) * SPLIT_HANDLE_WIDTH
    return Math.max(
      PANEL_LIMITS.commit.min,
      (workspaceWidth.value || 1280) - reservedDiff - reservedChanges - handles,
    )
  })
  const effectiveCommitWidth = computed(() => clamp(
    panelLayout.value.widths.commit,
    PANEL_LIMITS.commit.min,
    commitMaxWidth.value,
  ))
  const leadingHandleControlsCommit = computed(() => (
    panelLayout.value.visible.changes
    && !panelLayout.value.visible.diff
    && panelLayout.value.visible.commit
  ))
  const leadingHandleLabel = computed(() => t(
    leadingHandleControlsCommit.value
      ? 'gitAssistant.layout.resizeCommit'
      : 'gitAssistant.layout.resizeChanges',
  ))
  const leadingHandleValue = computed(() => (
    leadingHandleControlsCommit.value ? effectiveCommitWidth.value : effectiveChangesWidth.value
  ))
  const leadingHandleMin = computed(() => (
    leadingHandleControlsCommit.value ? PANEL_LIMITS.commit.min : PANEL_LIMITS.changes.min
  ))
  const leadingHandleMax = computed(() => (
    leadingHandleControlsCommit.value ? commitMaxWidth.value : changesMaxWidth.value
  ))
  const workspaceGridStyle = computed(() => {
    const visible = panelLayout.value.visible
    const columns: string[] = []

    if (visible.changes) {
      columns.push(
        visiblePanelCount.value === 1
          ? 'minmax(0, 1fr)'
          : !visible.diff && visible.commit
            ? `minmax(${PANEL_LIMITS.changes.min}px, 1fr)`
            : `${effectiveChangesWidth.value}px`,
      )
      if (visible.diff || visible.commit) columns.push(`${SPLIT_HANDLE_WIDTH}px`)
    }
    if (visible.diff) {
      columns.push(visiblePanelCount.value === 1 ? 'minmax(0, 1fr)' : `minmax(${DIFF_MIN_WIDTH}px, 1fr)`)
      if (visible.commit) columns.push(`${SPLIT_HANDLE_WIDTH}px`)
    }
    if (visible.commit) {
      columns.push(
        visiblePanelCount.value === 1
          ? 'minmax(0, 1fr)'
          : `${effectiveCommitWidth.value}px`,
      )
    }

    return { gridTemplateColumns: columns.join(' ') }
  })

  function persistPanelLayout() {
    try {
      localStorage.setItem(GIT_WORKSPACE_LAYOUT_STORAGE_KEY, JSON.stringify(panelLayout.value))
    } catch {
      // Layout persistence is optional; the workbench remains usable without storage.
    }
  }

  function togglePanel(panel: GitWorkspacePanel) {
    const visible = panelLayout.value.visible
    if (visible[panel] && visiblePanelCount.value === 1) return
    panelLayout.value = {
      ...panelLayout.value,
      visible: { ...visible, [panel]: !visible[panel] },
    }
  }

  function resetPanelLayout() {
    panelLayout.value = cloneDefaultPanelLayout()
  }

  function resetPanelWidth(panel: 'changes' | 'commit') {
    panelLayout.value = {
      ...panelLayout.value,
      widths: { ...panelLayout.value.widths, [panel]: PANEL_LIMITS[panel].default },
    }
  }

  function resizeChangesPanel(delta: number) {
    panelLayout.value = {
      ...panelLayout.value,
      widths: {
        ...panelLayout.value.widths,
        changes: clamp(effectiveChangesWidth.value + delta, PANEL_LIMITS.changes.min, changesMaxWidth.value),
      },
    }
  }

  function resizeCommitPanel(delta: number) {
    panelLayout.value = {
      ...panelLayout.value,
      widths: {
        ...panelLayout.value.widths,
        commit: clamp(effectiveCommitWidth.value - delta, PANEL_LIMITS.commit.min, commitMaxWidth.value),
      },
    }
  }

  function resizeLeadingPanel(delta: number) {
    if (leadingHandleControlsCommit.value) {
      resizeCommitPanel(delta)
    } else {
      resizeChangesPanel(delta)
    }
  }

  function resetLeadingPanelWidth() {
    resetPanelWidth(leadingHandleControlsCommit.value ? 'commit' : 'changes')
  }

  function observeWorkspaceBody() {
    if (!workspaceBody.value) return
    workspaceWidth.value = workspaceBody.value.clientWidth
    workspaceResizeObserver = new ResizeObserver(entries => {
      workspaceWidth.value = entries[0]?.contentRect.width ?? workspaceBody.value?.clientWidth ?? 0
    })
    workspaceResizeObserver.observe(workspaceBody.value)
  }

  function disconnectWorkspaceObserver() {
    workspaceResizeObserver?.disconnect()
  }

  watch(panelLayout, persistPanelLayout, { deep: true })

  return {
    PANEL_LIMITS,
    workspaceBody,
    panelLayout,
    workspaceGridStyle,
    effectiveCommitWidth,
    commitMaxWidth,
    leadingHandleLabel,
    leadingHandleValue,
    leadingHandleMin,
    leadingHandleMax,
    togglePanel,
    resetPanelLayout,
    resetPanelWidth,
    resizeCommitPanel,
    resizeLeadingPanel,
    resetLeadingPanelWidth,
    observeWorkspaceBody,
    disconnectWorkspaceObserver,
  }
}

function clamp(value: number, min: number, max: number) {
  return Math.min(max, Math.max(min, value))
}

function cloneDefaultPanelLayout(): GitWorkspaceLayout {
  return {
    visible: { ...DEFAULT_PANEL_LAYOUT.visible },
    widths: { ...DEFAULT_PANEL_LAYOUT.widths },
  }
}

function normalizePanelWidth(value: unknown, fallback: number, min: number) {
  const width = Number(value)
  return Number.isFinite(width) ? Math.max(width, min) : fallback
}

function loadPanelLayout(): GitWorkspaceLayout {
  try {
    const stored = JSON.parse(localStorage.getItem(GIT_WORKSPACE_LAYOUT_STORAGE_KEY) || 'null') as Partial<GitWorkspaceLayout> | null
    if (!stored?.visible || !stored.widths) return cloneDefaultPanelLayout()
    const visible = {
      changes: stored.visible.changes !== false,
      diff: stored.visible.diff !== false,
      commit: stored.visible.commit !== false,
    }
    if (!Object.values(visible).some(Boolean)) visible.diff = true
    return {
      visible,
      widths: {
        changes: normalizePanelWidth(stored.widths.changes, PANEL_LIMITS.changes.default, PANEL_LIMITS.changes.min),
        commit: normalizePanelWidth(stored.widths.commit, PANEL_LIMITS.commit.default, PANEL_LIMITS.commit.min),
      },
    }
  } catch {
    return cloneDefaultPanelLayout()
  }
}
