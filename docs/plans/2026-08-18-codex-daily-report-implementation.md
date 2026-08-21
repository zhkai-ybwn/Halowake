# Codex Daily Report Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a local Lumina workbench that reads Codex sessions by time range and produces editable factual work records plus an independent, editable prompt for a web-based AI.

**Architecture:** A Tauri command walks `%USERPROFILE%/.codex/sessions`, parses only the stable local JSONL event shapes, and returns session metadata plus user and assistant text. The Vue view applies explicit inclusion, project, and keyword filters; it renders a factual Markdown draft locally, without any model request, and provides separate copy actions for the draft and prompt.

**Tech Stack:** Tauri 2, Rust with `serde_json`, Vue 3 Composition API, TypeScript, Naive UI, vue-i18n.

---

### Task 1: Define and load local Codex sessions

**Files:**

- Create: `src-tauri/src/commands/codex_report.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

**Step 1: Add serializable request and response models**

Declare a `CodexSessionQuery` with `from`, `to`, and optional keyword, and response models containing the session ID, timestamps, CWD/project hint, user text, assistant text, and a computed `has_work_content` flag.

**Step 2: Parse JSONL defensively**

Walk `%USERPROFILE%/.codex/sessions`, select files whose session metadata or event timestamp overlaps the inclusive query range, and ignore malformed lines. Read `session_meta.payload.cwd`, `event_msg.payload.type == user_message`, and `response_item.payload.type == message` only; do not return developer instructions, tool inputs, or non-message events.

**Step 3: Register the Tauri command**

Export the module and add `load_codex_report_sessions` to the invoke handler. Return a readable error when the Codex sessions directory does not exist.

### Task 2: Add typed frontend service and deterministic formatter

**Files:**

- Create: `src/types/codex-report.ts`
- Create: `src/services/codex-report-service.ts`
- Create: `src/utils/codex-report.ts`

**Step 1: Mirror response models in TypeScript**

Define `CodexReportSession` and `CodexReportQuery` for the Tauri response and view filters.

**Step 2: Invoke the local command**

Expose `loadCodexReportSessions(query)` through `@tauri-apps/api/core` without introducing HTTP or AI services.

**Step 3: Format work facts locally**

Build pure utilities that normalize paths into project names, select session text matching a keyword, calculate the active time span, and render the Markdown work-record draft. Include only user requests and assistant output text; use labels such as `用户请求` and `处理结果`, never claim completion beyond the source text.

### Task 3: Build the Codex Daily Report view

**Files:**

- Create: `src/views/codex-report/CodexReportView.vue`
- Modify: `src/plugins/naive.ts`
- Modify: `src/i18n/messages/zh-CN.ts`
- Modify: `src/i18n/messages/en-US.ts`

**Step 1: Add initial filter state and loading flow**

Default both dates to today. Provide date range, project, keyword, and `only effective work` controls, an included-session checkbox list, and counts for total sessions, included sessions, projects, and effective time range.

**Step 2: Keep two separate editable panels**

Render the generated work record in one textarea and the default web-AI processing instruction in another. Re-reading sessions must refresh only the work record; it must never overwrite an edited instruction. Include a user-triggered reset for the instruction.

**Step 3: Add clipboard actions**

Implement copy for the work record, instruction, and combined content (instruction, separator, work record), with success/error notifications.

### Task 4: Surface the view in Lumina navigation

**Files:**

- Modify: `src/router/index.ts`
- Modify: `src/layouts/MainLayout.vue`

**Step 1: Register the route**

Add the `codex-report` child route using a lazy-loaded view.

**Step 2: Add a navigation entry and window title**

Add the report workbench to the existing compact sidebar, including localized label, icon, and titlebar name.

### Task 5: Verify without build or lint commands

**Files:**

- Review: all files above

**Step 1: Inspect changed files and source control status**

Use `git diff --check` and targeted source inspection to catch whitespace errors, missing imports, mismatched invoke command names, and accidental AI/network integration.

**Step 2: Hand off manual verification**

Do not run project build, lint, browser automation, or UI automation unless explicitly requested. Ask the user to open the new Codex Daily Report item, load a day with known sessions, edit both panels, and verify each copy action.
