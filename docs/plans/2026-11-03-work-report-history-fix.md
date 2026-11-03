# Work Report History Fix Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Restore historical coverage for all four supported tools and keep generated reports consistent with active filters.

**Architecture:** Keep the existing Tauri query and session model. Add streaming JSONL/Zstandard reading, support OpenCode SQLite and legacy split storage, merge by provider/session ID before applying inclusion rules, and compare real instants rather than timestamp strings. Existing project-name selections remain compatible.

**Tech Stack:** Rust, zstd, rusqlite, chrono, Vue, TypeScript, Vitest.

---

### Task 1: History inputs and parsing
- Modify `src-tauri/Cargo.toml`, `src-tauri/src/commands/codex_report.rs`.
- Support compressed files, archived Codex records and configured Codex/Claude roots; remove mtime exclusion that can discard imported historical content.
- Correct Claude nested message text/session metadata; preserve Antigravity workspace attribution outside the queried period; normalize timestamp comparisons.
- Add hermetic fixtures for each regression and run `cargo test --manifest-path src-tauri/Cargo.toml commands::codex_report`.

### Task 2: OpenCode and session identity
- Create `src-tauri/src/commands/codex_report/opencode.rs`.
- Read SQLite session/message/part in read-only mode with bounded message queries; retain flat and split JSON storage support. Discover projects and tools through the same storage roots.
- Merge same-provider/session IDs and messages before frontend exclusion; keep providers distinct.
- Test SQLite, split legacy storage, date boundaries and resumed sessions.

### Task 3: Report state and dates
- Modify `src/views/codex-report/CodexReportView.vue`, `src/utils/codex-report.ts` and session list identity.
- Prevent stale loads overwriting newer date/provider selections, apply auto-exclusion immediately, regenerate only pristine generated text when filters change so manual edits survive.
- Show date range and full dates for multi-day output. Add unit/component regressions.
- Keep existing selected-project persistence and provide a true unbounded select-all state.

### Task 4: Verification
- Run targeted Rust/Vitest tests, TypeScript check and diff checks.
- Run the repaired synchronous loader against local July–September data and compare the four audited projects with the baseline.
- Document actual results and remaining limitations; no browser automation without user authorization.


## Completion

Implemented all four tasks in the current workspace without touching the pre-existing Git assistant changes. The repaired local-data loader reproduced 125 included sessions for the audited four projects. Backend regression tests, six frontend unit/component tests, TypeScript, changed-file ESLint and whitespace checks cover the fix. Browser verification remains manual per user instructions. No commit, packaging, application restart or deployment was performed.
