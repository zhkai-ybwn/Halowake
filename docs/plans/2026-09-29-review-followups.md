# Skills review follow-ups implementation plan

**Goal:** Resolve the confirmed defects in the current Skills and Agent storage changes without adding unverified installable content.

**Architecture:** Keep the current SQLite catalog and Vue views. Align cache validity with the repository contract; add a bounded documentation mirror retry; translate new Skills UI strings through the existing locale dictionaries; expand known Agent storage roots by operating system.

**Tech Stack:** Vue 3, vue-i18n, Pinia, Tauri 2, Rust.

---

1. Align registry cache validation with the backend's nonempty requirement. Verify with TypeScript build.
2. Retry GitHub source guides through jsDelivr only after a Raw failure, keeping size and path checks. Verify with TypeScript build.
3. Translate the new catalog controls, compact row, install history and install form in both supported locales. Verify translation key parity and lint.
4. Add an empty-state scan action, dismissible sync feedback, and a nonblocking tag editor. Verify lint and build.
5. Cover macOS and Linux application data roots in the Agent storage scan. Verify with Rust check and library tests.
6. Run `git diff --check`, `npm run lint`, `npm run build`, `cargo check`, and relevant Rust library tests. UI interaction verification remains manual under AGENTS.md.
