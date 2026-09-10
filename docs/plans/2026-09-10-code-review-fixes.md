# Code Review Fixes Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 修复 2026-09-10 代码审查发现的配额真实性、账号持久化、异步反馈、Review 取消和 DevDock 状态一致性问题。

**Architecture:** 保持现有 Vue/Pinia/Tauri 分层，不引入新的持久化系统。配额适配器只在存在可信数值时创建额度条目；本地安装或登录信息仅用于账号说明。数据库增加显式初始化标记，从语义上区分“首次使用”和“用户主动保存空列表”。

**Tech Stack:** Vue 3、TypeScript、Pinia、Tauri 2、Rust、rusqlite。

---

### Task 1: 修复配额数据模型和汇总口径

**Files:**
- Modify: `src-tauri/src/quota/manager.rs`
- Modify: `src/stores/quota.ts`
- Modify: `src/components/quota/TopbarQuotaPill.vue`
- Test: `src-tauri/src/quota/manager.rs`

**Steps:**
1. 提取“可汇总积分”判断函数，仅聚合单位为空或明确为积分/credits/points 的额度。
2. 让前端单账号刷新采用同一判断，避免全量刷新与单卡刷新后汇总口径不同。
3. Topbar 对不同单位分别显示，不再统一拼接“点”。
4. 添加 Rust 单元测试覆盖积分与 Tokens/请求次数不可混加。

### Task 2: 移除适配器中的推测额度

**Files:**
- Modify: `src-tauri/src/quota/adapters/cursor.rs`
- Modify: `src-tauri/src/quota/adapters/qcode.rs`
- Modify: `src-tauri/src/quota/adapters/trae.rs`
- Modify: `src-tauri/src/quota/adapters/zcode.rs`
- Modify: `src-tauri/src/quota/adapters/opencode.rs`

**Steps:**
1. 修复 Cursor 临时字符串引用。
2. 远端额度请求失败时不再生成默认满额。
3. Qcode、Trae、OpenCode 本地状态不再生成 1 项特权、模型数或通道数额度。
4. Z-Code 不再根据缓存文件硬编码 300 万 Tokens；远端只在返回实际 remaining 字段时展示剩余量。
5. 删除所有没有数据依据的固定配速百分比。

### Task 3: 修复账号初始化、发现与配置 UI

**Files:**
- Modify: `src-tauri/src/storage/migrations.rs`
- Modify: `src-tauri/src/storage/history_repository/quota_repository.rs`
- Modify: `src-tauri/src/quota/manager.rs`
- Modify: `src-tauri/src/quota/discovery.rs`
- Modify: `src/components/quota/AccountEditModal.vue`
- Modify: `src/components/quota/AccountManageModal.vue`
- Modify: `src/views/quota/AiQuotaView.vue`

**Steps:**
1. 在 SQLite 元数据中记录配额账号配置已初始化，空列表仍视为有效配置。
2. Gemini 只在发现本地配置证据时返回。
3. 补齐 Cursor、Qcode、Trae、Zcode 的手动添加、CLI 提示和管理页名称。
4. 刷新函数返回真实结果；发现流程在保存或刷新失败时回滚占位卡并显示准确反馈。
5. 账号删除、启停保存失败时恢复旧状态；删除增加确认。

### Task 4: 修复 Review 与 DevDock 状态一致性

**Files:**
- Modify: `src-tauri/src/commands/review.rs`
- Modify: `src-tauri/src/review/repository.rs`
- Modify: `src/views/devdock/DevDockView.vue`

**Steps:**
1. 增加只更新 Review 取消状态的仓储函数，保留已有 findings、overview 和 token 用量。
2. DevDock 添加失败时移除乐观项目，删除失败时恢复项目，别名保存失败时恢复旧名称并提示。
3. 添加相应 Rust 仓储测试。

### Task 5: 凭据存储风险收口与验证

**Files:**
- Modify: `src-tauri/src/storage/history_repository/quota_repository.rs`
- Modify: `src-tauri/src/commands/ai_settings.rs`
- Modify: `README.md` or security-facing UI copy only if required

**Steps:**
1. 在不引入跨平台凭据依赖的前提下，限制数据库文件访问权限并避免生成额外明文副本。
2. 明确当前 SQLite 仍是本机同用户可读的兼容方案；不伪装为系统凭据库。
3. 运行 `cargo fmt --check`、`cargo test`、`npm run build`、`npm run lint` 和 `git diff --check`。
4. 不执行浏览器或自动化视觉测试；列出需要人工验证的关键交互。
