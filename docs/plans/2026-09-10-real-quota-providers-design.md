# Real quota provider integration design

## Goal

Replace guessed or outdated quota integrations with provider-specific data sources that have working open-source references. Never publish a numeric quota unless it came from a provider response or is clearly identified as a local estimate.

## Provider decisions

- Qoder CN: read the existing desktop `dt-`/`jt-` credential and call `GET https://openapi.qoder.com.cn/api/v2/quota/usage`. Show `userQuota` and `addOnQuota` separately, and use `GET /api/v2/user/plan` only as optional plan enrichment. A manually supplied `pt-` PAT is exchanged through `/api/v1/jobToken/exchange` first.
- Cursor: read `cursorAuth/accessToken` from `state.vscdb` and call the Connect RPC endpoint `POST https://api2.cursor.sh/aiserver.v1.DashboardService/GetCurrentPeriodUsage`. Preserve the independent Cursor Models and Other Models percentages.
- ZCode Start Plan: use the locally encrypted JWT with `GET https://zcode.z.ai/api/v1/zcode-plan/billing/balance`, including the client-identification headers required by the endpoint. Render each returned balance pool independently.
- Trae: keep local login detection only for the China client. The open-source implementation for the international service exposes session usage history, not a verified remaining-quota contract; do not turn that into a fake quota.
- OpenCode: official quota API is not available. Local message-cost scanning may be added later as an explicitly labeled estimate, but it must not be mixed with remote account quota.

## Error handling

Authentication failures must leave the provider unhealthy and explain whether the desktop login is stale or the manually supplied credential was rejected. Optional plan/profile enrichment may fail without discarding a successfully fetched quota. Credentials are never logged or included in error bodies.

## Verification

- Unit-test response parsing, pool separation, timestamp normalization, PAT detection, and invalid response rejection.
- Run Rust tests and Clippy on changed provider modules.
- Run frontend lint and production build after copy changes.
- Browser/UI automation remains out of scope; manually confirm cards against each provider dashboard.
