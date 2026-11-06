# Performance and review fixes Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 修复代码审查的 3 个缺陷，并落实性能报告的 6 项优化。

**Architecture:** 项目筛选用 null 表示全部、数组表示显式选择，兼容旧的空数组偏好。后端为 JSONL 建立有容量限制的文件解析缓存，按大小、修改时间失效并合并并发读取；OpenCode 的失败来源作为警告返回。前端按需加载、约束页面轮询并虚拟化大 Diff。

**Tech Stack:** Vue 3, TypeScript, Vite, Vitest, Rust, Tauri 2, rusqlite.

---

### [x] Task 1: 日报筛选与选择性能
- 修改 types/codex-report、CodexReportView、FilterStrip、ProjectFilterModal、SessionList。
- null=所有项目，[]=零项目；读取旧偏好时迁移空数组，持久化带版本的结构。
- 建立 computed Set，过滤与条目渲染避免重复线性查找。
- 增加取消全选、反选、取消最后一项、父页面过滤回归用例。

### [x] Task 2: 后端记录与故障隔离
- 修改 commands/codex_report.rs、codex_report/opencode.rs。
- 统一 Claude 压缩后缀；增补子代理去重测试。
- OpenCode 按文件/数据库隔离失败，IPC 返回正常数据与警告，前端显示可重试提示。
- 测试正常与损坏数据库共存及其他来源仍可用。

### [x] Task 3: 历史解析缓存
- 新增 codex_report/history_cache.rs；共享三个 JSONL 解析器和元信息查询的数据读取。
- 缓存相关 JSON 记录，按文件元数据失效；约束内存容量与文件数量；单文件并发读取合并。
- 仍发现所有历史目录，避免日期/mtime 剪枝导致遗漏；检测追加、截断、删除与替换。
- 对快速筛选请求防抖，避免提交无用查询。
- 测试重复读取复用、单文件变化、并发与容量行为。

### [x] Task 4: 启动与 Markdown
- 拆出 markdown 文件 IPC 服务；业务路由和弹窗按需加载。
- 高亮使用 common 语言集，未知语言保持纯文本回退；Mermaid 首次有图表时加载。
- 主界面挂载后加载 AI 配置，AI 相关调用继续等待初始化完成。
- 生产构建比较入口体积，类型检查及相关测试。

### [x] Task 5: 轮询生命周期与日志
- 新增可复用的页面可见轮询 composable，激活/停用及 visibilitychange 都受控，保证请求不重叠。
- Markdown 主/独立窗口与 DevDock 接入；日志版本不变时避免重新传输及渲染。
- 用假计时器验证停用、隐藏、恢复及慢请求。

### [x] Task 6: 大 Diff 与进程停止
- UnifiedDiffViewer 使用固定行高窗口，仅渲染可见行及缓冲行；定位条使用滚动比例，移除全量 DOM 扫描。
- 内联大 Diff 复用虚拟化查看器，小 Diff 保留 diff2html。
- 进程停止/重启命令同步段移入 spawn_blocking。
- 验证大样本渲染行数有界、末行可访问、进程相关 Rust 测试。

### [x] Task 7: 完整验证与交付
- npm run build、相关测试后运行完整前端测试；Rust 类型检查与受影响测试。
- 检查最终差异并完成回归审查；记录构建对比和手动验证项。
- 不控制浏览器，不提交 Git，保留用户原有改动。
