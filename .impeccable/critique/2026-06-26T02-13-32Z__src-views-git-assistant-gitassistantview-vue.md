---
target: Lumina Git Assistant current UI screenshot and source
total_score: 27
p0_count: 0
p1_count: 2
timestamp: 2026-06-26T02-13-32Z
slug: src-views-git-assistant-gitassistantview-vue
---

# Lumina Git Assistant UI Critique

Target: src/views/git-assistant/GitAssistantView.vue
Score: 27/40

## Summary

The current Git Assistant UI has a solid desktop-tool foundation: restrained palette, consistent panels, clear repository context, and a practical table-first workflow. It feels functional and reasonably trustworthy. The main weakness is not visual decoration; it is task hierarchy. The screen exposes repository status, sync actions, commit authoring, AI generation, file selection, filters, and empty table state all at once, without a stronger primary path for first-time or distracted users.

## Priority Issues

- P1: Primary workflow is not staged. Commit message composition appears before file selection, while the disabled submit state depends on selected files. This makes the top of the screen feel actionable before the prerequisite is complete.
- P1: Top toolbar mixes project identity, status metrics, sync operations, directory selection, and refresh actions at equal weight. Users must parse too many controls before understanding the page.
- P2: Empty state in the change browser is too passive. With 0 changed files, the largest region becomes mostly blank instead of explaining the current clean state and offering useful next actions.
- P2: AI panel action hierarchy is partially unclear. Generate is strong, but history, log, prompt preview, and disabled review code have similar visual weight despite different usefulness.
- P2: Accessibility and keyboard confidence cannot be confirmed from screenshot and static scan. Some controls have visible labels, but focus order, table keyboard navigation, and status announcements need explicit verification.

## Strengths

- Restrained product UI language fits a developer desktop tool.
- Table density and resizable columns are appropriate for Git file review.
- Current project and branch context are visible, which reduces memory load.
- The right AI panel is a reasonable containment strategy for secondary automation.

## Recommended Direction

Keep the current shell. Do not add a heavy sidebar yet. For Git/DevDock, add a compact segmented switch near the repository selector and preserve the top bar as shared project context. Then improve the Git page by staging the core flow visually: project status, file selection, commit draft, AI assistance.
