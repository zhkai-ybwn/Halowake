# Lumina macOS-Inspired UI Overhaul Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use `executing-plans` to implement this plan task-by-task and `macos-design` for every visual decision.

**Goal:** Redesign Lumina as a native-feeling desktop workbench using macOS composition and interaction principles while preserving platform-native window controls, all existing workflows, internationalization, and independent light/dark themes.

**Architecture:** Keep the current Vue 3 and Tauri application structure. Establish semantic design tokens and reusable workbench primitives first, then migrate the application shell and feature surfaces without changing their domain behavior. Platform-specific chrome and shortcut labels are resolved at runtime; Windows keeps right-aligned caption controls while macOS uses integrated traffic lights.

**Tech Stack:** Vue 3, TypeScript, Pinia, Vue I18n, Naive UI, SCSS, Tauri 2.

---

## Scope

- Preserve Git Assistant, DevDock, Codex Daily Report, Settings, tray behavior, independent diff/log windows, locale switching, and `light` / `dark` / `system` appearance modes.
- Introduce a sparse 48–52px title/toolbar region, a labeled collapsible sidebar, solid content surfaces, translucent navigation chrome, native-feeling panels, progressive empty states, platform-aware shortcuts, and restrained motion.
- Add all new user-facing copy to both `zh-CN` and `en-US` locale files.
- Keep feature behavior stable while view structure and styling change.
- Do not add automatic updates, installer work, code signing, configuration migrations, telemetry, or a forced cross-feature product loop.
- Do not run Build, Lint, browser automation, or visual automation unless the user explicitly requests it.

## Task 1: Semantic Design Foundation

**Files:**

- Modify: `src/styles/tokens/_color-system.scss`
- Modify: `src/styles/tokens/_spacing.scss`
- Modify: `src/styles/tokens/_radius.scss`
- Modify: `src/styles/tokens/_shadow.scss`
- Modify: `src/styles/themes/_light.scss`
- Modify: `src/styles/themes/_dark.scss`
- Modify: `src/styles/base/_reset.scss`
- Modify: `src/styles/base/_typography.scss`
- Modify: `src/styles/base/_naive.scss`
- Modify: `src/styles/workbench/index.scss`

Create semantic window, sidebar, toolbar, content, elevated, control, separator, text, focus, status, blur, motion, and platform typography tokens. Design light and dark palettes independently and add reduced-motion behavior.

## Task 2: Reusable Native Workbench Primitives

**Files:**

- Modify: `src/components/workbench/WorkbenchButton.vue`
- Modify: `src/components/workbench/WorkbenchTopbar.vue`
- Modify: `src/components/workbench/WorkbenchDrawer.vue`
- Modify: `src/components/workbench/WorkbenchModalPanel.vue`
- Modify: `src/components/workbench/WorkbenchIdentity.vue`
- Modify: `src/components/workbench/WorkbenchSwitch.vue`
- Modify: `src/components/workbench/WorkbenchTag.vue`
- Create: `src/components/workbench/WorkbenchEmptyState.vue`
- Create: `src/components/workbench/WorkbenchShortcutHint.vue`
- Create: `src/components/workbench/WorkbenchIconButton.vue`

Make the primitives consume semantic tokens only. Standardize 28px controls, focus rings, layered elevation, translucent inspectors, quiet selected states, and 150–300ms feedback.

## Task 3: Platform-Adaptive Application Shell

**Files:**

- Modify: `src/layouts/MainLayout.vue`
- Modify: `src/i18n/messages/zh-CN.ts`
- Modify: `src/i18n/messages/en-US.ts`

Build a 50px draggable top region and a 224px labeled sidebar that can collapse to 56px. Keep Windows controls on the right; render macOS traffic lights on the left. Add route identity, sidebar toggle, settings placement, platform-aware shortcut hints, and a compact command switcher for navigation.

## Task 4: Git Assistant Surface

**Files:**

- Modify: `src/views/git-assistant/GitAssistantView.vue`
- Modify: `src/views/git-assistant/components/*.vue`
- Modify: `src/composables/git-assistant/*.ts` only when presentation state needs extraction

Recompose the screen as change list, diff content, and commit inspector. Move low-frequency repository and AI configuration actions into menus or drawers. Preserve every Git command and state transition.

## Task 5: DevDock Surface

**Files:**

- Modify: `src/views/devdock/DevDockView.vue`
- Modify: `src/views/devdock/components/*.vue`

Use a project source list, script content area, and on-demand process/log inspector. Collapse unused process chrome, consolidate project management actions, and add a native drag-over state without changing process execution.

## Task 6: Codex Report, Settings, and Auxiliary Windows

**Files:**

- Modify: `src/views/codex-report/CodexReportView.vue`
- Modify: `src/views/settings/SettingsView.vue`
- Modify: `src/components/settings/**/*.vue`
- Modify: `src/views/git-log/GitLogView.vue`
- Modify: `src/views/git-diff/GitDiffView.vue`

Apply source-list/content/inspector composition, segmented editing modes, compact filters, native settings rows, and the shared panel language.

## Task 7: Interaction and Accessibility Pass

**Files:**

- Modify: application shell and affected feature views
- Modify: both locale files

Add platform-aware `Ctrl/Cmd+K`, `Ctrl/Cmd+,`, page-specific find shortcuts, Escape dismissal, visible shortcut hints, keyboard navigation, clear focus states, reduced motion, and useful directory drag-and-drop feedback.

## Task 8: Approved Engineering Hardening

**Files:** determined by focused follow-up plans after the UI migration stabilizes.

Replace plain-text credential storage with OS credentials, prevent port-based termination of unrelated processes, isolate Codex session format adapters, split oversized modules, and add frontend/parser tests. Treat these as behavior-sensitive changes separate from the visual migration.

## Verification

- Use `git diff --check` and targeted source inspection after each batch.
- Check every new translation key in both locale files using `rg`.
- Check that components use semantic tokens and that no new hard-coded theme colors enter feature views.
- Hand off manual UI verification for Windows light, Windows dark, system mode switching, English, Chinese, sidebar expanded/collapsed, and all primary feature routes.
- Suggest Build, Lint, unit tests, and browser/UI validation to the user rather than running prohibited commands without explicit permission.
