# Codex quota scheduler implementation plan

**Goal:** Add a lightweight, opt-in Codex quota warmup schedule to the existing quota window.

**Architecture:** A Rust worker owns scheduling and bounded retries. Configuration and the latest 20 node records share the existing SQLite app_metadata store. Each attempt uses the official Codex app-server protocol in a temporary empty workspace and isolated CODEX_HOME; a short-lived copy of the current local login excludes its refresh token, user configuration, MCP servers and hooks. No independent service or additional provider support.

**Tech Stack:** Tauri, Tokio, rusqlite, chrono, tempfile, Vue, Naive UI, existing theme tokens and i18n.

## Accepted scope

- Defaults: disabled; 08:00, 13:00, 18:00; Monday–Friday; 60-second checks; 30-minute node deadline.
- Configurable times, weekdays, check interval (30/60/120/300 seconds), maximum wait (5–60 minutes).
- Local machine timezone. Resume only the current eligible node, never replay old nodes.
- Subscription login only; bind to its account ID and stop on account changes. No API-key fallback.
- Read fresh five-hour and weekly windows. Existing new windows satisfy a node without another request.
- At most one warmup request per node, persisted before dispatch. Ambiguous outcomes are verified without another request.
- Five consecutive transient failures stop a node; exponential backoff; every operation and the whole node are bounded. Authentication/protocol/unsupported quota failures stop immediately.
- Pause or configuration edits cancel current work. A deadline remains absolute across retries and restarts.
- History contains codes, timestamps and counts, never credentials or raw RPC errors.

## Implementation steps

1. Add isolated Codex protocol client, structured window classification and process timeouts.
2. Add validated configuration, durable node state, worker and three Tauri commands (load/save/check-now).
3. Add a compact settings modal from the quota toolbar; localize Chinese/English text.
4. Verify Rust state logic with inline unit tests, run existing relevant tests, TypeScript checking and production build. No browser automation or live warmup calls.

## Decisions

- Prefer one request per node over automatic inference retries: an uncertain request must not spend quota repeatedly.
- Use an isolated temporary login snapshot rather than inheriting arbitrary user hooks/tools. Read the user's current auth.json afresh for each attempt; never rotate or overwrite the original credentials. File-backed local ChatGPT login is required in this first version; expired/keyring-only credentials require user action.
- Keep normal polling configurable while implementing failure backoff internally.

## Manual acceptance

Open AI Quota → Codex schedule; edit/save/reopen; verify pause, weekdays and validation. With a supported local subscription login, enable a near-future node and inspect waiting/ready/confirmed states. Verify expiry, offline failure, pause during work and restart deduplication. UI and real-account validation are manual and were not authorized for automated browser testing.
