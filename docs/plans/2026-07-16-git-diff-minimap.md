# Git Diff Minimap Implementation Plan

**Goal:** Add a Lumina-styled, clickable change minimap to the standalone Git Diff window.

**Architecture:** Inspect each rendered `diff2html` line after every Diff load, classify it as normal, added, deleted, or modified, then collapse contiguous non-normal line states into minimap markers. Keep the code pane as the sole scroll container; render the viewport as two locator lines and support click-and-drag navigation.

**Tech Stack:** Vue 3, TypeScript, diff2html, SCSS.

---

### Task 1: Collect rendered change positions

**Files:**

- Modify: `src/views/git-diff/GitDiffView.vue`

1. Attach a ref to the code scroll container after `diff2html` has rendered.
2. Build a full line-state sequence and merge adjacent mixed add/delete runs into Lumina-primary modification markers.
3. Clear markers for empty or failed diffs.

### Task 2: Render and synchronize the minimap

**Files:**

- Modify: `src/views/git-diff/GitDiffView.vue`

1. Add a non-scrolling left minimap with normal, added, deleted, and modified colors.
2. Update its top and bottom locator lines on code-pane scrolling.
3. Map minimap clicks and drags back to the corresponding code line.

### Task 3: Verify

**Files:**

- Test: `src/views/git-diff/GitDiffView.vue`

1. Run `vue-tsc --noEmit` and targeted ESLint/Stylelint.
2. Run `git diff --check`.
3. Manually verify one changed file has a single code scrollbar, synchronized minimap markers, and click-to-jump behavior.
