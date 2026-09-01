# Halowake

### A local-first companion for AI coding workflows.

<p align="center">
  <img src="src/assets/logo.png" alt="Halowake Logo" width="84" height="84" />
</p>

<p align="center">
  <strong>AI Coding 时代的本地开发伴侣</strong>
</p>

<p align="center">
  <a href="#english">English Documentation</a> ·
  <a href="https://github.com/zhkai-ybwn/Halowake/releases">下载发布版本</a> ·
  <a href="https://github.com/zhkai-ybwn/Halowake/issues/new/choose">反馈与建议</a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/version-1.2.0-blue.svg" alt="Version 1.2.0" />
  <img src="https://img.shields.io/badge/Tauri-v2-24C8D8.svg?logo=tauri&logoColor=white" alt="Tauri v2" />
  <img src="https://img.shields.io/badge/Vue-3.5-4FC08D.svg?logo=vue.js&logoColor=white" alt="Vue 3" />
  <img src="https://img.shields.io/badge/Rust-1.77+-DEA584.svg?logo=rust&logoColor=white" alt="Rust" />
  <img src="https://img.shields.io/badge/License-MIT-green.svg" alt="License MIT" />
  <img src="https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg" alt="Platform" />
</p>

---

## 📖 产品定位与理念

> **Codex writes the code. Halowake handles everything around it.**

**Halowake** 不替代 Codex、Claude Code、OpenCode 或 Google Antigravity 等 Coding Agent，而是围绕现代 AI Coding 工作流，通过**本地处理（Local-first）**、**轻量工具**和**低成本模型**，承接编码之外大量高频、重复的辅助开发工作：

- **Local-first 优先**：会话解析、配置管理、审查数据与进程控制完全在本地运行，数据不出本地设备。
- **Low Token / 低成本**：日报生成 0 Token 磁盘解析，Git AI 提交与轻量审查采用高性价比模型，把昂贵的 Agent Token 留给高价值编码任务。
- **工作流闭环**：打通代码生成后的 Diff 审查、原子提交、多项目开发服务运维与跨工具历史沉淀。

---

## 🌟 真实功能矩阵

### 1. 🔀 Git Assistant —— AI 提交与原子暂存保护

- **基于 Diff 生成 Conventional Commit**：直接分析选中文件或工作区变更，利用低成本模型（DeepSeek、Ollama、OpenAI Compatible）生成结构化 Commit Message。
- **选择性提交原子保护（Atomic Staging Protection）**：仅提交当前勾选的文件，提交成功后自动安全恢复未勾选文件的暂存区状态，杜绝误提交与上下文污染。
- **完整 Git 工作流**：文件变更状态树、Working Tree / Staged / HEAD Diff 对比、分支管理与切换、Git Log 历史与冲突标记。

<!-- 截图占位：Git Assistant 主界面 -->
![Git Assistant 界面](src/assets/git-assistant-preview.png.png)

---

### 2. 🚢 DevDock —— 多项目本地进程看板

- **项目脚本自动发现**：自动扫描项目内 `package.json` scripts 以及 Python、Shell、PowerShell、Cmd 脚本。
- **进程树级终止（Tree Kill）**：强力终止父子衍生进程树，避免后台端口占用与僵尸进程驻留。
- **端口与本地 URL 嗅探**：实时分析服务输出日志，自动提取监听端口与 `localhost` 访问地址，支持浏览器直达。
- **统一高亮日志视窗**：内置 ANSI 彩色日志高亮、日志级别筛选与关键字搜索。

<!-- 截图占位：DevDock 进程管理界面 -->
![DevDock 进程管理界面](src/assets/devdock-preview.png)

---

### 3. 📝 AI Coding Session History —— 跨 Agent 会话聚合与日报生成

自动扫描并聚合本地主流 AI Coding 工具的历史会话：
- 🔵 **Codex CLI**：解析 `~/.codex/sessions/` 本地 JSONL 记录。
- 🟢 **Claude Code**：解析 `~/.claude/projects/` 会话记录。
- 🟣 **Google Antigravity**：读取 `~/.gemini/antigravity/brain/` 轨迹与项目关联。
- 🟠 **OpenCode**：读取本地 Storage 会话。

**核心特性**：
- **0 Token 本地磁盘 IO 解析**：纯本地高速读取与清洗，不产生任何 API Token 消耗。
- **灵活筛选与多项目聚合**：按时间范围（今天/昨天/自定义）、工具来源、项目路径及关键字精准过滤。
- **一键复制到 Web AI**：内置 Morning Standup、周报等 Prompt 模板，一键复制结构化事实至 ChatGPT / Claude / Kimi 等网页模型生成最终日报。

<!-- 截图占位：AI 工作记录与日报生成界面 -->
![AI 工作记录与日报生成界面](src/assets/report-preview.png)

---

### 4. 📊 AI Quota Tracking —— 算力配额与消耗节奏监控

- **多平台余额与限额聚合**：统一监控 DeepSeek、Gemini (Google AI Studio / Pro)、OpenRouter 等 API 账户的额度与健康状态。
- **Pace 节奏评估算法**：依据 5 小时 / 每日配额重置周期，动态计算当前消耗速率（On Pace 安全 / Over Pace 超速），预防限流。

<!-- 截图占位：AI Quota 配额看板 -->
![AI Quota 配额看板](src/assets/ai-quota-preview.png)

---

### 5. 🛡️ Local Code Review —— 本地代码审查引擎

- **确定性规则 + AI 语义诊断**：支持自定义规则库与大模型审查，提供 Strict / Balanced / Fast 多档预算模式。
- **行级缺陷分级**：对代码变更按 `Critical`、`Major`、`Minor`、`Suggestion` 精确定位至源码行。
- **离线 SQLite 持久化**：审查记录保存在本地 SQLite 数据库中，便于随时复盘追踪。

<!-- 截图占位：Local Code Review 界面 -->
![Local Code Review 界面](src/assets/code-review-preview.png)

---

## 📥 安装与运行

### 方式一：下载桌面安装包

前往 [GitHub Releases](https://github.com/zhkai-ybwn/Halowake/releases) 下载对应系统的安装包：
- **Windows**: `.exe` (NSIS 安装包)
- **macOS**: `.dmg` (Apple Silicon & Intel)
- **Linux**: `.deb` / `.AppImage`

### 方式二：从源码编译运行

**环境要求**：
- [Node.js](https://nodejs.org/) (>= 20.0.0)
- [Rust & Cargo](https://www.rust-lang.org/) (>= 1.77.2)
- Git
- [Tauri 2 构建依赖](https://v2.tauri.app/start/prerequisites/)

```bash
# 1. 克隆项目
git clone https://github.com/zhkai-ybwn/Halowake.git
cd Halowake

# 2. 安装前端依赖
npm install

# 3. 启动本地开发调试
npm run tauri:dev

# 4. 构建生产安装包
npm run tauri:build
```

构建产物输出路径：`src-tauri/target/release/bundle/`。

---

## 🔒 隐私与本地优先（Local-First）原则

- 🛡️ **数据零泄漏**：所有配置、Git 历史、审查记录与会话解析均在本地完成，无任何隐式云端上报。
- 🔑 **凭据本地自治**：API Key 仅存储于本地，只在用户发起操作时直连服务商，不经由任何第三方中间服务器。
- ⚙️ **执行边界可控**：DevDock 严格限制在用户主动添加的项目目录内执行脚本。

---

## 🤝 反馈与共建

- 🐛 **提交 Bug / 报错**：[创建 Issue](https://github.com/zhkai-ybwn/Halowake/issues/new/choose)
- 💡 **提出新需求 / 想法**：[提交 Feature 建议](https://github.com/zhkai-ybwn/Halowake/issues/new/choose)
- 💻 **参与代码贡献**：欢迎阅读 [CONTRIBUTING.md](CONTRIBUTING.md) 提交 PR。

---

## 📄 开源协议

本项目采用 [MIT License](LICENSE) 开源。

---

<a id="english"></a>

# Halowake (English)

### A local-first companion for AI coding workflows.

Halowake works alongside Codex, Claude Code, OpenCode and Antigravity, handling routine development work with local processing, lightweight tools and cost-efficient models.

> **Coding Agents handle coding. Halowake handles the workflow around coding.**

---

## 💡 Core Modules

1. **Git Assistant & AI Commit Generation**: Diff analysis, Conventional Commit generation using cost-efficient models (DeepSeek, Ollama, OpenAI compatible), and **atomic staging protection** to commit selected files without altering other staged changes.
2. **DevDock Multi-Project Hub**: Automatically detects project scripts, prevents runaway background processes with tree-level termination, and sniffs active listening ports / localhost URLs.
3. **AI Coding Session History & Zero-Token Standups**: Aggregates sessions from **Codex CLI**, **Claude Code**, **Google Antigravity**, and **OpenCode** using local disk IO (**0 Token Cost**), formatted for 1-click export to web AI models.
4. **AI Quota Tracking & Burn Pace Algorithm**: Aggregates balances across DeepSeek, Gemini, and OpenRouter with real-time **Pace evaluation** (On Pace / Over Pace).
5. **Local Code Review**: Offline deterministic rule checking + AI semantic review stored in local SQLite.

---

## 📥 Quick Start

Download pre-built installers from [GitHub Releases](https://github.com/zhkai-ybwn/Halowake/releases), or build from source:

```bash
git clone https://github.com/zhkai-ybwn/Halowake.git
cd Halowake
npm install
npm run tauri:dev
```

Build production installer: `npm run tauri:build`.

---

## 📄 License

Distributed under the [MIT License](LICENSE).

