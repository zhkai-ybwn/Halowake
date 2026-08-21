# Git Merge Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Merge a selected local or remote branch into the current branch, with a safe conflict recovery path.

**Architecture:** The Vue dialog selects a source branch and merge mode, then invokes one Tauri command. The Rust runner checks repository state, performs Git's normal merge or `--no-ff`, and leaves conflicted merges for the existing resolve/continue/abort controls.

**Tech Stack:** Vue 3, Naive UI, Tauri 2, Rust, Git CLI.

---

### Task 1: Add the merge command

**Files:**

- Modify: `src-tauri/src/git/models.rs`
- Modify: `src-tauri/src/git/runner.rs`
- Modify: `src-tauri/src/commands/git.rs`
- Modify: `src-tauri/src/lib.rs`

**Steps:** Add a typed payload, reject an active merge/rebase or dirty worktree, execute `git merge --no-edit` (optionally `--no-ff`), and return conflict guidance when Git leaves unmerged files.

### Task 2: Verify Git behavior

**Files:**

- Test: `src-tauri/src/git/runner.rs`

**Steps:** Create a temporary repository, merge a feature branch, assert its content is present, and assert a dirty working tree is rejected before merge.

### Task 3: Add the merge entry point

**Files:**

- Modify: `src/services/git/git-service.ts`
- Modify: `src/views/git-assistant/components/GitStatusBar.vue`
- Modify: `src/views/git-assistant/GitAssistantView.vue`
- Modify: `src/i18n/messages/zh-CN.ts`
- Modify: `src/i18n/messages/en-US.ts`

**Steps:** Add an `合并` top-bar action and compact modal that chooses a source branch and normal or no-fast-forward merge. Refresh the snapshot after a completed or conflicted command.

### Task 4: Verify

**Steps:** Run the focused Rust merge tests, `vue-tsc --noEmit`, and `git diff --check`.
