<div align="center">
  <img src="src/assets/logo.png" alt="Halowake Logo" width="92" />

  # Halowake

  **A Local-First Developer Workbench for AI Coding**

  Codex, Claude Code, Antigravity, and OpenCode write the code.  
  **Halowake handles everything around it: aggregating sessions, launching services, managing Git, skills, and AI quotas.**

  [![Release](https://img.shields.io/github/v/release/zhkai-ybwn/Halowake?label=Release)](https://github.com/zhkai-ybwn/Halowake/releases/latest)
  [![License](https://img.shields.io/github/license/zhkai-ybwn/Halowake)](LICENSE)
  [![Platforms](https://img.shields.io/badge/Windows%20%7C%20macOS%20%7C%20Linux-Desktop-555)](https://github.com/zhkai-ybwn/Halowake/releases/latest)
  [![Tauri](https://img.shields.io/badge/Tauri-2-24C8D8?logo=tauri&logoColor=white)](https://v2.tauri.app/)

  **[Download Releases](https://github.com/zhkai-ybwn/Halowake/releases/latest)** · **[Quick Start](#-get-started-in-3-minutes)** · **[Feature Overview](#-what-can-you-do-with-halowake)** · **[Feedback & Issues](https://github.com/zhkai-ybwn/Halowake/issues/new/choose)**

  [简体中文](README.md) · [English](README.en.md)
</div>

---

<p align="center">
  <img src="src/assets/report-preview.png" alt="Halowake — Aggregating sessions and work logs across multiple AI coding agents" width="100%" />
  <sub>A real-world use case: Extracting work facts from local agent sessions, selecting records, and copying them to an AI for daily standup reports.</sub>
</p>

## Why Halowake?

When working with multiple AI coding agents, writing code is only part of the job:

- Before finishing work, sifting through different agent logs to piece together what was accomplished today.
- Maintaining multiple projects at once, switching between terminals, starting services, and tracking down occupied ports.
- The agent modified dozens of files, but before committing you still need to review diffs, pick files, and craft commit messages.
- Skills are scattered across multiple tools and folders, making them hard to find and manage; subscription quotas need to be checked separately.

**Halowake does not provide another code-generation agent, nor does it replace your IDE.** It is a desktop app running locally on your machine that brings these repetitive, fragmented developer tasks together into one place.

## ✨ What Can You Do with Halowake?

### 1. Bring work records from multiple coding agents together

No need to scroll through chat history one by one. Halowake directly parses locally saved sessions from **Codex CLI, Claude Code, Google Antigravity, and OpenCode**, filtering by date, project, tool, and keyword, letting you select records to produce an editable summary of work facts.

- **Local extraction, 0 extra API tokens**: Scanning, parsing, and structuring raw sessions does not call any LLM.
- **Human-inspectable results**: Exclude noisy sessions, selectively include items, and edit Markdown.
- **One-click copy to AI**: Copy work facts together with standup/weekly prompt templates, then feed them to your preferred AI to generate final summaries.

> **Note**: "0 Token" refers to **local session extraction and work fact aggregation**; when copying content to an external AI to generate final reports, billing depends on the service you use.

### 2. View AI quotas in one place, with opt-in Codex quota warmup

Check configured AI service quotas, reset times, and consumption pace. For Codex, you can also **voluntarily enable local scheduled warmup** to align quota refresh cycles with your working hours.

- Supports Codex, Claude Code, Gemini, DeepSeek, OpenRouter, and more; **actual readable metrics depend on the provider and auth method**.
- Shows burn pace based on available quota windows; supports multi-account configuration and individual refresh.
- Codex warmup allows configuring **active weekdays, time slots, check intervals, and max wait limits**, with controls to pause or trigger manually anytime.
- Background scheduler features node deduplication, timeouts, and error handling, preventing unconditional repeated warmup requests.

> **Codex Warmup Boundaries**: This is an **opt-in helper feature (disabled by default)** that requires an existing local Codex CLI login session. It attempts to send a lightweight request via the Codex protocol, but **does not guarantee that the upstream service will open or reset a new 5-hour quota window**; the actual outcome depends on the server's quota response. This operation may consume actual quota, so review your subscription terms first.

<p align="center">
  <img src="src/assets/ai-quota-preview.png" alt="Halowake — AI Quota Management" width="100%" />
</p>

### 3. DevDock: Manage developer services across multiple projects

No need to remember which terminal launched which service, or repeatedly hunt down processes hogging ports.

- Discovers runnable commands from `package.json`, common Python entry points, and project scripts.
- Start, stop, and restart multiple project services in a single dashboard while monitoring output logs.
- Sniffs service ports and localhost URLs from logs to open pages with one click.
- Troubleshoot port conflicts, identify culprit processes, and terminate them after confirmation.

<p align="center">
  <img src="src/assets/devdock-preview.png" alt="Halowake — DevDock Multi-Project Process Workbench" width="100%" />
</p>

### 4. Git Assistant: Maintain full control before committing

After an AI generates code, you still decide **which files belong in the commit**.

- Inspect workspace status and Working Tree / Staged / HEAD diffs.
- Selectively commit specified files; stages selected files while preserving and restoring the original index state of unselected files to reduce accidental commit risks.
- Analyze diffs and generate Conventional Commit messages using your configured Ollama or OpenAI-compatible model.
- Browse commit history and branches, executing common Git operations like Pull, Push, Merge, Rebase.
- Pair with Local Code Review: deterministic rule checks + optional AI semantic audits to see files and issue severity levels.

<p align="center">
  <img src="src/assets/git-assistant-preview.png.png" alt="Halowake — Git Assistant File Changes and Commits" width="100%" />
</p>

### 5. Skills Center: Discover, install, and manage Agent Skills

Unified view for installed skills across Codex, Claude Code, Antigravity, and OpenCode, with discovery and selection from an online catalog.

- Fast search across synced skills using local SQLite FTS5; supports filtering and sorting.
- Inspect skill sources and documentation; install to the **current project, a specific agent, or global directories**.
- Check for updates, inspect installation history, and uninstall safely; detects local modifications to prevent silent overwrites.
- Online catalog is sourced from the independent [Halowake Skills Registry](https://github.com/zhkai-ybwn/halowake-skills-registry) (requires network connection to sync).

> Skills are external code and instructions. Inspect sources and `SKILL.md` before installing; never execute untrusted scripts in untrusted repositories.

### Additional handy tools

| Feature | Purpose |
| --- | --- |
| **Local Code Review** | Configurable review rules and models, audits selected files, and saves review records |
| **Agent Storage** | Inspect disk space across multiple agents and safely clean clearly listed caches |
| **Markdown Workbench** | Markdown preview, syntax highlighting, document outline (TOC), and detached window reading |
| **Desktop Experience** | Dark / Light theme, English & Chinese interface, system tray, and in-app update checks |

## 🚀 Get Started in 3 Minutes

### 1. Install

Head to **[GitHub Releases to download the latest version](https://github.com/zhkai-ybwn/Halowake/releases/latest)**, choosing the installer for your operating system.

| Operating System | Release Artifact |
| --- | --- |
| **Windows x64** | `Halowake_<version>_x64-setup.exe` |
| **macOS (Intel / Apple Silicon)** | `Halowake_<version>_universal.dmg` |
| **Linux (Debian / Ubuntu family)** | `Halowake_<version>_amd64.deb` |
| **Linux (AppImage)** | `Halowake_<version>_amd64.AppImage` |

> The artifacts above correspond to the current release pipeline and existing packages for v1.4.4. Features, permissions, and installation experience may vary across platforms; if you encounter issues, please submit your OS version and reproduction steps. **A standalone Windows portable `Halowake.exe` is not currently distributed in the v1.4.4 release.**

### 2. Open Your Project

Launch Halowake, then add or select a local project directory. You can start by checking detected project scripts in **DevDock**, or head to **Git Assistant** to inspect your repository status.

### 3. Start with a Scenario

- **Prepare a standup report**: Open "Session History", pick a date and Agent → load sessions → check relevant records → generate and copy.
- **Manage project services**: Open "DevDock" → select project scripts → start services → monitor logs and ports.
- **Safely commit changes**: Open "Git Assistant" → inspect diffs → select files → edit / generate commit message → commit.
- **Check quotas**: Open the Quota Center → configure or discover supported accounts → review quotas; Codex scheduled warmup can be opted into separately.

## 🔒 Local-First & Data Boundaries

Project scanning, Git operations, session parsing, report aggregation, and local state run on your machine without relying on Halowake-hosted cloud services.

However, **Local-first does not mean every feature is completely offline**:

- **AI Generation / AI Review**: When using online models, necessary prompts, selected diffs, or review snippets are sent to your configured provider; choosing local Ollama runs supported models on your device.
- **Quota queries & Codex warmup**: Accesses relevant service endpoints or communicates via local clients with upstream servers.
- **Skills registry sync & installation**: Downloads data from configured remote registries and skill repositories.
- **Credentials**: Uses local configuration and persistence; Windows protects certain keys via DPAPI. Protect your local accounts and never expose sensitive configurations or logs publicly.

**Always verify target repositories, processes, or directories before executing Git writes, process termination, or cache cleanup.**

## 🛠️ Run from Source

The development environment requires **Node.js 20+, Rust 1.77.2+, Git**, and OS-specific [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/).

```bash
git clone https://github.com/zhkai-ybwn/Halowake.git
cd Halowake
npm ci
npm run tauri:dev
```

Build production installer:

```bash
npm run tauri:build
```

Before submitting changes, you can run:

```bash
npm run lint
npm run test:unit
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

**Tech Stack**: Tauri 2 · Rust · Vue 3 · TypeScript · Vite · Naive UI · Pinia · SQLite (WAL / FTS5)

## 🤝 Community & Contributing

Halowake is continuously evolving. The most helpful contribution is **real-world usage feedback**: which agent you use, which OS, which step in your workflow takes the most time, and which feature did not meet expectations.

- [Report a Bug / Request a Feature](https://github.com/zhkai-ybwn/Halowake/issues/new/choose)
- [View Contributing Guide](CONTRIBUTING.md)
- [View Releases & Downloads](https://github.com/zhkai-ybwn/Halowake/releases)

If Halowake helps your daily workflow, please consider giving the repository a **Star** so more developers can discover it.

## 📄 License

[MIT](LICENSE) © Halowake contributors
