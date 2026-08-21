---
target: src/views/devdock/DevDockView.vue
total_score: 27
p0_count: 0
p1_count: 2
timestamp: 2026-06-26T05-02-40Z
slug: src-views-devdock-devdockview-vue
---

Design Health Score: 27/40

Target: src/views/devdock/DevDockView.vue

Overall: DevDock is now functionally credible for a local developer workbench, but the visual hierarchy is still in a transitional state. The multi-project list is useful and the process panel is the right architecture, yet expanded project rows, nested scroll areas, long command lines, and a cramped process panel create more cognitive load than necessary.

Strengths:

- The split between project scripts and process management matches the user's workflow.
- Running status is visible in both script rows and the process panel.
- The interface uses restrained product UI styling and avoids decorative slop.

Priority Issues:
P1: Nested scroll and expanded project height make the page feel unstable and hard to scan.
P1: Process panel actions are too compressed; title, status, pid, port, and five actions compete in a narrow column.
P2: Long script commands dominate rows and should be progressively disclosed or truncated with better affordance.
P2: Running service URL is present but not yet visually prominent enough for the primary post-start action.
P2: Help and recovery cues remain thin for failed starts, stopped processes, and stale ports.

Detector: clean, no findings.
