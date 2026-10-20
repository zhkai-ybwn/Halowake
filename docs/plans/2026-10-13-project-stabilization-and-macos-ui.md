# Halowake 稳定化与 macOS UI 优化实施计划（Gemini 编码交接版）

**交接对象：Gemini 编码模型。** 按本文任务顺序实现；不要一轮重写全项目。当前任务是优化现有应用，不是创建新产品或引入新的技能平台。

**Goal：** 让技能身份、安装目标和操作结果准确可核验，消除主要数据保护风险，再把 Skills 及共享界面收敛为符合 macos-design 的桌面工具。

**Architecture：** 保留 Vue 3 + Pinia + Tauri 2 + Rust + SQLite；建立来源身份/安装实例/安装计划/执行结果四个边界。搜索由一个查询契约负责过滤、排序和分页；UI 复用 workbench 组件与语义主题令牌。按功能边界逐步抽取现有逻辑，不先建设大型新框架。

**Tech Stack：** TypeScript、Vue 3、Pinia、Naive UI、SCSS、Tauri 2、Rust、rusqlite/SQLite FTS5。新增前端单元测试工具时选择与项目 Node 20.19.3/Vite 7 兼容的版本并更新 lockfile。

**日期与根目录：** 2026-10-13；`C:/Users/user/Desktop/templates/Lumina`。下文代码路径均相对此根目录，行号是本次评审定位，不是要求机械套用的补丁位置。

---

## 0. 给 Gemini 的执行约束

1. 先读取本文件、`docs/audits/2026-10-13-comprehensive-review.md`、当前 AGENTS.md，以及 `C:/Users/user/.codex/skills/macos-design/SKILL.md` 和它的三个 references。
2. 工作区已有大量未提交改动。先看 `git status` 和相关 diff；不 reset、不覆盖现有工作、不自动把全部文件加入提交。提交或 PR 由用户另行决定。
3. 本文授权范围是既有缺陷修复、必要的模型整理、主题和交互收敛。不要新增 Agent、推荐算法、技能评分体系、在线服务、下载渠道或 Skills 本地导入产品。
4. 页面与 UI 默认由用户手动验证。没有本次明确授权，不启动/控制浏览器，不运行 Playwright/Puppeteer/Cypress，不自动截图或进行视觉验证。可以运行单元测试、类型/lint、Rust 测试和构建。
5. 不在用户真实技能目录做更新、删除、崩溃恢复试验；使用临时 fixture。不要修改用户 API Key、真实历史、真实项目或本机 Agent 配置。
6. 先修 P1，再修状态/查询，再调整 UI。每项先建立能失败的业务回归案例，实现最小修复，再运行针对性检查；不为纯样式机械添加镜像测试。
7. 不通过降低门槛、删除负例、扩大同义词范围或放宽身份判定来“修好测试”。测试数据/标注有误时须给出证据并记录修改原因。
8. 修改时同步 TypeScript 类型、IPC 参数、Rust serde 与 UI 消费者；不得用 `as unknown as` 隐藏新的领域不匹配。
9. macos-design 的示例是参考。文件安装/覆盖/删除必须在真实结果已知后提示成功；系统保留快捷键不能被劫持；不为了形式达标添加无用途的拖拽、引导动画或全局热键。
10. 每批完成后报告：改了哪些用例、测试结果、遗留项、用户手动验收步骤。遇到环境缺失说明限制，保持可审查源码，不声称未做的验证通过。

## 1. 第二轮复核结论

### 1.1 上轮 18 项发现仍适用，部分证据得到加强

| 原编号 | 二次结论 | 本文任务 |
| --- | --- | --- |
| F01 合集子技能去重 | 同 catalog ID + repo 忽略 subPath 的分支仍存在 | T02 |
| F02 未受管更新覆盖 | 普通更新仍可按名称推断来源并删除备份 | T03 |
| F03 同步假成功 | 缓存写入异常仍被吞掉，内存与 SQLite 可不同版本 | T05 |
| F04 流水线假完成 | 隔离执行真实 store 再次复现 | T04 |
| F05 搜索验收未达标 | 重跑仍为 8 通过/1 失败，Strict P@1=69.2% | T07 |
| F06 截断后类别筛选 | category:null + limit:500 链路仍存在 | T06 |
| F07 自定义分类 | 再次复现标签命中却返回 0；身份键不同 | T06 |
| F08 已安装误判 | 再次复现不同仓库、相同 catalog ID 返回 true | T02 |
| F09 排序不传递 | 再次复现 A<B、B<C、C<A | T06 |
| F10 更新未知状态/超时 | 静态确认，未发起真实远端更新 | T03 |
| F11 项目上下文/草稿 | Vue computed 语义与浅复制共享数组再次复现 | T04 |
| F12 键盘/焦点 | 安装 div、display:none checkbox、关闭焦点陷阱仍存在 | T10 |
| F13 更新/卸载并发 | 卸载未获取同一操作锁；实际破坏性并发未执行 | T03 |
| F14 测试/CI | 前端业务测试入口缺失，CI 只跑 Rust --lib | T01、T07、T16 |
| F15 职责/首载 | 大文件、多套搜索/过滤、主页面静态导入仍存在 | T06、T08、T15 |
| F16 DevDock 启动竞态 | 检查与 spawn/登记分离仍存在，标为静态风险 | T13 |
| F17 日报请求竞态 | 晚响应仍可覆盖当前查询，标为静态风险 | T14 |
| F18 文案/承诺/文档 | 未审查数据与“经工程验证”等承诺不一致；硬编码仍存在 | T11、T16 |

重新执行：`npm run lint` 通过；Rust `--lib` 118 项通过；搜索集成 8 项通过、1 项失败。前端打包沿用上一轮同源码构建结果，不把它冒充第二次构建。

源码复现脚本：`docs/audits/evidence/2026-10-13-source-checks.mjs`。它只读取源码，真实加载 Vue/Pinia/TypeScript，替换网络/IPC；不持久化，不启动浏览器。执行：

```powershell
node docs/audits/evidence/2026-10-13-source-checks.mjs
```

脚本输出当前缺陷，不是“测试通过”证明；源码结构改变后把案例移入正常测试，不要求继续保留 regex 提取。

### 1.2 macos-design 遵守情况

结论：**视觉骨架已建立，原生交互和渐进呈现只部分遵守；不能宣布完全符合。** 没有本次界面截图，以下是源码证据，非视觉验收结论。

| 编号 | Skill 要求 | 当前实现与判断 | 动作 |
| --- | --- | --- | --- |
| U01 布局/窗口 | ~50px 顶栏、200–260px 侧栏、可折叠、内容为主 | 50px/224px/折叠56px 已有；主窗口平台按钮分开处理 | 保留；Skills 内部重复窗口壳和工具栏要简化 |
| U02 主题令牌 | 层级清楚、薄分隔、统一颜色和控件 | 有令牌，但源码 9 个引用名无定义，多项无 fallback | T09 清理，不再堆更多别名 |
| U03 系统外观 | 深浅独立设计，跟随系统改变 | 配色已独立；系统媒体事件未更新响应式依赖，getter 缓存不失效 | T09 修复；实际源码隔离执行确认 |
| U04 可读性 | 12–14px 正文、弱文本有层级、可辨认 | 多处主要说明11/11.5px，代表色对比不足 | T09/T11；实际渲染仍需手动核验 |
| U05 材质/阴影 | blur 用于侧栏/工具栏/浮层，正文保持不透明 | 共享 Sheet/Drawer 整体 blur24px，Skills 主表面混透明 | T09/T10 将模糊局限于头部/边缘材质 |
| U06 内容/空态 | 内容优先，空态隐藏无用筛选 | Skills 有空态但工具栏/阶段/排序先展示，空页认知负担高 | T08/T11 分离空目录与无匹配 |
| U07 搜索 | 显眼、Cmd/Ctrl+F、清除和结果数 | 已有搜索与提示；筛选/截断/回退状态不统一 | 保留入口，按 T06 修契约 |
| U08 快捷键 | 主操作可键盘完成，提示与行为一致 | 全局 Cmd/Ctrl+K、设置、搜索已有；Skills 主要动作提示/绑定不完整 | T10；不要抢系统保留组合 |
| U09 选择/焦点 | 原生控件、明确选中、上下文保持 | article/div 点击详情/选择；hidden checkbox；Modal/Drawer trap关闭 | T10 修复语义和焦点，选择不靠鼠标 |
| U10 反馈 | 即时反馈、错误回滚、短确认 | 有 toast/进度；失败、skipped、partial 显示成功 | T04/T05；文件操作不得乐观假成功 |
| U11 拖入拖出 | 使用原生 DnD，反馈与预览 | Markdown 仅 HTML drop + File.path 断言，未发现原生绑定或拖出 | T17；Skills 不因此增加新导入功能 |
| U12 初次引导 | 简短、教1–2个操作 | 有空态；未发现完整首用引导 | T11 可做轻量 inline 提示；单模态欢迎作为后置可选项 |
| U13 图标/关闭 | 单线图标、currentColor、×关闭 | 主体使用 Solar；共享关闭×符合；Skills局部 circle/filled混用 | T11 统一操作图标；供应商品牌图保留 |
| U14 动效 | 状态变化有反馈，克制、尊重 reduced motion | motion令牌和reduced-motion已有；个别 motion令牌不存在 | T09；只动画transform/opacity/颜色等必要属性 |
| U15 系统集成 | 原生窗口/系统外观尽可能复用 | Tauri无装饰窗口+自绘traffic lights；没有失焦灰态；强调色固定 | T12 补焦点/平台行为；系统accent后置，不强制新增插件 |

Skill 入口的 input radius=4px 与 sizing 表=6px 不完全一致。采用项目已有输入/按钮6px、卡片8px、面板12px作为统一基准，不为了1–2px重写控件。

### 1.3 新增确定证据

- **主题跟随系统失效（U03）：** 加载真实 preferences store，初始 light；mock media 改为 dark并触发已注册的 change callback，resolvedTheme 和 DOM data-theme 仍为 light。
- **令牌未定义（U02）：** `--lumina-fill-quaternary`（17处无fallback）、`--lumina-fill-secondary`（2）、`--lumina-fill-tertiary`（5）、`--lumina-surface-base`（8）、`--lumina-font-title`（1）、`--lumina-motion-fast`（4）、`--lumina-shadow-xs`（4）；另有 `--lumina-accent` 12处，其中1处无fallback，以及 `--lumina-on-primary` 1处带fallback。统计为源码扫描，不包括依赖库私有变量。
- **代表色对比（U04）：** `#8e8e93/#fff`=3.26:1；`#8e8e93/#f5f5f7`=2.99:1；`#77777d/#222224`=3.57:1；App.vue Naive 深色主按钮 `#fff/#4fa397`=2.99:1。workbench 深色主按钮 `#10201e/#64b5aa`=7.00:1。最终值需依据真实背景混色验证，不能把token计算等同于所有位置的截图对比。

## 2. 本轮产品和 UI 范围决策

### 2.1 保留与收敛

- 保留 Git、DevDock、日报、额度、Markdown 现有业务主流程；本轮只修具体缺陷。
- 保留来源元数据、内容 hash、锁、暂存/备份、SQLite事务、FTS基础，禁止以重写丢弃保护。
- Skills 默认“已安装”；“探索”和“安装记录”保留为同级分页入口。
- 本轮不增加流水线/评分/Agent。合集与流水线在 P1 结果语义修复后才开放完整批量能力。
- 评分和认证降为详情中的来源信息；无证据的“安全认证”“经工程验证”“完全相关”不展示为质量保证。
- 一个列表默认展示名称、简短描述、目标或来源摘要、状态和一个主要操作；其余进入详情/更多菜单。

### 2.2 Skills 工作台目标结构

迁为 `/skills` 路由，顶栏 Skills 按钮导航到该路由（保留更新数量）；命令面板提供入口。外层不再有关闭/最大化/双击“窗口”标题栏。右侧详情保持列表上下文；安装是单个受控 Sheet。

```text
主应用原有系统标题栏和侧栏（保留）
┌ Skills   已安装 | 探索 | 安装记录              当前项目/范围 ┐
│ 搜索……                 范围筛选     排序     更多         │
│ 内容列表                           可选详情面板          │
│  名称 · 目标 · 来源状态              来源/使用说明/实例     │
│ 状态栏：结果数 / 索引状态 / 后台操作                       │
└─────────────────────────────────────────────────────────┘
安装 Sheet：来源、明确目标、实际目录、操作进度、逐项结果
```

不新增 Skills 自己的第二个完整侧栏。列表不强制固定36px：一行列表36–44px，含描述的两行列表56–68px。28px控件、34px主要确认、12–16px内边距。

### 2.3 重要语义

“已发现”只说明磁盘有目录；“已安装到 Codex”说明目标实例存在；“可更新”需要已确认来源；“已被 Agent 读取”需要真实集成证据，本轮不宣称自动激活。

共享 `.agents/skills` 不等于所有 Agent 通用读取；标题明确为“共享目录”，附简短兼容提示。扫描支持与安装/更新/卸载支持各自列出，不能用一个 Agent 列表混为一谈。

## 3. 数据契约与不变量

以下是目标契约草案，可按现有命名调整，但含义不可省略。优先在 `src/types/skills.ts` 及 Rust 对应结构演进，不把所有文件一次性迁移。

### 3.1 来源身份与安装实例分开

```ts
type SourceIdentity =
  | { kind: 'git'; repoUrl: string; subPath: string }
  | { kind: 'unknown' }

interface SkillInstallation {
  installationKey: string        // 后端给出的规范化目标目录身份
  skillKey?: string              // 来源明确时存在
  catalogIds: string[]           // 关联，不作为唯一事实来源
  canonicalName: string
  source: SourceIdentity
  path: string
  target: { mode: 'project' | 'specific_agent' | 'global'; agent?: string; projectPath?: string }
  management: 'halowake' | 'external' | 'unmanaged'
  dirty: boolean | null          // null = 没有基线，不能推断未修改
}
```

- `skillKey` 由规范化 repoUrl + subPath 生成稳定键；canonicalName 是目录命名，不足以证明相同来源。
- GitHub owner/repo 可按既有规则规范化、去 `.git`/末尾斜杠；subPath 保持大小写，不能统一小写（Git路径大小写敏感）。拒绝绝对路径、`..` 和越出repo的链接。
- source未知时 name/id仅能用于候选提示，不能返回确认已安装、接管更新或覆盖。
- 同来源不同subPath是不同安装单元；合集catalog ID属于父条目。
- 路径规范化由后端按操作系统规则处理；不把Unix/macOS所有路径都小写。
- 旧catalog ID目录保留原位、按已确认元数据映射。迁移不能自动改名或移动真实目录。
- 标签/收藏迁移先解析到稳定技能键；匹配不唯一时保留旧数据并提示，不静默关联第一个同名技能。

### 3.2 安装计划先于执行

```ts
type ItemStatus = 'installed' | 'skipped' | 'failed' | 'missing' | 'cancelled'
interface InstallationItemResult {
  itemKey: string                // 安装单元 + 目标
  skillKey?: string
  targetPath?: string
  status: ItemStatus
  errorCode?: string
  messageKey?: string
}
interface InstallationBatchResult {
  operationId: string
  status: 'completed' | 'partial' | 'failed' | 'cancelled'
  requested: number
  installed: number
  skipped: number
  failed: number
  missing: number
  cancelled: number
  items: InstallationItemResult[]
}
```

- 每个“单元×目标”是一条结果；`requested` 等于逐项状态计数总和。多Agent不能只用“技能数”表达目标完成率。
- 预检缺流水线成员默认阻止执行并列出 missing；不静默缩短计划。
- skipped计数与installed分开；全部已存在时 installed=0，并显示“无需新增”。
- 验证/计划错误可以在执行前返回错误；执行中出现部分写入时必须返回结构化明细，不能只throw一条字符串丢失已完成路径。
- 每个目标原子替换；批量允许partial，不冒充跨所有目录的全事务。
- 进度含operationId/itemKey/phase；关闭界面不得清空仍在运行的执行状态。
- 来源和最终canonicalName可能需clone后确定：确认前若只能显示根目录就标“名称将在读取SKILL.md后确定”，不能拿catalog ID伪装最终路径；读取后展示真实计划。

### 3.3 搜索和目录版本

```ts
interface CatalogQuery {
  query: string
  stage?: string
  category?: string
  allowedIds?: string[]
  sort: 'relevance' | 'popular' | 'name'
  offset: number
  pageSize: number
  catalogRevision?: string
}
// 响应至少包含 hits、totalMatched、hasMore、catalogRevision、
// matchTier、engineStatus、elapsedMs；匹配解释返回字段/词/状态码，前端本地化。
```

过滤 → 候选检索/排名 → 排序 → 分页；同一过滤集计算total。允许明确标注上限，但不能把Top-K数量称为全目录总量。浏览无query也使用明确的一套排序；popular不混入质量分。搜索版本与已发布目录版本一致。

## 4. 实施任务与依赖

### T01 前端业务测试基础与基准保存（P1，先做）

**文件：** 修改 `package.json`、`package-lock.json`；新增 `vitest.config.ts`、`src/test/setup.ts`。新增下述各任务对应的 `*.test.ts`。

1. 记录已有工作区状态和检查结果，不把已有文件变更当成本任务成果。
2. 安装兼容版本的 Vitest、Vue Test Utils；涉及DOM用例可使用happy-dom/jsdom，仅模拟DOM，禁止自动启动真实浏览器。
3. 配置 `@` alias，mock Tauri invoke/channel、文件选择器、网络和localStorage；所有测试不得写真实配置。
4. 添加 `test:unit` 命令，配置固定fixture与单测清理；不要在正常CI导入用户本机历史/目录数据库。
5. 将证据脚本的四个Skills案例和主题事件案例转成业务测试，先证明当前失败。

**验收：** 测试入口能单独运行；模拟调用完整、无网络；修复后的断言依赖业务结果，避免检查内部实现行数/函数名。

### T02 身份与合集去重（P1，依赖T01）

**修改：** `src/types/skills.ts`、`src/stores/skill-explore.ts`、`src/services/skills/skills-service.ts`、`src-tauri/src/commands/skills.rs`。按复用需要新增 `src/services/skills/skill-identity.ts`；Rust先在既有模块提取纯函数，不要求立刻拆整文件。

1. 定义source/installation不变量，扫描结果补subPath和管理状态。
2. 修复find_existing的2a分支和旧catalog候选特殊分支：不能因为catalog ID一致跳过repo/subPath核验。
3. 按目标返回安装实例匹配；目录卡“已安装”可显示在哪些目标，不禁用向其他目标安装。
4. 旧来源不完整时标unknown，维持只读，不通过名称补“确定来源”。
5. 标签/收藏键演进提供幂等兼容读；不删除旧存储直到迁移确实成功。

**测试：** 同repo同catalog ID不同subPath安装A/B；同name异源冲突；旧catalog目录正确去重；A目标存在、B目标仍可装；unknown不判确认匹配；大小写敏感subPath；目录遍历拒绝。

**验收：** A/B均生成正确实例，重复A只skip A；冲突不修改现有文件；旧目录保持原位。

### T03 更新/卸载保护、状态与超时（P1，依赖T02）

**修改：** `src-tauri/src/commands/skills.rs`、`src/types/skills.ts`、`src/stores/skills.ts`、`src/components/skills/installed/InstalledSkillsView.vue`、`src/components/skills/SkillsManagerModal.vue`（迁路由后对应新view）。共享Git执行超时修改放 `src-tauri/src/git/runner/process.rs`，保持非网络命令既有行为。

1. 默认只自动更新halowake受管、来源确认的技能；外部安装器管理与unmanaged不自动接管。
2. 无hash基线的dirty=null；普通更新明确拒绝。不按name/LIKE LIMIT1选择上游。
3. 强制覆盖受管dirty文件必须有明确操作范围和可恢复备份；不要成功即删除唯一恢复副本。已有Git stash路径检查失败，不忽略stash错误后hard reset。
4. 卸载和更新用同一installation锁；锁内重新验证目标与链接状态，防止检查和操作脱节。
5. scan/check/update/uninstall回写共享scope generation；旧项目结果不得覆盖当前项目。
6. 检查结果区分update_available/up_to_date/unknown/error/unsupported。离线不报全部最新。
7. 网络Git/npm命令有有界超时；默认建议单远端15–30秒，在设置或内部常量中集中管理，超时终止本任务子进程并释放锁。不要把截止时间等同于成功。
8. 同repo检查复用结果，限制并发；“仓库有新提交”与“子技能内容变化”区分。此轮不要求新增复杂内容diff算法。

**测试：** unmanaged含自定义文件普通更新拒绝；dirty受管需明确force；update/uninstall并发只允许一个获锁；超时/离线返回unknown/error；旧scope不回写；失败恢复fixture保留原内容。

**验收：** 不丢用户文件，未知状态可见，所有修改同一目标操作互斥。真实目录禁止用作回归试验。

### T04 安装计划、部分结果与项目上下文（P1，依赖T02）

**修改：** `src/stores/skill-explore.ts`、`src/types/skills.ts`、`src/services/skills/skills-service.ts`、`src/components/skills/explore/SkillInstallModal.vue`、`src-tauri/src/commands/skills.rs`、安装历史view。新增轻量 `src/stores/workspace.ts` 或复用实际已有响应式项目store；将Git选择项目的设置位置一并接入。

1. 当前项目由一个响应式来源维护，localStorage仅用于启动恢复；扫描/安装捕获scope快照。
2. 向导草稿深复制需要编辑的数组，不共享targetAgents；取消不改偏好。customProjectPath展示和执行必须相同。
3. 普通/子技能/合集/流水线先生成完整计划、检查missing，再执行；去掉三套重复统计路径。
4. 后端返回逐目标结果，前端新增数由installed状态计算，历史写入相同operationId/itemKey。
5. 批量partial停留结果页，列失败项和已完成目标，可重试失败项；全skip显示无需新增。
6. 结果/进度状态独立于弹层，关闭视图不reset执行状态；若不支持取消则明确显示进行中并约束关闭，不提供假取消按钮。
7. 保存偏好只在用户确认并选择记住后发生；不会因草稿修改提前改变store。

**测试：** 全skip=installed0；缺成员被阻止；一半失败保留明细；多Agent部分完成保留路径；A→B项目展示/执行B；取消草稿不改变store；晚progress不污染另一operation。

**验收：** 页面安装数、路径、历史、磁盘事实一致。

### T05 目录校验、缓存发布与同步反馈（P1，依赖T01）

**修改：** `src/services/skills/skill-catalog-service.ts`、`registry-cache.ts`、`src/stores/skill-explore.ts`、`src-tauri/src/storage/skill_registry_repository.rs`、`src-tauri/src/commands/skill_registry.rs`。

1. 远端JSON在运行时校验id/name/gitUrl/subPath、tags数组、author对象、stage、数值范围及可选字段；不要靠TS断言。
2. 缺少可选展示字段可以规范化为默认值；缺少安装必要来源字段必须标不可安装或拒绝条目。明确选择整批拒绝/隔离坏行策略并返回诊断。
3. SQLite事务成功后才发布catalogRevision、内存目录和lastSyncTime。持久化失败保留上一个完整版本。
4. 下载和持久化分别表达；失败与cache fallback返回状态枚举，调用方按状态使用warning/error，不统一success toast。
5. 一次读取snapshot同时得到skills/pipelines/revision；初始化按需进行，避免分别全量加载。
6. 流水线引用不完整时可保留条目显示“当前目录不完整”，不得当完整可执行套餐。

**测试：** mock落库失败不更新版本/同步时间；畸形author/tags不进page；事务失败旧目录和FTS一致；下载成功但写失败不能提示同步成功；空缓存离线有明确状态。

### T06 查询、分类、标签与稳定排序（P2，依赖T02/T05）

**修改：** `src/types/skills.ts`、`src/stores/skill-explore.ts`、`src/stores/skills.ts`、`src/utils/skill-synonyms.ts`、`src/components/skills/explore/SkillExploreView.vue`、`installed/InstalledSkillsView.vue`、`src-tauri/src/storage/skill_registry_repository.rs`。

1. 类别和排序进入查询契约，过滤在截断前执行；明确total/hasMore与catalogRevision。
2. popular按明确热度字段降序，name按稳定排序，relevance由后端结果排序。相等时skillKey/id稳定兜底。
3. 移除差值≥5切换比较规则，保证传递性；不要在前端再次混入认证/指南分重排后端相关度。
4. 本地Installed过滤用联合类型：all / updates / agent(value) / stage(value) / tag(value)，各分支互斥。切换维度重置不可用值。
5. 统一标签键与身份映射；不再用字符串同时表示阶段和用户标签，避免标签名“coding”等发生冲突。
6. 后端不可用时保留简单可解释的本地词面匹配，展示fallback；不维护第二套持续扩张的自然语言排序。
7. 每次查询/目录/过滤/分页变化带请求世代；清词/换scope/旧响应均不会回写最新结果。需要时保持旧结果并标加载，不混用旧查询徽章。

**测试：** 第501名目标类别仍召回；category+stage+starred组合；有标签→匹配1；标签名与stage相同仍正确；A/B/C比较器无循环；同集合不同输入序列结果一致；多请求乱序；语言切换标签可读。

### T07 搜索质量和CI（P2，依赖T06）

**修改：** `.github/workflows/ci.yml`、`src-tauri/tests/skill_search_poc.rs`；新增小型fixture `src-tauri/tests/fixtures/skill_registry.json` 和生产搜索回归测试文件。

1. 将独立PoC算法、生产固定回归集、本机真实目录评测分开。固定测试必须调用生产入口。
2. 固定fixture覆盖确切名称、中文、混合词、多约束、泛化词、负例、过滤前截断。
3. 保留黄金集/留出集区别；不要对18/26失败简单降低88%门槛。当前小样本的标签也应人工复核。
4. 真实数据评测缺数据库时标清跳过，不能计入“通过质量门槛”；CI不能依赖开发者APPDATA。
5. release模式单独测cold/warm耗时，不把本轮debug P95当发布承诺；先记录基准再设可复现预算。
6. CI加入前端unit、生产build和确定性Rust integration；后续跨平台矩阵按依赖安装成本配置。

**验收：** 新CI会捕获本次具体排序/过滤回归；搜索质量门槛明确，失败不能静默遗漏。

### T08 Skills工作台路由与懒加载（P2，依赖P1完成）

**修改：** `src/router/index.ts`、`src/layouts/MainLayout.vue`、`src/components/skills/TopbarSkillsPill.vue`、`src/components/skills/SkillsManagerModal.vue`。新增 `src/views/skills/SkillsView.vue`；复用现有installed/explore/history/detail/install子组件。

1. 先把Skills主体提成view，再让顶栏和命令面板统一导航；路由按tab query或内部持久状态恢复，不保留两套管理入口。
2. 删除内层窗口最大化/关闭/双击标题栏行为；保留应用真正窗口行为。
3. 目录store不在顶栏初始化；首次进入Skills才读缓存，需要时同步。更新pill可用轻量安装摘要，不为pill加载全部目录。
4. 主重型页面改动态import，保持现有KeepAlive预期；避免一次迁移改变Git/DevDock生命周期。
5. Details保持当前列表上下文；小窗口时详情可为drawer，大窗口可并列，不新增第二级sidebar。

**验收：** 顶栏入口、命令面板、回到原模块均正常；后台操作不因路由切换丢失；初始chunk不再静态包含全部Skills工作台（依据构建依赖链核对）。

### T09 主题、令牌、字号和材质（P2，可与业务收敛分批做）

**修改：** `src/stores/preferences.ts`、`src/composables/useTheme.ts`、`src/App.vue`、`src/styles/tokens/_color-system.scss`、`themes/_light.scss`、`themes/_dark.scss`、`base/_naive.scss`、`workbench/index.scss`、共享Sheet/Drawer，以及使用未定义令牌的组件。

1. store增加响应式systemTheme，在media change回调更新；resolvedTheme只依赖响应式state。显式light/dark不跟系统切换，system实时切换。
2. 对9个未定义令牌逐项迁移：surface-base→content/window语义；fill-*选择control-bg或surface语义；font-title→font-display；motion-fast→duration-fast；shadow-xs→shadow-sm。accent/on-primary改现有primary/on-accent。不得仅全局新增9个同义变量糊住问题。
3. App.vue Naive主色/文字色与主题令牌统一；深色primary不要再配固定白字。使Naive控件和Workbench控件同theme、同on-accent语义。
4. 正文/主要说明13px，次级信息12px，必要caption11–11.5px；搜索结果的任务描述不再当10px元数据。密集代码/log可保留等宽11–12px并提供适当缩放。
5. 可读文字在实际背景≥4.5:1；大字和非文字操作边界≥3:1，禁用态单独处理。tertiary用于辅助/装饰时可更弱，但不能用于必须阅读的说明。
6. 内容背景solid；blur/vibrancy留titlebar/sidebar/toolbar/popover。Sheet/Drawer正文保持不透明；分层阴影复用已有token。
7. 保持现有品牌teal，macos-design不要求把产品全部改蓝；不使用全局反色filter。
8. 统一motion token，reduced motion维持状态反馈；不对布局宽高和所有CSS属性统一transition:all。

**测试：** system light→dark→light同时更新getter/DOM/Naive；显式light/dark不变化；重复初始化无重复监听；静态令牌扫描无未定义且无fallback的项目变量；代表文字配对对比值达标。

**手动验收：** 两主题、系统切换、125%/150%缩放、中文/英文长文本；不得以脚本色值计算宣称截图全部达标。

### T10 键盘、焦点、安装Sheet与快捷键（P2，依赖T04/T08）

**修改：** Skills row/card/detail/install、`WorkbenchSheet.vue`、`WorkbenchDrawer.vue`、`WorkbenchCloseButton.vue`、`src/utils/platform-shortcuts.ts`、快捷键settings；必要时新增轻量焦点/快捷键composable。

1. 三种安装目标用原生radio group；子技能/Agent使用可聚焦checkbox（视觉隐藏而非display:none，或Naive原语）。label与输入关联，focus-visible落到可见元素。
2. row详情入口使用独立button或完整键盘语义；避免button嵌button。选择、详情和安装动作语义分开。
3. Modal真实模态开启trap/初始焦点/关闭返回触发者；非模态并列详情不要声明aria-modal。Drawer若带遮罩则按模态管理，不关闭trap冒充非模态。
4. 同时只一个活动确认Sheet；危险确认默认聚焦取消，展示准确path/影响目标/dirty状态。
5. Enter提交只在该表单合法且非执行中生效；不在任意输入里全局拦截Enter。Esc只关闭最上层可关闭交互；执行中遵循T04状态策略。
6. Cmd/Ctrl+F聚焦当前内容搜索，Cmd/Ctrl+K命令面板，Cmd/Ctrl+,设置；主“重新扫描”可用当前Skills范围Cmd/Ctrl+R并展示提示。更新全部/卸载不设无确认危险快捷键。
7. 所有shortcut展示从同一映射生成，settings补Skills和现有上下文说明。不要注册Cmd+Space/Cmd+Tab系统保留组合。
8. tab使用标准tablist、aria-selected、tabpanel关联和方向键行为；或改普通导航button，不保留不完整tab语义。

**测试：** 模拟DOM下radio/checkbox状态、disabled防重复、初始/返回焦点、Esc只影响顶层、输入中快捷键边界；实际native键盘遍历交给用户。

### T11 渐进呈现、信息密度与文案（P2，依赖T04/T06/T08）

**修改：** Skills相关view、`WorkbenchEmptyState.vue`、i18n两语言、`GitReviewPanel.vue`、相关Settings panels。

1. 空目录：标题、简单说明、单一同步/添加项目入口；隐藏阶段、排序、分页等无用控件。无匹配：保留搜索/当前条件和清除按钮，不能隐藏找回结果的入口。
2. 默认列表优先名称/任务描述/目标/状态；作者、路径、标签、仓库stars移至第二行摘要或详情，避免各自变成常驻彩色pill。
3. 分类和高级排序放紧凑菜单；一个视图一个主要动作。安装结果必须直接可见，不能只靠2秒toast。
4. 认证、score、社区来源表达可核验信息；未有证据时使用“目录条目/社区索引”，删除硬编码数量与未经验证承诺。exact语义改“满足当前检索约束”，不等于技能能力已验证。
5. 进度和失败返回code/params，前端i18n；补search fallback、空态、安装计数和GitReview硬编码。后端原始错误可放详情，不作为全部用户文案。
6. 操作图标统一Solar线性16/20px与×关闭；状态/品牌可有必要填充。共同buttons复用Workbench，不复制同一长CSS。
7. 首次使用显示1–2条可关闭inline快捷键提示，记住dismissed；不增加多步欢迎向导。可选首次modal后置，不能挡住紧急操作。

**验收：** 中英文状态一致、计数真实、空态只有可用操作；主流程不要求先理解SDLC/套餐/评分。

### T12 窗口chrome与平台行为（P2/P3，依赖T09）

**修改：** `src/layouts/MainLayout.vue`、`src-tauri/tauri.conf.json`（仅必要时）、共享窗口工具。

1. 保留Windows/Linux平台按钮，不给它们强制显示macOStraffic lights。
2. macOS traffic lights接入窗口focused状态，失焦为灰；异步监听unlisten妥善清理。已安装Tauri API有 `getCurrentWindow().isFocused()` 与 `onFocusChanged()`。
3. 标题栏留连续可拖区域，按钮/输入/触发器不触发drag/doubleclick maximize；点击拖拽手动验收。
4. 对decorations:false与可用原生titlebar方案做小范围平台核验；没有macOS环境不能声称原生按钮/最大化与fullscreen语义全部符合。不要仅凭CSS把maximize当成Mac fullscreen。
5. 系统accent读取属于后置集成项：本轮保留品牌accent和系统appearance，不为形式合规新增未经确认插件/权限。

**验收：** 平台正确的窗口按钮、聚焦样式和拖拽；没有环境的项目明确待验。

### T13 DevDock原子启动与进程展示（P2）

**修改：** `src-tauri/src/commands/project_process/process_runtime.rs`、`project_process.rs`、必要的DevDock进程view。

1. 以project+command key在锁内注册starting，spawn前占位，失败撤销；同key并发只能产生一个child。
2. 不在持有全局processes锁时进行慢IO；单key状态/锁负责生命周期。保留进程tree终止和job对象。
3. 列表不以“只显示最新同key”掩盖仍存活的重复实例；正常UI可分组，但数据必须保留可诊断。
4. 必要的运行上限明确计算starting/running，不能只prune已结束项冒充限制。

**测试：** 临时测试命令10次并发启动只产生一个PID；spawn失败可重试；不同key可并发；停止与history只写正确一次。严禁在真实项目上测。

### T14 日报、审查和额度请求生命周期（P2）

**修改：** `src/views/codex-report/CodexReportView.vue`、`src/stores/review.ts`、`src/stores/quota.ts`；范围限请求状态，不重写各服务商adapter。

1. 日报loadSessions捕获日期/provider/scope快照和generation，晚响应不覆盖新筛选；loading只由最新请求结束。
2. 审查refreshActive/open捕获sessionId，打开新session后旧响应不得覆盖；保留running任务和历史。
3. 额度全量/单账户刷新防止旧快照覆盖新响应或复活已删除账户；处理silent失败，summary与同一quotas版本一致。
4. 抽取小型generation工具仅在第二处实际复用时进行；不要为此新增通用复杂task engine。

**测试：** mock可控Promise按B→A乱序返回，UI最终保持B；删除账户期间旧刷新返回不复活；loading不会被过期请求错误清除。

### T15 职责拆分与加载成本（P2/P3，业务回归通过后）

**修改/可新增：** Skills服务与store；`src-tauri/src/commands/skills/identity.rs`、`installer.rs`、`updates.rs`、`history.rs`（按需）；`storage/skill_registry_repository`中的query/tokenize/ranking职责。

1. 先把T02/T04/T06里的纯逻辑抽成独立用例，UI只处理展示、表单和导航。
2. Rust commands保持IPC适配层；文件安全/事务逻辑与目录搜索不交叉。路径校验/lock/hash只有一套真实实现。
3. 不以“文件≤300行”作为硬指标；禁止仅为行数拆出循环依赖或同名巨大utils。
4. 删除已无调用的旧state/词典/别名仅在rg查证和回归通过后；既有兼容数据要保留可迁移读。
5. 用同构建方式比较入口chunk和依赖图。修静态导入/重复读取，不靠提高chunkSizeWarningLimit隐藏警告。

**验收：** 行为测试无退化、依赖更清楚；初始入口体积变化有前后数字；首次进入Skills与返回不重复全量扫描/初始化。

### T16 发布门槛、文档与跨平台凭据说明（P2/P3）

**修改：** `.github/workflows/ci.yml`、`release.yml`、`README.md`、`docs/legacy-compatibility.md`、本计划实施记录；必要时补存储说明，不在本轮自造跨平台加密。

1. 发布前显式完成或复用测试/类型/lint/build/确定性Rust测试门槛；tag构建不能绕过失败的必要验证。
2. 版本徽章、功能矩阵、Skills目标兼容范围与实际版本同步；注明共享目录不保证Agent自动读取。
3. 本地Key：Windows DPAPI已实现，macOS/Linux当前为受限权限文件/DB内原值，不称跨平台加密。系统凭据库属于后续独立方案，设计/环境验证后再实现。
4. 处理6个EOF空白问题，只format当前任务涉及文件，避免全项目无关格式改动。
5. 每阶段写已完成/未完成/测试/手动待验，不更新“全面验收通过”直到实际完成。

### T17 原生Markdown拖入与导出能力边界（P3，最后做）

**修改：** `MarkdownPreviewView.vue`、`MarkdownDocumentViewer.vue`、markdown service、Tauri相关capabilities仅必要项。

1. 使用已安装API中 `getCurrentWebview().onDragDropEvent()` 或window等价事件读取native payload.paths，不依赖浏览器File.path断言。
2. 处理enter/over/leave/drop状态、路径类型、多个文件选择规则与错误提示；显示将打开的文件，不自动覆盖/写出。
3. 在组件KeepAlive生命周期中只让当前活动surface消费drop，清理unlisten；独立preview window也有明确归属。
4. 保留文件选择器和已有复制/保存出口，确保非拖拽等价操作。真正拖出到Finder需要受支持原生能力，现有API未证明支持；没有环境/依赖时标待实施，不用HTML draggable假装已实现。
5. 不增加Skills本地安装入口，不为了Skill的拖拽条目扩大包管理范围。

**验收：** native拖入Markdown有高亮/成功打开/非法文件反馈；真正拖出未完成时在能力矩阵诚实标部分支持。

## 5. 执行批次与退出条件

| 批次 | 任务 | 退出条件 |
| --- | --- | --- |
| A 数据安全与真实结果 | T01–T05 | 子技能不误去重、普通更新不覆写未知目录、缓存发布一致、partial/skipped准确 |
| B 查询与质量 | T06–T07 | 类别不丢召回、标签正确、排序稳定、生产回归集进入CI |
| C macOS工作台 | T08–T12 | 路由工作台、系统theme正确、令牌闭合、键盘可完成操作、文案真实；视觉待用户验收 |
| D 其他模块与维护 | T13–T16 | 并发/晚响应回归通过、职责拆分、发布门槛和文档一致 |
| E 原生文件交互 | T17 | 支持范围实际核验，拖出缺口诚实保留 |

批次A失败不能通过批次C的视觉改版掩盖。批次C可先修主题和可读性，不必等待整个职责拆分完成。

## 6. 检查命令与预期

常规检查应从项目根目录运行，使用已有依赖；下面 `test:unit` 在T01添加后才可使用：

```powershell
npm run test:unit
npm run lint
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo test --manifest-path src-tauri/Cargo.toml --test skill_registry_search
git diff --check
```

`skill_registry_search` 是T07建议新增的确定性integration文件名，实施前不存在，不应在交接时伪称已通过。`npm run build` 的prebuild会更新图标产物，检查其diff；只查当前Vite打包可用 `npm exec vite build`，但不能把它当完整prebuild验证。

现有真实目录评测按需运行：

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --test skill_search_poc
```

当前预期是1项失败（69.2%对88%）；完成T07后按已记录fixture/质量门槛复核，禁止删除门槛让它静默通过。性能评测用release并记录机器/数据/模式，避免比较两次debug波动。

每个任务先跑对应业务测试；相关检查通过后再在批次末做一次完整检查，避免没有新变化时反复全量跑。Rust fmt/clippy作为新增或改动Rust模块的质量检查，不能为通过它们把大量无关文件自动format。

## 7. 用户手动UI验收矩阵

未经用户本次明确授权，Gemini只交付下面清单，不自行开启浏览器。

| 场景 | 操作 | 预期 |
| --- | --- | --- |
| 深浅/系统主题 | system模式切OS light/dark；再设置固定light | CSS和Naive同步；固定light不受OS变化影响 |
| macOS窗口 | 切到另一app再切回、拖标题栏、点三个按钮 | 正确focused灰态、可拖空白区、平台行为明确 |
| Windows窗口 | 最小化/最大化/关闭、顶栏搜索 | Windows按钮保留，控件不成为拖区 |
| 窄窗口/缩放 | 1120×720，125%/150%，长中英文路径 | 主操作与错误可见，正文可滚动，无悬浮按钮遮挡 |
| 无目录离线 | 清空临时缓存fixture后离线打开探索 | 只显示有用说明和同步/重试；没有虚假推荐数量 |
| 无搜索命中 | 输入负例查询、选择类别 | 条件可见，可清除；不宣称已验证噪声被“完全拦截” |
| 项目切换 | Git A→B，再开Skills安装 | 展示B并最终落到B；无旧项目误写 |
| 同合集A/B | 安装A和B，再执行相同计划 | 两者存在；再次installed=0/skipped正确 |
| 部分失败 | 临时fixture一个目标拒绝写入 | 已完成目标保留，失败明细可读，可只重试失败项 |
| 键盘 | Tab/Shift+Tab、radio方向键、checkbox空格、Esc、Cmd/Ctrl+F | 全流程可完成，最上层焦点和返回正确 |
| 更新/卸载 | 手工目录、dirty受管、外部管理目录 | 可操作状态准确，无静默覆盖，无假最新 |
| 语言切换 | 中文↔英文，再看安装/搜索/审查 | 无固定中文业务消息，计数和单位一致 |
| 原生拖入 | 从文件管理器拖Markdown、非法类型、多个文件 | 反馈/选择规则明确，不依赖File.path |

## 8. Gemini交付格式与完成定义

每批提供：用例变化、修改文件、测试实际输出摘要、未完成项、手动验收清单。最终不能只写“已优化UI”或“已重构”；要逐项标T01–T17和F01–F18/U01–U15的状态。

“完成”要求：已修任务的回归通过；旧数据能读且不静默丢失；所有成功提示有真实结果；代码/IPC契约一致；没有新增未定义令牌；浏览器/系统未验项明确列出。外部服务适配和真正native拖出未实测时保留待验，不虚报。

## 9. 可直接复制给Gemini的起始指令

> 请在当前Halowake项目执行 `docs/plans/2026-10-13-project-stabilization-and-macos-ui.md`，先读取该文档、关联评审与macos-design Skill。当前有未提交工作，保留它们。先只实现批次A（T01–T05），按任务建立业务回归并完成最小修复；不要同时重写UI或增加推荐/Agent/评分功能。所有文件操作测试用临时fixture，禁止在真实skills目录做破坏性验证。未经我本次明确同意，不启动或控制浏览器、不跑浏览器或自动视觉测试。完成批次A后报告修改、测试、遗留项和手动验收，再按本文依赖顺序继续后续批次。不要通过削弱测试门槛或放宽来源匹配消除失败。

这段起始指令建议分批使用：它明确第一批范围，避免模型把完整文档误当一轮无限扩张的重构许可。
