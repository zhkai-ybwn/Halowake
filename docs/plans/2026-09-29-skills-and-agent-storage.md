# Skills Management and Agent Storage Implementation Plan

**Goal:** Refine catalog pagination, enable safe uninstallation of installed Skills, and show where supported Agent tools use disk space.

**Architecture:** Keep catalog pagination within the existing Vue component. Route uninstallation through the existing Tauri command after replacing its string based path check with exact, revalidated skill root checks. Add a separate read only Tauri scanner for known Agent data roots and present the results in the existing storage settings page.

**Tech Stack:** Vue 3, Pinia, Naive UI, Tauri 2, Rust.

---

### Task 1: Refine catalog pagination

**Files:** `src/components/skills/explore/SkillExploreView.vue`

1. Keep the current page calculation and filtering.
2. Group the range, page buttons, page size and jump controls into a compact layout; show jump only when useful.
3. Check keyboard labels, narrow widths, and `npm run lint`.

### Task 2: Add safe Skill uninstallation

**Files:** `src-tauri/src/commands/skills.rs`, `src/types/skills.ts`, `src/services/skills/skills-service.ts`, `src/stores/skills.ts`, `src/components/skills/installed/InstalledSkillsView.vue`

1. Mark scanned standalone skill directories that are eligible for uninstall.
2. At command time, require the target to be a direct child of an approved skill root and reject directory links and non-skill entries.
   Detect `skills` CLI lock entries and direct those skills back to their original manager so its metadata stays consistent.
3. Show the exact path, Agent associations and local modification warning in a confirmation dialog.
4. Disable the row while deleting, rescan on success, and test the path guard in the existing Rust test module.

### Task 3: Add Agent disk inventory

**Files:** `src-tauri/src/commands/agent_storage.rs`, `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs`, `src/types/storage.ts`, `src/services/storage-service.ts`, `src/components/settings/panels/StorageSettingsPanel.vue`

1. Scan only known Agent data roots on demand, without following links or reparse points.
2. Group locations by Agent, report size and incomplete scans, and avoid double counting overlapping roots.
3. Display totals and individual locations with an open folder action in Storage settings.
   Include the largest immediate children for each location, following the directory breakdown used by open-source disk usage tools such as dua.
4. List exact cache folders separately, require confirmation, and clear only their contents while skipping links and files in use.

### Verification

Run `npm run lint`, `npm run build`, `cargo check`, `cargo test --lib`, and `git diff --check`. Browser validation requires separate authorization under the workspace rules.
