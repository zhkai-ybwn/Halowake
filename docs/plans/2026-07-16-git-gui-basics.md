# Git GUI Basics Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make Lumina usable for everyday repository setup, branch management, and file-level staging.

**Architecture:** Add small Tauri commands around native Git CLI operations. Keep the existing Git Assistant as the only UI surface: repository actions in the top bar, branch controls beside the current branch, and staging actions in the change explorer.

**Tech Stack:** Tauri/Rust, Vue 3, Naive UI, native Git CLI.

---

### Task 1: Repository Git commands

**Files:**
- Modify: `src-tauri/src/git/models.rs`, `src-tauri/src/git/runner.rs`, `src-tauri/src/commands/git.rs`, `src-tauri/src/lib.rs`
- Modify: `src/services/git/git-service.ts`

1. Add payloads and response models for stage/unstage, branches, init, and clone.
2. Add native Git runner operations with path arguments separated by `--`, current-branch deletion protection, and clone destination validation delegated to Git.
3. Register commands and expose typed frontend service functions.
4. Verify with focused Rust tests for stage/unstage and branch mutation.

### Task 2: Git Assistant controls

**Files:**
- Modify: `src/views/git-assistant/GitAssistantView.vue`, `src/views/git-assistant/components/GitStatusBar.vue`, `src/views/git-assistant/components/GitChangeExplorer.vue`
- Modify: `src/i18n/messages/en-US.ts`, `src/i18n/messages/zh-CN.ts`

1. Add init/clone and branch controls in the existing workbench UI.
2. Add stage/unstage all and context-menu actions for individual files.
3. Refresh the existing snapshot after successful mutations.
4. Verify with TypeScript, lint, stylelint, and Rust tests.

### Task 3: Branch reference management

**Files:**
- Modify: `src-tauri/src/git/models.rs`, `src-tauri/src/git/runner.rs`, `src-tauri/src/commands/git.rs`, `src-tauri/src/lib.rs`
- Modify: `src/services/git/git-service.ts`, `src/views/git-assistant/GitAssistantView.vue`

1. Load local heads and remote tracking refs as distinct branch types, including current and upstream state.
2. Let local refs switch directly; let remote refs create and switch to a local tracking branch.
3. Expose set/drop upstream controls for local branches.
4. Verify with focused Rust tests for local and remote checkout semantics plus frontend static checks.
