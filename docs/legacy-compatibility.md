# Halowake 遗留兼容性规范 (Legacy Compatibility Guide)

本文档明确记录了 **Halowake**（原 Lumina）在品牌统一过程中，为了保障老用户数据完整性、平滑自动升级与现有项目向下兼容而刻意保留的 **Legacy Identifiers** 及其设计原因。

---

## 1. 核心原则 (Core Principle)

> **用户可见、仓库可见、发布可见全部统一为 Halowake；只有底层兼容性原因才允许保留 Lumina。**

在品牌变更中，强行修改底层数据路径、包标识符或已有配置目录会导致用户历史记录丢失、自动更新中断或破坏团队协作配置。因此，系统严格区分“对外品牌”与“底层兼容标识符”。

---

## 2. 保留的 Legacy Identifiers 清单与保留原因

| 标识符 / 路径 | 所属层次 | 保留原因与兼容性影响 |
| :--- | :--- | :--- |
| **`com.yubei.lumina`** | Tauri App Identifier (`tauri.conf.json`) | **应用数据目录寻址与自动更新核心**<br>• 决定了操作系统级应用数据目录（Windows: `%APPDATA%\com.yubei.lumina`；macOS: `~/Library/Application Support/com.yubei.lumina`；Linux: `~/.config/com.yubei.lumina`）。<br>• 决定了 Windows NSIS 安装注册表键与卸载链路。<br>• 保证 Tauri 增量自动升级（Tauri Updater）能够无缝覆盖安装，防止升级后用户本地数据目录被重置为空白。 |
| **`lumina.db`** (及 `-wal`, `-shm`) | SQLite 数据库文件 (`src-tauri/src/storage/database.rs`) | **本地离线数据持久化核心**<br>• 存储了用户的 Git 提交历史、Local Code Review 审查记录、DevDock 运行日志与执行历史、AI Quota 账户与配额缓存、Codex 日报模板及本地存储清理偏好。<br>• 保留此文件名可确保现有用户升级到新版本后，原有历史记录与配置即时可用，避免数据断裂。 |
| **`.lumina/` 目录** | 用户项目配置与 Git 规则 (`.lumina/project.json`, `.lumina/git-profile.json`) | **工作区向下兼容与团队共享**<br>• 用户已在本地或团队代码仓库中创建了 `.lumina/project.json`（DevDock 项目命令、环境变量、Python 解释器配置）与 `.lumina/git-profile.json`（Git AI Commit 提示词规则）。<br>• 保留 `.lumina/` 路径读取与写入，现有工程无需做任何迁移即可正常启动与审查。 |
| **`lumina://request-exit`** | Tauri IPC 事件协议 (`src-tauri/src/lib.rs`, `MainLayout.vue`) | **主进程与前端受控退出协议**<br>• 托盘“退出并停止全部进程”通过该内部 IPC 事件通知前端优雅终止全部受管后台服务并清理进程树。<br>• 内部协议无需对外暴露，保留以避免事件名版本不匹配。 |
| **`lumina.*`** | Webview `localStorage` 本地偏好键 | **用户界面偏好连续性**<br>• 包含 `lumina.preferences.locale`（语言）、`lumina.preferences.themeMode`（主题）、`lumina.preferences.closeAction`（关闭窗口行为）、`lumina.git.repoPath`（最近仓库）等。<br>• 保留这些 Key 可防止用户在升级桌面端后界面语言或主题被重置。 |
| **`--lumina-*`** | 前端 CSS Token 命名空间 (`src/styles/tokens/`) | **内部样式系统命名**<br>• 内部设计系统变量（如 `--lumina-primary`, `--lumina-surface-secondary`）不具有外部品牌曝光属性，保留可避免全量样式重构带来的潜在视觉回归。 |

---

## 3. 已完成统一的对外品牌 (Unified Branding)

以下所有用户可见、仓库可见与发布可见的命名均已完全切换为 **Halowake**：

- **桌面端交互**：系统托盘右键菜单（“显示 Halowake”）、托盘悬停 Tooltip（`Halowake`）、主窗口标题、更新弹窗、关于面板。
- **发布与分发**：`package.json` package name（`"name": "halowake"`）、`Cargo.toml` package name（`name = "halowake"`）、`tauri.conf.json` productName / mainBinaryName（`Halowake`）。
- **工程文档**：`README.md`（中英双语）、`PRODUCT.md`、`CONTRIBUTING.md`、GitHub Releases / Issues 链接（`zhkai-ybwn/Halowake`）。
- **后端运行时信息**：Git 规则配置相关的错误提示文案、测试用例临时目录前缀（`halowake-profile-test-*`、`halowake_test_*`）。

---

## 4. 后续开发指导 (Guidelines for Contributors)

1. **新功能文案**：所有面向用户的新 UI、日志、文档一律使用 `Halowake`。
2. **数据迁移原则**：若未来必须更改 `com.yubei.lumina` 或 `lumina.db`，必须提供全自动、零停机、无感知的跨目录数据迁移与双向兼容层，并在版本发布说明中明确记录迁移机制。
