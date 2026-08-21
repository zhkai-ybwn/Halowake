---
target: Lumina Git Assistant current UI after layout pass
total_score: 31
p0_count: 0
p1_count: 1
timestamp: 2026-06-26T03-00-42Z
slug: src-views-git-assistant-gitassistantview-vue
---

# Lumina Git Assistant UI Critique

Target: src/views/git-assistant/GitAssistantView.vue
Score: 31/40

## Design Health Score

| #         | Heuristic                       | Score     | Key Issue                                                                                                                                                 |
| --------- | ------------------------------- | --------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1         | Visibility of System Status     | 3         | Repository sync, selected file count, and empty states are visible; long-running AI/Git progress still depends on dialogs/drawers.                        |
| 2         | Match System / Real World       | 3         | Git terminology and flow are natural for developers; DevDock switching is still not represented in the shell.                                             |
| 3         | User Control and Freedom        | 3         | Refresh, folder switching, selection clearing, drawers, and dialogs provide exits; undo/recovery for accidental commit composition is limited to history. |
| 4         | Consistency and Standards       | 3         | Panels, buttons, and table patterns are more consistent after layout cleanup; some Naive UI and custom controls still mix interaction vocabularies.       |
| 5         | Error Prevention                | 3         | Submit gating and the new two-step commit flow prevent the main premature-submit error; remote/sync edge states are still dense.                          |
| 6         | Recognition Rather Than Recall  | 3         | The commit precondition is now visible, and empty states explain context; right-click table actions remain discoverability-heavy.                         |
| 7         | Flexibility and Efficiency      | 3         | Batch selection, double-click diff, context menu, resizable columns, recent repos, and history serve power users. Keyboard accelerators are not explicit. |
| 8         | Aesthetic and Minimalist Design | 3         | Restrained product UI and grouped top actions reduce noise; toolbar metrics and file selection presets still create a busy first scan.                    |
| 9         | Error Recovery                  | 3         | Error banner and command dialogs appear to support recovery; quality of exact messages needs runtime review.                                              |
| 10        | Help and Documentation          | 4         | Inline flow copy, empty-state guidance, labels, and concise hints are now strong for the primary task.                                                    |
| **Total** |                                 | **31/40** | **Good**                                                                                                                                                  |

## Anti-Patterns Verdict

This no longer reads like a generic AI-generated dashboard. The interface is recognizable as a dense developer workbench: restrained color, consistent small controls, table-first Git workflow, and no decorative hero/card-grid patterns. The main remaining tell is functional density rather than AI slop: too many top-level metrics and preset file-selection actions compete during the first scan.

Deterministic scan: 0 findings. The detector returned an empty JSON array for GitAssistantView, GitCommitAssistant, GitChangeExplorer, and GitStatusBar. No false positives.

Visual overlays: not run. The user's AGENTS instructions prohibit default browser-based testing, so no browser overlay or Playwright inspection was attempted.

## Overall Impression

The last layout pass moved the UI from acceptable to solid. The primary commit workflow is now legible: pick files first, then edit/submit the commit message. Empty states are also more helpful. The biggest opportunity is now workflow consolidation: reduce the number of equally visible controls in the top bar and file-selection toolbar, especially before adding DevDock.

## What's Working

- The new commit flow strip makes the dependency between selected files and commit message explicit, which fixes the previous sequencing problem.
- The empty states now teach the current state instead of leaving a blank table surface.
- Grouping Pull/Fetch/Push separately from folder/refresh actions makes the top toolbar easier to parse.

## Priority Issues

[P1] DevDock navigation has no reserved home yet
Why it matters: The product direction is now project-level workspace plus Git and DevDock views, but the current shell still presents the app as Lumina - Git Assistant. Adding DevDock without a deliberate switcher will either crowd the top toolbar or force a later shell rework.
Fix: Add a compact segmented control next to the repository switcher: Git Assistant | DevDock. Keep the selected project path shared across both views.
Suggested command: $impeccable shape DevDock integration

[P2] File selection presets still overload the second toolbar
Why it matters: Select All, None, Untracked, Versioned, Added, Deleted, Modified, Conflicted, Files are useful, but nine choices in one row exceeds the 4-item decision threshold and looks like filters even though they mutate selection.
Fix: Keep All / None / Recommended visible, move the rest into a Select by status dropdown.
Suggested command: $impeccable distill GitChangeExplorer selection toolbar

[P2] Power-user affordances are present but not surfaced consistently
Why it matters: Double-click diff, right-click context menu, resizable columns, and F5 refresh are efficient, but users discover them by hints or accident.
Fix: Add compact tooltips and a small command/help affordance rather than more visible text. Keep this subtle.
Suggested command: $impeccable clarify Git Assistant microcopy

[P2] Runtime accessibility is still unverified
Why it matters: Static source shows labels and focus styles in places, but table row click/double-click, right-click menus, and drawer focus behavior need actual keyboard/screen-reader validation.
Fix: Run an accessibility-focused audit when browser testing is explicitly allowed.
Suggested command: $impeccable audit Git Assistant accessibility

## Persona Red Flags

Alex (Power User): Alex benefits from batch selection, resizable columns, history, file log, and context menus. Red flag: key actions like diff, command history, and status selection are not exposed through a visible command palette or shortcut model, so efficiency depends on learned mouse gestures.

Jordan (First-Timer): Jordan now sees the correct sequence in the commit panel and gets useful clean/empty states. Red flag: the file selection toolbar still contains many status words at once, and it is not obvious that clicking them changes selection rather than filtering the table.

Sam (Accessibility-Dependent User): Sam has visible labels for most buttons and fields. Red flag: interactive table rows, double-click diff, context menu actions, and drawer/modal focus trapping cannot be trusted without runtime keyboard testing.

## Minor Observations

- The commit flow strip is useful, but it may need wrapping behavior if translated strings get longer.
- The top status metrics are valuable for Git users, but DevDock will need a different set; avoid simply adding more pills.
- The empty clean state mentions Log, but Log is currently in the AI/action side panel, not near the empty state.

## Questions to Consider

- Should DevDock become a peer view of Git Assistant now, before more project-level UI is added?
- Which file selection presets are actually used daily, and which belong in a dropdown?
- Should Lumina expose a command palette instead of adding more toolbar buttons?
