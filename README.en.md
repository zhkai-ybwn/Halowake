# Halowake

<p align="center">
  <img src="src/assets/logo.png" alt="Halowake Logo" width="96" height="96" />
</p>

<p align="center">
  <strong>A Local-First Companion for AI Coding Workflows</strong>
</p>

<p align="center">
  <a href="./README.md"><b>简体中文</b></a> | <b>English</b>
</p>

<p align="center">
  <a href="https://github.com/zhkai-ybwn/Halowake/releases/tag/v1.4.4"><img src="https://img.shields.io/badge/release-v1.4.4-blue.svg?style=flat-square" alt="Release v1.4.4" /></a>
  <a href="https://v2.tauri.app/"><img src="https://img.shields.io/badge/Tauri-v2-24C8D8.svg?style=flat-square&logo=tauri&logoColor=white" alt="Tauri v2" /></a>
  <a href="https://vuejs.org/"><img src="https://img.shields.io/badge/Vue-3.5-4FC08D.svg?style=flat-square&logo=vue.js&logoColor=white" alt="Vue 3.5" /></a>
  <a href="https://www.typescriptlang.org/"><img src="https://img.shields.io/badge/TypeScript-5.x-3178C6.svg?style=flat-square&logo=typescript&logoColor=white" alt="TypeScript" /></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-1.77+-DEA584.svg?style=flat-square&logo=rust&logoColor=white" alt="Rust 1.77+" /></a>
  <a href="./LICENSE"><img src="https://img.shields.io/badge/License-MIT-green.svg?style=flat-square" alt="License MIT" /></a>
  <img src="https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg?style=flat-square" alt="Platform" />
</p>

<p align="center">
  <a href="https://github.com/zhkai-ybwn/Halowake/releases">Download Releases</a> ·
  <a href="#-feature-matrix">Features</a> ·
  <a href="#-installation--getting-started">Getting Started</a> ·
  <a href="https://github.com/zhkai-ybwn/Halowake/issues/new/choose">Feedback & Issues</a>
</p>

---

## 📖 Product Vision & Philosophy

> **Codex, Claude Code & Antigravity write the code. Halowake handles everything around it.**

**Halowake** does not replace coding agents like Codex, Claude Code, OpenCode, or Google Antigravity. Instead, it pairs with modern AI coding workflows to handle routine, repetitive development tasks outside code generation with a privacy-first, cost-efficient approach:

- 🛡️ **Local-First & Zero Telemetry**: Configuration, Git commit histories, code review records, session analysis, and storage metrics run 100% locally on your machine. Your code and data never leave your device.
- ⚡ **Zero-Token & Cost-Efficient**: Daily standup report generation uses 100% local disk IO parsing (**0 Token Cost**). Git AI commits and lightweight reviews utilize cost-effective models (DeepSeek, Ollama, OpenAI Compatible), reserving expensive Agent tokens for high-value coding tasks.
- 🔄 **Workflow Closed-Loop**: Connects code generation with diff review, atomic staging commit protection, multi-project dev service orchestration, Agent skill ecosystems, and rolling quota keep-alive scheduling.

---

## 🌟 Feature Matrix

```
┌────────────────────────────────────────────────────────────────────────┐
│                              HALOWAKE                                  │
├───────────────────┬───────────────────┬────────────────────────────────┤
│  🚀 Agent Economy │  🚢 Dev Processes │  🔀 Git & Workflow             │
│ • Skills Hub 2.0  │ • DevDock Hub     │ • Git Assistant (AI Commits)   │
│ • FTS5 Search     │ • Port Kill/Check │ • Atomic Staging Protection    │
│ • Codex Warmup    │ • Tree Process Kill│• Local Code Review (SQLite)   │
├───────────────────┴───────────────────┴────────────────────────────────┤
│  📝 Insights & Workspace Health                                        │
│ • 0-Token Standups (Codex / Claude Code / Antigravity / OpenCode)      │
│ • AI Quota Burn Pace Monitoring (5-Hour Rolling / Daily Resets)        │
│ • Agent Storage Analyzer & Cache Cleaner                               │
│ • Standalone Markdown Preview (TOC / KaTeX / Syntax Highlighting)      │
└────────────────────────────────────────────────────────────────────────┘
```

---

### 1. 📦 Agent Skills Hub 2.0 (Agent Skills Ecosystem & Store)

A unified desktop store to discover, search, install, and audit modern AI coding agent skills across **Codex**, **Claude Code**, **Google Antigravity**, and **OpenCode**.

- **10,000+ Curated Skills**: Explore open-source skills covering code review, automated testing, DevOps, refactoring, and internationalization.
- **FTS5 High-Precision Hybrid Search (Strict P@1 = 88.5%)**:
  - Powered by SQLite FTS5 inverted indexing and BM25 weighted ranking;
  - Tiered field authority ranking (`Name` > `Title` > `Tags` > `Summary` > `Description`), with developer synonym expansion, cross-language queries, and natural language intent matching;
- **Conflict Prevention & Overwrite Protection**:
  - Automated dependency verification during installation;
  - Safeguards custom local modifications against accidental overwrites;
- **Version Detection & Lifecycle Auditing**:
  - Real-time updates detection for installed skills;
  - One-click safe uninstallation with local audit trail persistence.

![Agent Skills Hub](src/assets/lumina-workflow.png)

---

### 2. ⏰ Codex Quota Scheduler & Warmup (Codex Quota Scheduler)

Addresses the **5-hour rolling quota reset window** of Codex subscriptions with an opt-in, bounded, local background warmup scheduler.

- **Rolling Quota Keep-Alive**: Automatically sends a lightweight probe at scheduled intervals to activate fresh 5-hour quota windows ahead of your coding sessions.
- **Flexible Scheduling**:
  - Defaults to weekdays (Mon–Fri) at `08:00`, `13:00`, and `18:00`;
  - Fully configurable target time slots, weekdays, check intervals (30s to 300s), and maximum wait timeouts (5 to 60 mins);
  - One-click manual check/warmup with pause and resume controls;
- **Isolated Sandboxing**:
  - Warmup runs in a temporary directory and clean workspace with an isolated `CODEX_HOME`;
  - Interacts via the official Codex app-server protocol, copying only the active local ChatGPT session credentials **without refresh tokens, user configurations, external MCP tools, or custom hooks**;
- **Anti-Overspend & Intelligent Backoff**:
  - Maximum of one request per scheduled node; skips execution if a fresh window is already active;
  - Exponential backoff with circuit breaker after 5 consecutive transient network errors; terminates immediately upon auth or protocol failures;
  - Zero sensitive tokens or raw RPC credentials logged to SQLite.

---

### 3. 🚢 DevDock — Multi-Project Process Orchestrator

A central orchestrator for multi-repo and multi-runtime local developer services.

- **Smart Script Discovery**: Scans `package.json` scripts across monorepos alongside Python, Shell, PowerShell, and Cmd scripts.
- **Cross-Platform Tree-Level Process Termination**:
  - Uses Windows Job Objects and Unix Process Groups;
  - Kills child and grandchild processes reliably to prevent zombie background processes and port leaks;
- **Port Sniffing & Localhost URL Extraction**: Automatically extracts active listening ports and `localhost` URLs from ANSI service logs for 1-click browser navigation.
- **🔍 Port Troubleshooter & Process Cleaner**:
  - Inspects port occupancies and locates culprit process names and PIDs;
  - One-click force-kill for stuck background services;
- **ANSI Color Log Monitor**: Real-time terminal log viewer with log level filtering, keyword search, and script restart/rerun controls.
- **Pinning & Sorting**: Pin frequently used services to the top and customize display order.

![DevDock Process Management](src/assets/devdock-preview.png)

---

### 4. 🔀 Git Assistant — AI Commits & Atomic Staging Protection

Streamlines code review, staging, and structured commits following AI code generation.

- **Diff-Driven Conventional Commit Generation**: Analyzes actual workspace diffs using budget-friendly models (DeepSeek, Ollama, OpenAI-compatible) to draft Conventional Commits messages.
- **🛡️ Atomic Staging Protection**:
  - Commits only the user-selected files from the UI;
  - Automatically restores unselected staged files to their exact index state post-commit, completely preventing unintentional commits or context pollution;
- **Full-Featured Git Workbench**:
  - Hierarchical file change tree with folder expand/collapse;
  - Three-tier diffing: Working Tree, Staged (Index), and HEAD diffs;
  - Clear conflict status indicators;
  - Branch checkout, creation, and merging;
  - Git Log history inspection with commit diff inspection.

![Git Assistant](src/assets/git-assistant-preview.png.png)

---

### 5. 📝 AI Coding Session History & Zero-Token Standups

Aggregates local sessions across major AI coding tools into an offline developer journal.

- **Native Multi-Agent Parsing**:
  - 🔵 **Codex CLI**: Parses `~/.codex/sessions/` JSONL records;
  - 🟢 **Claude Code**: Parses `~/.claude/projects/` session logs;
  - 🟣 **Google Antigravity**: Parses `~/.gemini/antigravity/brain/` transcripts;
  - 🟠 **OpenCode**: Reads local session state and workspace diffs;
- **⚡ Zero-Token Local Disk IO**: Parses and structures records locally using high-speed disk IO with **zero external API token costs**.
- **Multi-Dimensional Filters**: Filter by date range (Today, Yesterday, Last 7 Days, Custom), tool source, repository path, or keywords.
- **One-Click Standup Prompts**: Copy formatted, structured work facts into web models (ChatGPT, Claude, Kimi, etc.) to generate morning standups and weekly summaries.

![AI Session History](src/assets/report-preview.png)

---

### 6. 📊 AI Quota Tracking & Burn Pace Monitor

A centralized dashboard to track balances, limits, and consumption pace across multiple AI providers.

- **Unified Account Balances**: Tracks DeepSeek, Gemini (AI Studio / Pro), OpenAI, Claude, OpenRouter, Cursor, ZCode, and QCode.
- **📈 Burn Pace Evaluation**:
  - Evaluates consumption velocity against 5-hour rolling windows or daily refresh cycles;
  - Flags status as `On Pace` (healthy) or `Over Pace` (approaching limit warning) to help developers stay within budget;
- **Modal Quota Hub**: Multi-account management with customizable refresh intervals and timezone alignment.

![AI Quota Dashboard](src/assets/ai-quota-preview.png)

---

### 7. 🛡️ Local Code Review Engine

Performs dual-engine deterministic rule checking and semantic AI audits before commits or merges.

- **Deterministic Rules + AI Diagnostics**: Executes deterministic local rules (naming conventions, security prohibitions) combined with LLM semantic analysis for architectural code smells.
- **Tiered Budget Modes**: `Strict` (deep analysis), `Balanced` (standard), and `Fast` (quick check).
- **Line-Level Defect Severity**: Categorizes findings into `Critical`, `Major`, `Minor`, and `Suggestion` with actionable remediation guidance.
- **Offline SQLite Storage**: Saves review histories locally for audit and trend tracking.

![Local Code Review](src/assets/code-review-preview.png)

---

### 8. 🧹 Agent Storage Analyzer & Cache Cleaner

Intensive AI coding assistant usage accumulates gigabytes of cached transcripts, temporary workspaces, and logs.

- **Deep Agent Disk Scan**: Scans disk footprints across Antigravity, Codex, and Claude Code;
- **Storage Breakdown Visualizer**: Displays disk consumption by directory, cache category, and log volume;
- **Safe Cleanup**: Safely prunes stale temporary files and oversized transcripts with one click to reclaim gigabytes of disk space.

---

### 9. 📄 Standalone Markdown Workspace

- **Multi-Window Preview**: Open Markdown files in detached floating windows without interfering with your main workspace.
- **GitHub Flavored Markdown (GFM)**: Full rendering support for tables, task lists, alerts/callouts, syntax-highlighted code blocks, and KaTeX math formulas.
- **Interactive Outline (TOC)**: Automatically extracts document headings into a clickable table of contents for smooth navigation.

---

### 10. 🎨 Modern Desktop Interface & Unified Design System

- **macOS & Modern Desktop Aesthetics**: Subtle borders, layered elevation shadows, design tokens, and smooth spring animations.
- **Adaptive Light & Dark Themes**: High-contrast, eye-friendly themes tailored for daylight and late-night coding.
- **Standardized Component Library**: Consistent modal panels, sliding sheets, drawers, and unified close buttons (`WorkbenchCloseButton`).

---

## 🛠️ Tech Stack & Architecture

```
┌────────────────────────────────────────────────────────┐
│                   Frontend (UI Layer)                  │
│       Vue 3.5  •  TypeScript  •  Vite  •  Naive UI     │
│        Pinia State  •  Vue-Router  •  Vue-I18n         │
└───────────────────────────┬────────────────────────────┘
                            │  Tauri IPC (Async Commands)
┌───────────────────────────┴────────────────────────────┐
│                   Backend (Rust Core)                  │
│     Tauri v2  •  Tokio Async Runtime  •  Rusqlite      │
│   SQLite (FTS5 / WAL)  •  Chrono  •  Reqwest  •  Ring  │
│  Job Object (Win) / Process Group (Unix)  •  ANSI Dec  │
└────────────────────────────────────────────────────────┘
```

- **Frontend**: Vue 3.5 Composition API, TypeScript strict mode, Naive UI components, and CSS token systems;
- **Backend Core**: Rust 1.77+ on Tauri v2, powered by Tokio for concurrent background tasks;
- **Persistence**: Embedded SQLite 3 with WAL journal mode and FTS5 inverted indexing for sub-millisecond full-text queries.

---

## 📥 Installation & Getting Started

### Method 1: Pre-Built Installers (Recommended)

Download the latest release from [GitHub Releases](https://github.com/zhkai-ybwn/Halowake/releases):

- **Windows**:
  - `Halowake_x.x.x_x64-setup.exe` (NSIS setup wizard with auto-update support)
  - `Halowake.exe` (Portable executable)
- **macOS**: `.dmg` (Universal binary for Apple Silicon & Intel)
- **Linux**: `.deb` / `.AppImage`

### Method 2: Build from Source

#### Prerequisites
- [Node.js](https://nodejs.org/) (>= 20.0.0)
- [Rust & Cargo](https://www.rust-lang.org/) (>= 1.77.2)
- Git
- OS-specific [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/)

#### Build Steps

```bash
# 1. Clone repository
git clone https://github.com/zhkai-ybwn/Halowake.git
cd Halowake

# 2. Install dependencies
npm install

# 3. Start local development mode
npm run tauri:dev

# 4. Compile production bundle
npm run tauri:build
```

The compiled binaries will be output to `src-tauri/target/release/bundle/`.

---

## 🔒 Privacy & Local-First Guarantees

1. **Zero External Data Leakage**: Project script execution, Git commands, code reviews, and session parsing occur locally. No telemetry or code data is ever uploaded to external servers.
2. **Encrypted Local Credentials**: API keys are saved locally in SQLite and only connect directly to specified upstream providers (DeepSeek, OpenAI, Google) when user actions are triggered.
3. **Bounded Execution**: DevDock strictly confines execution to user-approved project directories; Codex warmup runs strictly inside temporary isolated sandboxes.

---

## 🤝 Community & Contributing

Contributions, feature requests, and bug reports are welcome!

- 🐛 **Report a Bug**: [GitHub Issues](https://github.com/zhkai-ybwn/Halowake/issues/new/choose)
- 💡 **Request a Feature**: [Feature Request](https://github.com/zhkai-ybwn/Halowake/issues/new/choose)
- 💬 **Code Contribution**: Please refer to [CONTRIBUTING.md](CONTRIBUTING.md) for workflow details.

---

## 📄 License

Distributed under the [MIT License](LICENSE).
