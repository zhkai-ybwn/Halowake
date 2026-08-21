# Git 工具 macOS UI 全量整改设计

## 目标

在不改动 Git 业务数据流的前提下，将 Git 主工作台、所有弹窗/抽屉、Code Review、命令反馈以及独立 Git Log/Diff 窗口统一为原生 macOS 工具风格。关注度分数仍是本地文件关注优先级，不作为代码质量分。

## 设计决策

- 主窗口继续采用工具栏 + 三栏可调整工作区，内容优先，不增加网页式卡片堆叠。
- 模态交互统一使用 macOS Sheet：12px 圆角、0.5px 描边、分层阴影、清晰标题栏、底部操作区。
- Prompt 和历史保留右侧 Inspector/Drawer，使用 vibrancy 与滑入动画保持上下文。
- Git 命令执行使用独立操作 Sheet，日志区域采用 SF Mono，状态和进度提供即时反馈。
- 空状态统一为图标、主文案、辅助文案和单一 CTA；无内容时隐藏无意义工具。
- 控件高度遵循 28/34px 两档，正文 13px，说明 11–12px，间距以 8px 网格组织。
- 关闭图标统一使用简洁 `x`，不再使用 filled-circle close 图标。
- 所有浮层支持 Esc；主要确认操作保留 Enter 语义，危险操作与主操作分离。
- Light/Dark 继续使用 Lumina 主题 token，不直接反色；浮层使用带饱和度的背景模糊。

## 覆盖清单

1. GitStatusBar 顶部工具栏与仓库、分支、同步、视图操作。
2. GitChangeExplorer 搜索、过滤、批量选择、评分进度、表格、右键菜单和空状态。
3. GitDiffViewer 文件信息、模式切换、Diff 内容与空状态。
4. GitCommitAssistant 表单、选择提示和提交操作。
5. 最近仓库管理 Sheet。
6. 分支选择 Sheet。
7. 冲突处理 Sheet。
8. 合并分支 Sheet。
9. Clone/Init 仓库 Sheet。
10. Code Review 报告、历史、规则、进度和 Finding 卡片。
11. Prompt Inspector 与提交历史 Inspector。
12. GitCommandDialog 命令进度和输出。
13. 独立 Git Log 窗口。
14. 独立 Git Diff 窗口及 UnifiedDiffViewer。

## Review 性能与错误处理

- Compact 总输入不超过单批 24k 字符，避免重复固定 Prompt 和串行网络等待。
- Standard/Deep 使用 24k 字符上限分批，保留局部失败结果。
- OpenAI-compatible 响应先读取原始字节，再解析 JSON；错误包含 HTTP 状态和响应摘要。
- 响应体传输失败允许一次受控重试，请求使用 `Accept-Encoding: identity` 和短连接规避代理压缩/复用异常。
- 限制 AI 输出 Token，保持结构化 findings 数量上限。

## 验收

- 所有清单界面均使用同一层级、圆角、边框、阴影、关闭方式和按钮语义。
- 主界面切换功能不丢评分、勾选或 Review 状态。
- Compact Review 正常情况下只产生一个 AI batch。
- Review 网络/响应错误可显示具体状态或响应摘要，不再只有 `error decoding response body`。
- 原关注度函数保持原始权重和计算逻辑，解释明细不参与最终分数计算。
