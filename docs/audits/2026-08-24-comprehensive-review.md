# Lumina 全面审查报告

审查日期：2026-08-24  
审查范围：当前工作区全部前端与 Tauri/Rust 源码，重点覆盖尚未提交的 787 行新增、187 行删除改造  
审查方式：静态代码审查、Git 变更审查、Impeccable 机械 UI 检测、配置与测试资产盘点  
未执行：浏览器/运行态 UI、Build、Lint、自动化测试（遵循项目 AGENTS.md 的执行边界）

## 1. 结论

**发布结论：有条件阻止发布（Conditional No-Go）。**

当前实现没有发现会让应用必然无法启动的 P0 阻断项，但存在 **6 个 P1、8 个 P2、3 个 P3**。其中，数据保留策略对新增历史记录实际不生效、DevDock 历史抽屉不能及时看到刚完成的任务、旧数据损坏会连带阻断 SQLite 正常数据加载，属于需要在发布前解决的功能与数据一致性问题。键盘焦点管理和 API Key 明文双份持久化也不宜留到发布后。

四个主维度评分：

| 维度 | 得分 | 结论 |
|---|---:|---|
| UI / UX | 72 / 100 | 视觉系统一致，信息密度符合开发者工具定位；无障碍、国际化和破坏性操作反馈仍有缺口 |
| 功能完整性 | 61 / 100 | 主流程已成形，但历史、迁移和保留策略存在可复现的语义断层 |
| 逻辑可靠性 | 58 / 100 | 正常路径清楚；并发、失败回滚、生命周期刷新不足 |
| 代码质量 | 63 / 100 | Rust 层有一定测试与模块化；前端大文件、双数据源和零测试形成明显维护风险 |
| **综合** | **64 / 100** | **可继续内测，不建议按当前状态发布** |

### UI 技术健康度

| 维度 | 得分（0–4） | 关键结论 |
|---|---:|---|
| Accessibility | 2 | 多个模态框关闭焦点约束；模型摘要只支持鼠标点击 |
| Performance | 2 | 首次激活重复全量扫描；多处 `transition: all` 与布局属性动画 |
| Responsive | 3 | 作为桌面应用，1120×720 最小窗口与布局一致；局部弹层有流式宽高 |
| Theming | 4 | 明暗主题、语义色、表面层级和 Naive UI 覆盖较完整 |
| Implementation Integrity | 3 | 产品特异性与设计令牌体系明确；历史双数据源和少量旧组件存在漂移 |
| **总分** | **14 / 20** | **Good：弱项需要定向修复** |

## 2. 发布前必须处理（P1）

### P1-01 数据保留设置对 Git/DevDock 新历史记录不生效

- 类别：功能 / 数据生命周期
- 证据：
  - `src/composables/git-assistant/useGitCommit.ts:77-87` 新 Git 历史固定写入 `expiresAt: null`。
  - `src-tauri/src/commands/project_process.rs:837-852` DevDock 运行历史固定写入 `expires_at: None`。
  - `src-tauri/src/storage/history_repository.rs:139`、`:342` 只删除 `expires_at IS NOT NULL` 且已过期的记录。
  - `src-tauri/src/commands/storage.rs:171-188` 虽然读取保留天数并执行清理，但无法命中上述永久记录。
- 影响：用户配置的 1–3650 天保留期对两类新增历史无效，数据库会持续增长；存储设置 UI 对用户形成错误承诺。
- 建议：保存时由后端统一根据当前 retention days 计算 `expires_at`；迁移旧 `NULL` 数据时明确选择“永久保留”或回填过期时间，并在 UI 中说明策略。
- 验收：把保留期设为 1 天，构造过期记录后强制清理，确认 Git 与 DevDock 历史均被删除且统计准确。

### P1-02 DevDock “最近运行”抽屉存在刷新和计数语义错位

- 类别：功能 / 状态一致性
- 证据：
  - `src/views/devdock/DevDockView.vue:748-753` 运行历史只在显式调用 `refreshRunHistory` 时加载。
  - `src/views/devdock/DevDockView.vue:589-592` 五秒轮询只刷新进程，不刷新历史。
  - `src/views/devdock/DevDockView.vue:51` 打开抽屉仅切换布尔值，没有刷新历史。
  - `src/views/devdock/DevDockView.vue:42` 角标使用 localStorage 的 `recentCommands.length`，而抽屉内容使用 SQLite 的 `runHistory`（`:77-83`）。
  - `startRecentCommand`（`:558-561`）已无调用方，说明旧“最近命令”链路仍残留在新“运行历史”实现中。
- 影响：任务刚完成后打开抽屉可能看不到记录；角标数量与实际列表不一致；用户无法判断记录是否丢失。
- 建议：确定唯一产品语义（最近命令或运行历史），使用单一数据源；在打开抽屉和检测到进程终态时刷新，角标直接绑定同一集合。
- 验收：运行成功、失败、主动停止各一次，不切换路由直接打开抽屉，三条记录与角标必须立即一致。

### P1-03 旧 localStorage 数据异常会阻断已有 SQLite 项目加载

- 类别：逻辑 / 迁移可靠性
- 证据：`src/views/devdock/DevDockView.vue:359-394` 把 JSON 解析、逐条迁移和 SQLite 加载放在同一 `try` 中；任一旧数据解析或单条保存失败都会跳到 `catch` 并把 `projects` 置空。
- 影响：即使 SQLite 中已有正确项目，只要遗留 localStorage 是损坏 JSON，DevDock 每次启动仍显示空列表；原始坏数据没有隔离，问题会持续复现。
- 建议：迁移和主数据加载分开；迁移失败只记录并保留旧数据，仍继续加载 SQLite；逐条迁移、成功标记、幂等重试。
- 验收：写入非法旧 JSON，同时预置 SQLite 项目，启动后 SQLite 项目仍可见且错误可诊断。

### P1-04 模态框焦点约束被系统性关闭

- 类别：UI / Accessibility
- 标准：WCAG 2.4.3 Focus Order、2.1.2 No Keyboard Trap；ARIA Dialog Modal Pattern
- 证据：
  - `src/views/git-assistant/GitAssistantView.vue:264-269`、`:324`、`:391`、`:458`、`:491` 多个模态框设置 `trap-focus=false`，同时常设 `auto-focus=false`。
  - `src/views/codex-report/CodexReportView.vue:337-343` 项目筛选弹层关闭焦点约束。
  - `src/views/devdock/components/DevDockLogModal.vue:2-8` 日志弹层同样关闭。
- 影响：键盘用户可把焦点移动到遮罩后的页面，屏幕阅读器上下文与视觉上下文分离；打开弹层后也没有可靠的初始焦点。
- 建议：恢复模态焦点陷阱与关闭后的焦点归还；将初始焦点放到标题后的首个安全控件，危险确认框默认聚焦取消操作。
- 验收：仅用键盘遍历所有弹层，焦点不得离开弹层，Esc/关闭后回到触发按钮。

### P1-05 API Key 同时以明文写入 SQLite 和 JSON

- 类别：安全 / 凭据存储
- 证据：
  - `src-tauri/src/commands/ai_settings.rs:17-23` 模型配置包含 `api_key`。
  - `src-tauri/src/commands/ai_settings.rs:76-89` 先写数据库，再“兼容性”写一份 JSON。
  - `src-tauri/src/storage/history_repository.rs:391-412` 把整个设置序列化进 `app_ai_settings.settings_json`。
- 影响：一个 API Key 产生两个明文副本，扩大备份、日志采集、误上传和本机其他进程读取的暴露面；用户删除单一文件也不能确认凭据已清除。
- 建议：API Key 使用系统凭据库（Windows Credential Manager/macOS Keychain/Linux Secret Service）；数据库与 JSON 只保存引用或脱敏元数据。若短期无法改造，至少只保留一个受限权限文件并提供明确的“删除凭据”动作。
- 验收：保存、修改、删除模型后，在配置目录和数据库中均不可检索到明文 Key。

### P1-06 前端关键工作流没有自动化测试资产

- 类别：代码质量 / 发布风险
- 证据：项目内没有 `*.test.*` 或 `*.spec.*` 前端测试文件；与之相对，Rust 源码有 59 个 `#[test]`。
- 影响：本次改造涉及迁移、历史、KeepAlive 生命周期、进程轮询和多弹层交互，但没有测试保护；P1-01 至 P1-04 均属于很容易通过单元/组件测试捕获的回归。
- 建议：优先补服务层和组合式函数测试，再补 DevDock 状态机组件测试；不要求一开始追求覆盖率数字，先锁定发布主链路。
- 最小测试集：损坏迁移数据、历史刷新、清理保留期、启动失败、停止/重启、重复点击、模态焦点、双语状态文案。

## 3. 下一迭代应处理（P2）

### P2-01 首次进入 DevDock 会重复执行全量初始化扫描

- 证据：`src/views/devdock/DevDockView.vue:196-210` 的 `onMounted` 和 `onActivated` 都调用全量扫描、进程刷新、历史刷新；该视图位于 `keep-alive`（`src/layouts/MainLayout.vue:43-46`）中，首次挂载同样会激活。
- 影响：项目较多时启动阶段产生重复 IPC 和文件扫描，加载状态互相覆盖，错误也可能被后一次请求改写。
- 建议：挂载负责一次性数据初始化，激活仅刷新时效性数据；或增加初始化 Promise/世代号去重。

### P2-02 项目增删改不是事务式 UI 更新

- 证据：
  - `src/views/devdock/DevDockView.vue:247-255` 先加入 UI 再等待数据库保存，失败时不回滚。
  - `src/views/devdock/DevDockView.vue:349-356` 先从 UI 删除，数据库失败时不恢复。
  - `src/views/devdock/DevDockView.vue:336-338` 别名保存以 `void` 触发，用户看不到失败状态。
- 影响：当前会话看似成功，重启后数据“恢复/消失”，形成幽灵状态。
- 建议：采用保存成功后提交 UI，或乐观更新+失败回滚；破坏性操作失败必须显式提示。

### P2-03 历史迁移可能产生重复项，且同一会话失败后不重试

- 证据：`src/services/git/git-history-service.ts:41-70` 在迁移开始前把 `migrated=true`；无 id 项使用随机 id。中途失败会保留 localStorage，但本会话不重试；下次启动会给已成功迁移的无 id 项生成新 id。
- 影响：部分数据迁移后再失败，会导致重复历史；用户无法区分哪条是原始记录。
- 建议：使用内容指纹生成确定性 id，逐条记录迁移状态，全部成功后再删除旧数据。

### P2-04 进程上限没有真正限制运行中进程，并存在重复启动竞态

- 证据：
  - `src-tauri/src/commands/project_process.rs:732-744` 只有在超过 40 个时清理已结束项；若全部在运行，则继续保留并允许后续启动。
  - `find_active_script` 检查（`:493-509`）与新进程插入（`:470-474`）不在同一临界区，并发 IPC 可同时通过检查。
- 影响：重复进程可能争抢端口；大量长期进程会突破预期资源上限。
- 建议：在后端以“项目+命令”键做原子占位，明确运行中上限及错误文案。

### P2-05 新增运行历史没有国际化

- 证据：`src/views/devdock/components/DevDockRecentDrawer.vue:78-107` 把成功、失败、已结束、运行中以及时间单位直接写成中文/英文混合文本。
- 影响：英文界面仍出现中文状态；`code:`、`ms/s/m` 的格式不随语言或区域变化。
- 建议：状态文案进入 i18n；日期时间使用 `Intl.DateTimeFormat`，持续时间使用统一格式器。

### P2-06 清空全部运行历史没有确认与成功反馈

- 证据：`src/views/devdock/components/DevDockRecentDrawer.vue:14-16` 单击即发出清空；`src/views/devdock/DevDockView.vue:768-774` 直接删除全部记录，失败也只内部报告。
- 影响：误触不可恢复，用户无法确认操作是否成功。
- 建议：增加明确范围的确认（“清空全部项目的 N 条记录”），成功 toast，失败保持列表并显示错误。

### P2-07 模型摘要只能鼠标展开

- 类别：Accessibility
- 证据：`src/components/settings/panels/ModelSettingsPanel.vue:18-36` 使用无 role/tabindex/键盘事件的 `div.model-summary` 承担展开操作。
- 影响：键盘和部分辅助技术用户无法打开模型设置。
- 建议：使用原生 `<button>` 或为折叠标题采用标准 disclosure 模式，并提供 `aria-expanded`、`aria-controls`。

### P2-08 关键文件体积过大，职责耦合明显

- 证据：`GitAssistantView.vue` 2528 行、`CodexReportView.vue` 1596 行、`project_process.rs` 1320 行、`DevDockProjectList.vue` 940 行、`DevDockView.vue` 891 行；后端 `git/runner.rs` 2599 行、`git/prompt.rs` 2200 行。
- 影响：UI、状态、IPC、迁移、轮询和格式化逻辑难以独立验证；小改动容易跨区域回归。
- 建议：按工作流边界拆分状态控制器/服务/纯格式化函数；不要仅按视觉区块机械拆组件。

## 4. 可排期优化（P3）

### P3-01 动效策略对减少动态效果的处理过于粗暴

- 证据：`src/styles/workbench/index.scss:104-112` 全局把所有动画和过渡压到 `0.01ms`；令牌层同时又把时长设为 0。
- 影响：满足“减少运动”意图，但会消除部分有助理解状态变化的反馈，并形成重复规则。
- 建议：保留非空间位移的必要反馈，分别处理旋转 loading、弹层入场和布局位移。

### P3-02 机械检测发现 9 个候选动效/风格项，仅部分成立

- 成立项：弹层 spring easing 与“克制、专业”品牌略有冲突；`transition: all` 难以预测；Git Review 最大化直接动画 width/height 会触发布局。
- 误报/可接受项：Quota/Storage 的进度条宽度动画表达真实进度；SplitHandle 宽度变化范围小；侧栏选中边线属于旧组件且当前主布局已使用另一实现。
- 建议：只处理已确认的弹层、侧栏和 Review 面板，不应为消除检测器告警而重写合理进度动画。

### P3-03 变更中有两个空白格式问题

- 证据：`git diff --check` 报告 `src-tauri/src/storage/database.rs:53` 和 `src/services/project/project-service.ts:219` 文件尾多余空行。
- 影响：不影响功能，但会制造无意义 diff。

## 5. UI / UX 评估

### 做得好的部分

- 产品定位清晰：桌面开发者工作台采用较高信息密度，没有落入营销页或 AI 炫技界面。
- 明暗主题体系完整：表面、文本、分隔、状态色、代码区和阴影均使用语义令牌；Naive UI 也有对应覆盖。
- 桌面响应边界一致：Tauri 最小窗口 1120×720 与主布局约束一致，避免出现声明可缩放但内容必然裁切的窗口尺寸。
- 关键动作多数使用原生按钮，焦点样式在工作台组件中已有可复用实现。
- 状态不仅依赖颜色：DevDock 历史有文字 badge，进程状态也有文案。

### 仍需人工页面验证

由于本次按规则未启动浏览器，以下项目不能从源码 alone 做最终判定：

1. 1120×720、1280×820、4K/150% 缩放下是否出现内容裁切、不可见滚动条或弹层溢出。
2. 明暗主题下正文、次级文本、危险色、diff 背景的实际对比度。
3. 长项目名、深层 Windows 路径、超长命令和 500 行日志时的截断与滚动表现。
4. 中文、英文切换后导航、按钮、抽屉标题是否抖动或溢出。
5. 全键盘完成：添加项目、配置命令、运行/停止、Git 选择与提交、查看/关闭弹层。
6. 减少动态效果开启时 loading 和状态切换是否仍然清楚。

## 6. 逻辑与功能评估

### 做得好的部分

- SQLite migration 每个版本使用独立事务并在成功后更新 `user_version`，避免半套 schema。
- 命令执行使用 `std::process::Command` 参数化构造，没有把用户输入拼接成 shell 字符串。
- 进程日志限制为 500 行，结束进程会写入状态和退出码；应用退出流程会先尝试停止全部托管进程。
- 项目目录新增了磁盘根目录、系统目录与用户主目录保护；Python 和脚本发现有明确白名单。
- Tauri capability 权限较克制，生产 CSP 禁止内联脚本；日志 HTML 渲染启用了 `ansi_up.escape_html=true`。
- Markdown 预览在送入 `marked` 前对原始 HTML 做了转义，降低了直接标签注入风险；仍建议后续限制链接协议。

### 主要系统性问题

- **双数据源未收口**：DevDock 同时保留 recentCommands/localStorage 与 runHistory/SQLite，职责交叠。
- **失败路径弱于成功路径**：多处错误只写内部日志，UI 先改后存且不回滚。
- **生命周期逻辑分散**：mount、activate、轮询、弹层打开分别刷新不同状态，导致新鲜度不一致。
- **保留策略没有下沉到写入层**：前端/进程线程自行构造 expires 字段，容易绕过全局存储设置。

## 7. 代码质量与测试评估

### 优点

- 前后端类型名称与 camelCase 序列化基本一致，服务层封装了 Tauri invoke。
- Rust 层已有 59 个单元测试，新增 migration/history repository 也包含基础测试。
- 错误文案多数携带上下文，数据库 SQL 使用参数绑定。
- 设计令牌和工作台基础组件已经形成可复用层。

### 缺口

- 前端没有自动化测试，无法保护 composable、store、迁移与生命周期逻辑。
- 过大的 SFC 和 Rust 文件把多个变更原因绑定在一起。
- 多个异步调用使用 `void`，虽然部分内部 catch，但调用方无法据此回滚或反馈。
- 新旧路径并存后没有删除无调用函数和旧计数语义，增加认知负担。

## 8. 建议执行顺序

1. **数据正确性**：修复 expires_at 写入、历史刷新、旧数据迁移隔离。
2. **交互可达性**：恢复模态焦点管理，替换模型摘要点击 div。
3. **凭据安全**：收口 API Key 存储，明确迁移/删除策略。
4. **状态一致性**：合并 DevDock 最近命令与运行历史，给增删改加回滚。
5. **测试保护**：先覆盖上述四组缺陷，再做文件拆分。
6. **UI 完成度**：补 i18n、清空确认、动效与人工多尺寸/双主题验证。

## 9. 建议的发布门槛

发布前至少应满足：

- 6 个 P1 全部关闭并有对应测试或明确的人工复现记录。
- DevDock 成功、失败、停止、重启、迁移、清理六条链路通过。
- Git 提交历史保存/恢复/清理通过，失败提交是否入历史的产品语义已确认。
- 全键盘主流程通过，所有模态焦点可预测。
- API Key 不再出现两个明文副本。
- 用户明确执行 Build/Lint/测试后无阻断错误；本报告不替代这些运行验证。

## 10. 审查限制

- 本报告是静态审查，不声称已验证运行时渲染、真实文件系统差异、平台进程树行为或网络模型调用。
- 未执行 Build/Lint，因此潜在类型错误、未使用符号和样式规则错误仍需后续命令确认。
- 未执行测试，因此报告中的测试数量是资产盘点结果，不代表现有 Rust 测试当前全部通过。
- 机械 UI 检测只作为候选线索，已在报告中区分成立项与误报，不以告警数量直接等同质量。
