# Unified Git Diff Window Implementation Plan

**Goal:** Show both working-tree and historical file diffs in one reusable standalone Tauri window with formatted line-by-line output.

**Architecture:** Add a `git-diff` webview route and a small window service modelled on the existing Git Log window. Pass a discriminated diff request over Tauri events; the window calls the existing Git commands itself, then renders the returned unified diff with `diff2html`.

**Tech Stack:** Vue 3, TypeScript, Tauri 2 WebviewWindow and events, diff2html.

---

### Task 1: Create the reusable Diff window

**Files:**

- Create: `src/services/git/git-diff-window.ts`
- Create: `src/views/git-diff/GitDiffView.vue`
- Modify: `src/router/index.ts`

1. Define working-tree and historical request types, create/focus one `git-diff` window, and re-send its latest request when the window reloads.
2. Add the `/diff` route and a dedicated view that listens for the request, loads the appropriate existing Git diff service, and clears stale output while loading.
3. Render all returned unified diffs with `diff2html` using the existing line-by-line configuration.

### Task 2: Route both entry points through it

**Files:**

- Modify: `src/views/git-assistant/GitAssistantView.vue`
- Modify: `src/views/git-log/GitLogView.vue`

1. Make the change explorer's double-click open the standalone window with its current staged/unstaged/HEAD comparison mode.
2. Replace the history-page file Diff modal and raw `<pre>` renderer with a call to the same window service.
3. Keep the main-page single-click preview unchanged.

### Task 3: Verify the integration

**Files:**

- Test: `src-tauri/src/git/runner.rs` (existing regression suite)

1. Run `npm run lint` to type-check and lint all touched Vue/TypeScript files.
2. Run `cargo test --manifest-path src-tauri/Cargo.toml --lib` to confirm existing Git command regressions remain green.
3. Manually verify in the desktop app: opening a working-tree file and a history file reuses one Diff window and renders line numbers, hunks, additions, and deletions.
