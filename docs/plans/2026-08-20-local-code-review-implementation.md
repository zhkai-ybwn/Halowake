# Local Code Review Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use executing-plans to implement this plan task-by-task.

**Goal:** Implement an end-to-end local Code Review flow that ranks files without AI, reviews only user-selected files with a bounded prompt, validates structured findings, and persists sessions, rules, findings, and usage in SQLite.

**Architecture:** Tauri owns a SQLite database and an in-memory review task registry. Rust builds immutable selected-file review contexts, applies deterministic and semantic rules, calls the configured AI provider, validates structured JSON, and persists a normalized report. Vue uses a Pinia store and Tauri events to render task progress and reports independently of route lifecycle.

**Tech Stack:** Rust, Tauri 2, rusqlite, Tokio, Serde, Vue 3, Pinia, TypeScript, Naive UI.

---

### Task 1: SQLite foundation

**Files:**

- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/storage/mod.rs`
- Create: `src-tauri/src/storage/database.rs`
- Create: `src-tauri/src/storage/migrations.rs`
- Modify: `src-tauri/src/lib.rs`

**Steps:**

1. Add `rusqlite` with bundled SQLite and UUID support dependencies.
2. Create `AppDatabase` managed state at `app_data_dir/lumina.db`.
3. Enable foreign keys, WAL, busy timeout, and run versioned migrations.
4. Create review sessions, files, findings, rules, and AI call tables with indexes.
5. Add focused migration/database unit tests without running Build or Lint automatically.

### Task 2: Review domain and repository

**Files:**

- Create: `src-tauri/src/review/mod.rs`
- Create: `src-tauri/src/review/models.rs`
- Create: `src-tauri/src/review/repository.rs`
- Modify: `src-tauri/src/lib.rs`

**Steps:**

1. Define review session, file, finding, rule, usage, and report models.
2. Implement transactional create/update/read/list/cancel/rule CRUD operations.
3. Mark abandoned running sessions interrupted on app startup.
4. Store rule snapshots and usage without prompts, API keys, or raw source.

### Task 3: Planner and structured AI review

**Files:**

- Create: `src-tauri/src/review/planner.rs`
- Create: `src-tauri/src/review/prompt.rs`
- Create: `src-tauri/src/review/validator.rs`
- Create: `src-tauri/src/review/runner.rs`
- Modify: `src-tauri/src/git/prompt.rs`
- Modify: `src-tauri/src/git/models.rs`
- Modify: `src-tauri/src/commands/git.rs`

**Steps:**

1. Extend Attention results with explainable score breakdown.
2. Load diffs only for selected paths and group related paths into bounded batches.
3. Merge built-in, global SQLite, and project semantic/deterministic rules by id and scope.
4. Execute zero-token deterministic rules before the AI call.
5. Build a compact structured prompt and parse strict review JSON.
6. Validate selected paths, diff line ranges, evidence, confidence, and deduplicate findings.
7. Persist partial success, limitations, estimated/provider usage, and final report.

### Task 4: Tauri task commands

**Files:**

- Create: `src-tauri/src/commands/review.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

**Steps:**

1. Add start/get/list/cancel review commands.
2. Add finding status and global rule CRUD commands.
3. Keep abort handles in a managed task registry.
4. Emit revisioned `local-code-review-updated` events.

### Task 5: Frontend domain and report UI

**Files:**

- Create: `src/types/review.ts`
- Create: `src/services/review-service.ts`
- Create: `src/stores/review.ts`
- Create: `src/views/git-assistant/components/GitReviewPanel.vue`
- Create: `src/views/settings/ReviewRulesView.vue`
- Modify: `src/views/git-assistant/GitAssistantView.vue`
- Modify: `src/router/index.ts`
- Modify: `src/components/settings/Settings.vue`
- Modify: `src/i18n/messages/zh-CN.ts`
- Modify: `src/i18n/messages/en-US.ts`

**Steps:**

1. Mirror Rust report, finding, rule, usage, and progress types.
2. Add service methods and a Pinia store that refreshes from SQLite-backed commands.
3. Start Review from the current manually selected files and selected `light-review` model.
4. Render structured summary, severity filters, file list, findings, limitations, and token details.
5. Add global deterministic/semantic rule management with validation.
6. Restore running/completed sessions when returning to the Git route.

### Task 6: Storage lifecycle integration

**Files:**

- Modify: `src-tauri/src/commands/storage.rs`
- Modify: `src/types/storage.ts`
- Modify: `src/services/storage-service.ts`

**Steps:**

1. Count SQLite main, WAL, and SHM files.
2. Delete expired unpinned review sessions transactionally.
3. Report deleted database records separately from reclaimed file bytes.

### Task 7: Verification

**Steps:**

1. Run `git diff --check` and targeted source searches.
2. Inspect all registered commands and frontend invoke names for exact agreement.
3. Do not run Build, Lint, browser automation, or visual tests unless the user explicitly requests them.
4. Provide suggested Rust tests, frontend typecheck, and manual route-switch scenarios for the user to run.
