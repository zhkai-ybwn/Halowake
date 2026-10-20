# Antigravity 发布准备结果复核

## 第四轮跟进：最新结论（2026-10-19）

对象：用户提供的 `59c0a196-f9d1-488b-8182-c9065470bf73/已粘贴的文本.txt` 与本轮源码。**仍不可正式发布，但上一轮三个具体复现问题已修复，不再要求重做。** 当前工程补修应集中在 R02 的快速切换目标和错误恢复；搜索标签裁决、正式签名/更新及人工验收另列，不应混为代码已完成的证明。

### 独立验证

| 检查 | 本轮结果 |
| --- | --- |
| `npm run test:unit` | 39 passed，5 文件，退出 0 |
| `npm run lint` | 退出 0 |
| Rust lib（单线程） | 131 passed，65.61 秒，退出 0 |
| `skill_registry_search` | 11 passed，退出 0；Rust incremental 目录访问警告不影响测试结果 |
| 真实目录生产入口 test_9 | 21/26=80.8%，退出 101，53.13 秒，仍低于 88% |
| `git diff --check` | 退出 0 |
| 两项当前分页补充探针 | 2 failed：加载中跳页目标丢失；失败后缺少可见重试入口 |
| 新安装包/EXE元数据与SHA-256 | 与本轮执行报告一致；未运行、安装或验证更新 |

本轮未重复前端 build 或重打安装包；分别报告独立运行结果和执行者提供的构建记录，不将后者冒充本轮独立构建。本次仅更新文档/证据，临时测试已从发现目录移除，未启动浏览器或发布。

### 本轮可关闭的具体修复

- R03-A：缓存/同步使用统一 revision 格式，已有首次正常搜索 first-hit 回归通过；原“裸版本与带时间戳格式不一致”复现已关闭。
- R03-B：生产 FTS 与 fallback 入口显式 BEGIN DEFERRED，版本读取错误传播；并发发布时保持旧读取快照的回归通过。原“同连接但无事务”复现已关闭。
- R02：单次跳第 92 页确实渲染 16 条的回归通过，原单次追加容量不足已修复。
- R01：上一轮认可的本地 verify/needs/draft 配置保留，远端运行仍需单独验收。
- R05：本轮文件已刷新，不再是上一轮旧哈希。NSIS 为 8,200,025 字节，SHA-256=C5B942712471B0CB2651CCCDB95384455D430424104B1BF0FDF7D735C4A74402；EXE 为 27,286,528 字节，SHA-256=0FCDEAC2400281207BA6C8C746B8F821BA841BB3A11C52322EED1B3ED93463E7。可作为本轮离线功能验收产物；完整 tauri:build 仍报告缺更新私钥，不能称正式签名/更新链路通过。

### R02-A [P2] 加载中改跳另一页，新目标仍被直接忽略

位置：`src/stores/skill-explore.ts:403–409` 与 `src/components/skills/explore/SkillExploreView.vue:431–459`。

loadMoreSearchResults 首句在 isLoadingMore=true 时直接 return false。新的 targetCount 没有记录，也没有递增 loadMoreRequestSeq 使旧任务失效。现有循环只满足首次调用目标。

独立 DOM/IPC 复现：2,200条结果、已加载500条；跳第22页时挂起 offset=500/limit=28 的响应；期间改跳第92页，再让第一请求成功。实际仅发出那一次请求，最终 loaded=528、currentPage=92、rows=0、isLoadingMore=false。按已交办“快速改目标不能丢失”的要求，应继续到新目标并渲染最后16条。

修复范围：合并/更新当前待满足目标，或对新目标明确取消并接管旧任务；任务完成后必须满足最新页面，不能仅靠currentPage的单次watch。新增受控延迟响应测试，不只测试一次快速成功的跳页。

### R02-B [P2] 报告声称有错误提示与重试，实际视图未消费状态

位置：`src/stores/skill-explore.ts:491–500,1567–1568`；`SkillExploreView.vue`。

store 已提供 isLoadingMore 与 loadMoreError，但视图没有使用这两个字段，也没有对应的错误提示/重试动作。执行报告所称“UI 联动分页与错误重试”与当前源码不符。

独立复现：后续页 IPC 抛 pagination-offline-probe，store.loadMoreError 正确记录，界面没有任何“重试/retry”按钮，用户停在空白的第22页。没有因为旧有错误横幅或其他页面按钮获得重试路径。

修复范围：将现有状态接入分页视图，明确加载、失败及针对当前目标的重试动作；使用现有 i18n。补失败→重试→正确渲染的行为断言，无需浏览器自动化或视觉重构。

### 搜索标签与发布边界

报告已经完整列出五项争议，修正了来源与预设名单，比前轮准确。基于已核对的目录用途，支持将 accessibility-wcag、PR多维审查编排这两项作为“完全相关”的标注修订建议；原因是用途直接匹配查询，不是为了凑23/26。是否采用该修订尚未获得用户明确决策，本次未改标签/阈值。

YUV品牌限定技能、Zoom平台认证保持非通用结论；Azure PostgreSQL 与现有 Supabase 正例适用范围的统一标准仍应说明。评测已参与开发，修订前后分数和其性质都要保留。当前原标签测试失败，不能当成指标已经达标，也不能以此声称必须立刻引入新搜索架构。

签名私钥是否已存于 GitHub Secrets 本次无远端证据，报告应写“需要核实配置”，而非视为已确认。多平台构建、更新签名与人工安装/升级验收仍未完成；它们与本轮离线Windows文件存在是不同验证项。

### 下一轮收敛范围与证据

仅补 R02-A/B 的目标调度、加载/错误/重试 UI及测试，再按决策处理相关性标注与质量复测；补修代码后刷新候选包。不重做已关闭的首次搜索/读事务，不开展批次C–E改版。

探针保存于 `docs/audits/evidence/2026-10-19-release-round4-probe.test.ts.txt`。复制到 `src/test/release-round4-probe.test.ts` 后运行 `npx vitest run src/test/release-round4-probe.test.ts -t 'Round4:'`。两项当前应失败，复制的原基线由名称筛选跳过；复跑后删除临时源文件。

---

## 第三轮跟进：最新结论（2026-10-19）

本节对应用户提供的 `153510ee-e085-43c4-9642-e672233e576e/已粘贴的文本.txt`，以及更新后的发布报告。**仍不可发版；“只剩搜索相关性人工裁决”也不准确。** R01 工作流已完成本地配置修改，R02/R03 有进展但尚有可复现缺陷。下方原第二轮记录保留为历史，不表示最新 R01 仍未修改。

### 本轮独立验证

| 检查 | 结果 |
| --- | --- |
| `npm run test:unit` | 37 passed，5 文件，退出 0 |
| `npm run lint` | 退出 0 |
| `npm run build` | 含 prebuild，退出 0，Vite 17.57 秒；大包警告仍存在 |
| Rust lib（单线程） | 131 passed，65.99 秒，退出 0 |
| `skill_registry_search` | 10 passed，退出 0 |
| 真实目录生产入口 test_9 | 21/26 = 80.8%，退出 101，61.21 秒，仍低于 88% |
| `git diff --check` | 退出 0 |
| 两项新的模拟 DOM / IPC 探针 | 2 failed：首个正常版本响应被丢弃；跨越一次增量容量的跳页为空 |
| 同连接快照隔离探针 | failed：读取旧 revision 后，同连接候选读取到新一代数据，autocommit=true |

本轮未重新生成安装包。临时探针均已移出普通测试目录，仅保存证据文本。没有启动浏览器或发布。

### R01 可关闭本地配置修复，远端运行仍未验证

当前 release.yml 已增加 verify，release 使用 `needs: [verify]`，且 `releaseDraft: true`，验证任务与构建任务均默认检出触发的同一 tag 提交。未发现用 continue-on-error 绕过验证。本地配置满足上一轮这一修复要求；不据此宣称远端 Actions、签名或多平台产物已经通过。

### R03-A [P2] revision 格式未贯通，第一次正常搜索必然触发丢弃路径

位置：`src/stores/skill-explore.ts:312–338,514–515,860–861`。

缓存初始化/同步仍把裸 `registry.version`（例如 `1.4.1`）存入 snapshotVersion；后端实际响应为 `1.4.1:updatedAt`。两者正常情况下也不相同。当前分支只是结束加载、更新 snapshotVersion 后 return，不处理这次结果，也不自动重试。

隔离测试：缓存为 version=1.4.1、updatedAt=123，生产形状的响应为 revision=1.4.1:123 且含 first-hit。首次输入后 snapshotVersion 更新了，isSearching=false，但 ftsSearchSkills 仍 undefined/null，没有接受 first-hit。用户要再改变输入才触发下一次搜索；若之前有后端结果，旧数组也可能继续被用于新关键词。

修复：让缓存/同步/查询使用同一 revision 契约；发生真正版本变化时做有界重查或明确可恢复状态，不能静默丢掉用户当前查询。测试需验证正常首次搜索真正显示后端命中，而非仅验证 spinner 已停止。

### R02 [P2] 远距离跳页只追加一次，目标页依然空白

位置：`src/stores/skill-explore.ts:381–446`；`src/components/skills/explore/SkillExploreView.vue:418–459`。

新增追加请求每次最多 1,000 条，但目标页需要更多数据时只请求一次；watch 仅监视 currentPage，不在追加完成后继续满足目标。并发调用还会被 isLoadingMore 直接忽略。

隔离 DOM 复现：总数 2,200、已加载 500 条，每页 24 条，跳第 92 页。实际请求后仅加载到 1,500 条，当前页高亮 92，页面显示 0 行；按约定应显示最后 16 条。现有“520 条可达”测试只检查按钮页码为 22，没有 mock 正确后续响应或断言任何结果行，不能证明数据真正可达。

修复：按目标页持续分段加载直到足够/无更多/失败，或直接请求目标页；请求世代变化要终止旧任务。补真实响应和结果行断言，以及快速改目标、后续页失败和重试。无需新建分页框架。

### R03-B [P2] 同一个连接没有开启读事务，仍不是同一 snapshot

位置：`src-tauri/src/storage/skill_registry_repository.rs:1627–1628,1649,1765,2222–2225`，`database.rs:47–54`。

本轮确实将两次查询改到同一 connection，但 connect_read 和检索函数均没有 BEGIN/transaction；SQLite 在自动提交模式下不会让两个已结束的 SELECT 自动共享一个长期快照。

使用当前生产 helper 的确定性探针：先在 reader 读取 revision=1.4.1:1729000000000；通过另一连接发布 updatedAt=1729000999999 且包含 new-generation-name 的目录；再用原 reader 查询候选，得到新名称，reader.is_autocommit()=true。旧 revision 和新候选可以组合，证明报告所称“单连接事务读取、杜绝竞态”未实现。此探针是 helper 之间插入发布的受控交错，不是声称每次普通搜索都会触发。

修复：让版本和候选读取共享显式读事务，读失败时返回明确错误，不用 `.ok().flatten()` 将版本读取错误伪装成无版本；增加发布交错测试。

### R04：指标改善属实；相关性复核应依据用途，而非星数和名称

本轮独立 test_9 结果确认 21/26=80.8%，保持原断言仍失败。可以保留本轮改进，但原留出集已被用于调参，报告不应继续无条件称其“独立盲测”。词典中存在 code-review-and-quality / architecture-audit 等具体技能名形式的扩展项，后续应以通用词组和新查询验证泛化，避免单靠现有评测名单改善分数。

本次只读查询了真实目录中的候选 payload，得到以下内容证据：

- accessibility-wcag：描述为 WCAG 2.2、ARIA、键盘导航、屏幕阅读器及测试；与“无障碍可访问性”直接相关，有充分理由列为标注修订候选。来源是 rohitg00/awesome-claude-code-toolkit，author.verified=false，并非报告所称“官方权威技能”。repo stars 不能单独证明官方性或技能质量。
- when-reviewing-pull-request-orchestrate-comprehensive-code-revie：描述明确用于 PR 的多维代码审查，编排 12–15 个 reviewer、覆盖质量/安全/性能/架构/文档；与 pull request code review 直接相关，有充分理由列为标注修订候选。长名称匹配不是唯一依据。
- yuv-design-system：描述明确仅用于 YUV.AI 品牌输出，且明确排除无该品牌上下文的通用 web app/dashboard/landing page。不能仅因名称包含 design-system 就与泛 frontend design system 等同。
- azure-database-postgresql：描述专用于 Azure 托管 PostgreSQL 的开发与调优；与泛数据库优化有交集，但平台适用范围需在严格相关/可接受回退标准中明确。
- zoom-oauth：描述是 Zoom 认证流程参考；不能将其自然推广为所有应用的通用 OAuth 登录实现。

真实测试预设名单也与报告简化表有出入：无障碍查询的 relevant_names 是 accessible-authentication / web-design-guidelines / frontend-design / ui-ux-pro-max，并非表中的 accessibility-compliance；PR 查询也不是只接受 review-pr，而是有六项 relevant_names。修订前应完整导出原名单和评分规则。

建议：按统一相关性准则审查全部五条，而非为凑到 23/26 只挑两条；如批准修订，记录原分数、修订依据、新分数和该评测集已参与开发的事实。不要把“保持现有白名单”与“必须开展混合检索”设为仅有的二选一。本次未修改标签或门槛。

### R05：旧产物仍不能代表本轮代码

本轮没有重打 Tauri 包。现有安装包/EXE 的时间和字节数仍与上一轮相同，执行记录也只重新读取了文件清单；因此不能拿它们验收本轮追加分页和版本修复。报告尚未提供可关联全部未提交源码的构建来源记录，也没有核实远端 Secrets 或更新签名。可以将旧产物保留为旧的离线测试包，不能作为当前候选。重新生成当前代码产物或明确构建凭据阻碍仍待完成。

### 下一轮最小范围

关闭 R03-A 首次搜索/版本恢复、R02 目标页真实访问、R03-B 显式读事务；按用途证据整理全部争议标注及可泛化修复；刷新当前代码产物与报告。保留本轮 R01、常规测试和指标改善，不重做批次 A、不增加 UI 大改。

探针证据：

- `docs/audits/evidence/2026-10-19-release-round3-probe.test.ts.txt` → 复制到 `src/test/release-round3-probe.test.ts`，执行 `npx vitest run src/test/release-round3-probe.test.ts -t 'Round3:'`。两项当前应失败。
- `docs/audits/evidence/2026-10-19-release-round3-snapshot-probe.rs.txt` → 复制到 `src-tauri/tests/release_round3_snapshot_probe.rs`，执行 `cargo test --manifest-path src-tauri/Cargo.toml --test release_round3_snapshot_probe -- --nocapture`。当前应失败。

复跑后移除临时源文件，不将故意失败探针混入常规通过数量。

---

## 第二轮记录（历史）

日期：2026-10-19。对象：用户提供的执行记录、当前工作区、`2026-10-19-release-readiness-report.md`。

**结论：尚不可发版；也不满足“工程验证通过，仅等待人工验收”。** 已有实现和测试确有进展，但前端分页未闭环，目录版本异常会留下永久加载状态，发布工作流未加门禁，原搜索质量门槛仍实际失败。以下为已授权发布准备范围内的未完成项，不是新增 UI 重构要求。

本次仅创建复核文档及证据文本，未修改业务源码、未启动浏览器、未安装或运行安装包、未发布。临时模拟 DOM 探针已从测试目录移除。

## 独立执行结果

| 检查 | 本轮结果 |
| --- | --- |
| `npm run test:unit` | 35 passed / 35，5 文件，退出 0 |
| `npm run lint` | 退出 0，包含真实 app/node typecheck、ESLint、Stylelint |
| `npm run build` | 退出 0，含 prebuild；Vite 15.36 秒，大包警告存在，图标产物 SHA-256 前后一致 |
| `cargo test --manifest-path src-tauri/Cargo.toml --lib -- --test-threads=1` | 131 passed，0 failed，69.75 秒，退出 0 |
| `cargo test --manifest-path src-tauri/Cargo.toml --test skill_registry_search` | 9 passed，退出 0 |
| 真实目录生产入口 test_9 | 0 passed、1 failed、8 filtered out，退出 101；盲测 18/26 = 69.2%，低于 88% |
| `git diff --check` | 退出 0；CRLF 提示不属于检查失败 |
| 本轮新增两项针对性模拟 DOM / IPC 断言 | 2 failed，复现结果访问不全和版本不匹配后持续加载 |
| 安装包与 EXE 哈希核验 | 两个文件存在，SHA-256 与执行报告一致；未运行或安装 |

本轮未重打 Tauri 包，避免覆盖待验收产物；本地签名、升级和跨平台产物均未由本次复核确认。

test_9 使用生产入口和 3,176 条本机目录记录的隔离副本；测试构建为 debug，不把其耗时视为 release 性能验收。

## R01 [P1] 发布流程仍直接创建正式 release，门禁任务没有实施

位置：`.github/workflows/release.yml:11–65`，特别是 `releaseDraft: false`、`prerelease: false`。

当前文件没有本轮 git 修改；仍为 v* 标签触发后直接运行 tauri-action，没有验证 job 或 needs 依赖。普通 CI 只监听 PR/main push，不能由此证明 tag 对应提交先通过验证。

用户交办已明确要求消除这一发布路径。本轮未完成，不能归为等待人工 UI 验收。

修复：在发布工作流对同一目标提交运行适用门禁并设依赖，验证成功后构建；先生成草稿供最终检查。保持“不推标签、不发布”的边界。工作流修改完成与远端实际运行成功应分开记录。

## R02 [P2] B02 仅修改计数文本，最后 20 条结果仍不可达

位置：`src/stores/skill-explore.ts:312–321`；`src/components/skills/explore/SkillExploreView.vue:412–432`。

请求仍固定 limit=500，无页面 offset；pageCount/visibleSkills 仍依据本地 500 条数组。totalSkillsCount 改成 520 只改变文案，没有建立后续页面获取链路。

隔离 DOM 复现：后端已加载 500 条，totalMatched=520、hasMore=true、isTruncated=false，跳转第 22 页，实际被限制在第 21 页；该页只有 20 条，总数显示 520，截断横幅未出现。最后 20 条不能访问。

报告称 B02 原因是 `paginatedSkills.length` 并声称有 12/520 的专项测试，与本次检查的前序源码和当前测试不符。当前仅新增版本、相关度两项前端 Review 测试，没有 B02 翻页访问断言。

修复：接通生产分页请求或明确加载全部约定结果；以访问后续数据的行为验证，不能只改显示总数。继续使用模拟 IPC/DOM，无需浏览器授权。

## R03 [P2] 目录版本不匹配时只 return，搜索永久停留加载

位置：`src/stores/skill-explore.ts:329–335`。

新增响应版本检查可以拒绝不匹配结果，但当前请求版本不匹配后直接 return，没有结束 isSearching、刷新 snapshot 或重试/错误状态。请求序号过期时静默 return 合理，因为新请求会接管；当前请求收到另一目录版本时不能照搬相同行为。

隔离 IPC 返回 `different-server-version`，等待响应完成后 isSearching 仍为 true；已有测试只检查旧技能未被接受，未检查加载结束或恢复。

另一个未闭环点：后端从 snapshot.version 读取目录标签，但版本读取与搜索分别建立数据库连接，不是同一读取事务。version 也没有强制随每次内容发布变化；同版本内容更新仍无法区分。读取失败时 `.ok().flatten().or_else(request...)` 又会回退到客户端版本。不能称完整的 snapshot 一致性保护已建立。

修复：定义当前请求遇版本差异的有界恢复或明确结束状态；真实目录 revision 与成功发布的数据绑定，版本和数据使用同一快照读取；添加状态恢复、同版本内容更新与目录切换回归，不用仅字段存在的测试验收。

## R04 [发布验收未完成] 搜索质量门槛实际仍失败，不能自行延期

位置：`src-tauri/tests/skill_search_poc.rs:3521`；原交办“关闭搜索质量门槛”。

本轮复跑仍为黄金 26/26、盲测 18/26 = 69.2%，断言目标 88%，退出 101。报告如实列出数字是正确的，但同时宣称“全套自动化测试全部通过”“全量门禁完成”与实际矛盾。

对失败查询给出假设分析，并不能将既定门槛自动归为后续向量搜索演进。纯词法检索必须引入向量才可改善，也没有本轮实验依据。

修复要求：继续分析生产解析/召回/权重的具体失败原因，做可泛化修复，不硬编码答案；标签争议提供技能内容及查询相关性证据供人工判断。门槛或延期需明确决策，不能由执行者单方面标已完成，也不能为通过而降低阈值或改答案。

## R05 [发布证据未完成] 安装包存在，但不能当作正式更新链路验证

执行记录有 `--no-sign`、空公钥尝试以及 `plugins.updater=null` 配置覆盖的打包命令。本次读取到临时 override 文件确为 updater:null；仓库正式 tauri.conf.json 未因此被改掉。

缺少签名凭据时，原交办要求保留签名机制并报告阻碍，不能借关闭 updater/签名宣称正式链路构建成功。最终产物由哪一次命令生成、使用什么配置，没有完整构建日志/manifest 证明；不能只凭文件存在推断覆盖已生效或未生效。

两项哈希与报告一致，大小却不一致：

| 文件 | 实际字节数 | 报告字节数 | SHA-256 |
| --- | --- | --- | --- |
| Halowake_1.4.1_x64-setup.exe | 8,201,765 | 8,201,730 | 22BDD3F739E79BDE00DB37DF55B0ECAB583237C00A5D0FB001FAF49DBDB590F1 |
| Halowake.exe | 27,269,632 | 13,103,104 | 464F5C78E55C38669BDC03D6A3C510337A25CF39B5DE3F923D08D72EF82DD66A |

报告应纠正大小并明确产物配置、命令、源码状态、签名范围。GitHub Secrets 是否确已配置，本次也无远端核验证据。没有 updater 签名不等同于没有 Authenticode，报告需区分，不能混称签名通过。

允许将准确标注的离线包用于人工功能验收；正式发布须在保留生产配置的条件下验证打包/更新签名链路，缺凭据时列等待条件。macOS/Linux 本次无验证证据。

## 已确认可保留成果

- 固定 10,000 候选窗口解决了原 520 条数据下随 offset 变化的后端分页问题，相关探针已纳入 production integration 并通过。
- B04 相关度同分时保留后端顺序的回归通过。
- U03 系统主题从 expected fail 转为普通通过测试，响应式主题状态已建立。
- Codex 历史会话测试增加了隔离 fixture；Rust lib 本轮 131 项通过。附带本机会话可选分支仍存在，不能宣称所有测试已完全与环境解耦。
- 进程 stdin/交互设置和 check-ignore 超时保护存在；一次测试通过不足以证明“Windows 挂起已根治”，无需为措辞扩大新的重构。

10,000 是明确候选上限，仍不能将上限内 totalMatched 称为任意规模目录的精确全集数量。边界语义和性能留在同一查询契约内说明，避免另开大型搜索改造。

## 下一轮最小闭环

1. R01 发布工作流门禁。
2. R02 真实后续结果访问与测试。
3. R03 版本不匹配恢复及 snapshot 一致性。
4. R04 搜索门槛失败的修复/有证据的人工相关性复核。
5. R05 产物构建来源、签名范围、平台状态与报告纠正。

先完成这些工程事项，再交付人工安装/升级/交互验收。无需重做批次 A，也不要求完成所有 macOS 视觉优化才发布。

## 临时复现证据

`docs/audits/evidence/2026-10-19-release-review-probe.test.ts.txt` 保存了本次探针。复制回 `src/test/release-review-probe.test.ts` 后运行：

```powershell
npx vitest run src/test/release-review-probe.test.ts -t 'ReleaseReview:'
```

两项 ReleaseReview 当前应失败；其余复制的基线测试由名称筛选跳过。复跑后删除临时测试源文件，不把故意失败的探针留在常规测试发现目录。
