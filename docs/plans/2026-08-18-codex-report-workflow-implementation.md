# Codex Report Workflow Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Refine the Codex Daily Report screen into a two-column workflow with a persistent prompt drawer, manual control over automatically excluded sessions, and Markdown preview.

**Architecture:** Keep the existing Tauri session reader and local work-record formatter. In the Vue view, distinguish automatic exclusion from the user's final inclusion selection, store the prompt locally, and render the editable Markdown through the lightweight `marked` package.

**Tech Stack:** Vue 3, TypeScript, Naive UI, Tauri, `marked`.

---

### Task 1: Add Markdown rendering support

**Files:**

- Modify: `package.json`
- Modify: `package-lock.json`
- Modify: `src/views/codex-report/CodexReportView.vue`

Install `marked`, render only the current work-record string in a read-only preview mode, and avoid introducing a rich-text editor.

### Task 2: Separate automatic exclusion from inclusion

**Files:**

- Modify: `src/views/codex-report/CodexReportView.vue`

Use `hasWorkContent` only to establish default included IDs after loading. Keep all loaded sessions available in the list, calculate included sessions directly from user-selected IDs, and provide All / Included / Excluded list filters so manually included sessions remain included.

### Task 3: Reshape the workbench

**Files:**

- Modify: `src/views/codex-report/CodexReportView.vue`

Replace the right prompt panel with a right-side drawer; add local prompt persistence, date shortcuts, compact session summary, a two-column main layout, renamed actions, and an explicit copy-for-AI success message.

### Task 4: Static verification

**Files:**

- Review: files above

Run `git diff --check` and inspect dependency and source diffs. Do not run project build, lint, tests, or browser automation unless explicitly requested.
