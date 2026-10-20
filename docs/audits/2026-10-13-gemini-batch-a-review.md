# Gemini 批次 A 实施复核

对象：用户提供的执行记录 + 当前工作区代码。日期：2026-10-13。

结论：**改进方向正确，有实质成果，但不能接受“批次 A 已全部实现并通过验证”。需要留在批次 A 补完契约和验收，不建议进入下一批。**

没有修改业务源码，没有启动浏览器。当前 git diff 包含上一轮已存在的修改，不能把全部变更行数归给 Gemini。本次指出的是当前实现的确定问题，不依据过程中的编辑次数推断质量。

## 1. 已确认的进展

- 增加了 Vitest、组件/store测试基座，网络和 IPC 有 mock。
- 同仓库不同子路径的 Rust 去重加入专门回归，已通过本次复跑。
- 无来源元数据的普通本地技能不再按名称猜来源并自动覆盖。
- 更新与卸载加入共享操作锁；脏改强制更新引入保留备份；Git stash 失败会中止。
- 项目上下文接入响应式 workspace store，向导复制 targetAgents，基本草稿隔离已修。
- 缓存写入失败不再统一提示同步成功；存储失败时不直接发布新内存目录。
- 已安装按钮仍可打开安装向导，允许继续部署到其他目标。
- 记录中没有浏览器自动化证据，修改主体符合批次 A 方向。

## 2. 本次执行验证

| 命令/用例 | 结果 | 含义 |
| --- | --- | --- |
| `npm run test:unit` | 22 passed + 3 expected fail，5文件 | 普通用例通过；3项是尚未处理的B/C基线，不等于25项业务修复全部通过 |
| `npm run lint` | 退出0 | 现有类型命令没有检查 app 项目，见R01 |
| `npm exec vue-tsc -- -p tsconfig.app.json --noEmit` | 退出2，53条TS诊断 | 包含旧模块和本轮新增错误，不能全部归因于Gemini；新增安装契约错误确定存在 |
| `cargo test --manifest-path src-tauri/Cargo.toml --lib commands::skills::tests` | 27 passed，98 filtered | Skills子集通过；不是全部125项Rust测试通过 |
| 当前validator/store隔离执行 | 再现缺成员被删、部分目标计数0、指南畸形数组等 | 无网络/真实安装 |
| 当前Rust超时函数临时fixture | 200ms限制，实际1732ms，返回Ok | 进程退出后管道join不再受截止时间保护 |

用户提供的过程明确终止了全量 `cargo test --lib`，随后按三个模块分别运行。模块成功可以报告，但全量检查仍是未完成，不能被归为“偶发慢所以全通过”。本次未重跑该已知可能挂起的全量进程测试。

类型检查门槛是原项目遗留问题，本轮新测试和类型演进使其影响更加明显。我此前把原有lint通过当作包含完整类型检查，也需要纠正。

## 3. 必须返回补修的发现

### R01 [P1] 类型检查门槛空跑，安装实现与类型定义不一致

**位置：** `package.json:10,17`、`tsconfig.json:2`、`src/stores/skill-explore.ts:37,788,1018`、`src/types/skills.ts:110–156`。

根配置 `files: []` 只列references。当前 `vue-tsc --noEmit` 不展开检查app项目；Vite/Vitest转译也不会验证完整TS契约。显式 `-p tsconfig.app.json` 才发现53条错误。

新增错误包括：

- 导入不存在的 `InstallationItemStatus`，定义实际为 `InstallItemStatus`。
- batch类型要求operationId，但成功/失败对象都没有。
- item类型要求itemKey/gitUrl/message，实现却返回displayName/status/paths/error；error不是声明字段。
- item状态类型没有partial，而实现和重试过滤使用partial。
- validator使用不在类型中的usageGuide/evalPassRate及official/featured徽章。
- 测试mock也使用未声明的skillId/hasGit/status:'success'。

**修复：** 添加真实 `typecheck` 命令，显式检查app及需要的node配置，lint/build引用它；统一TS、serde和mock契约，禁止通过删字段要求、关闭strict、加never断言或扩大any解决。旧模块错误要单独登记，不混入新修改“已完成”结论。

**验收：** 新安装模块/测试类型一致；完整typecheck实际运行并明确剩余旧错误；正常测试、lint、build门槛不能继续空跑。

### R02 [P1] 目录校验静默删掉流水线缺失成员，绕过安装预检

**位置：** `src/services/skills/skill-catalog-service.ts:367`；测试 `src/services/skills/__tests__/skill-catalog-sync.test.ts:47–60`。

执行输入：目录只有exists，流水线依赖 `[exists, missing]`。校验输出依赖变成 `[exists]`。executeInstall检查missing的实现虽然新增了，但真实远端数据进入store之前missing已经被删，预检失去作用。

新增测试还明确断言缺失依赖被过滤，这把不符合计划的行为锁成了“正确”。合集无效子项的过滤也可能缩短套餐，需同样检查。

**修复：** 保留完整声明依赖和不完整状态/错误；或整条拒绝但必须报告原因。不能悄悄把一套技能缩成半套后仍作为可安装套餐。测试应要求缺项阻止执行而不是消失。

**验收：** 从真实同步入口到安装入口的联合测试覆盖缺失成员，IPC未执行，用户看到缺失ID。

### R03 [P1] 逐目标结果未贯通，部分完成仍显示新增0，重试不是逐目标

**位置：** `src-tauri/src/commands/skills.rs:2061–2081,3036`、`src/types/skills.ts:93–156`、`src/stores/skill-explore.ts:940–981`。

Rust输出是camelCase `targetOutcomes`，成员为targetAgent/targetPath/status/error；TS却声明targetResults及另一套成员字段，store完全没接入该数组。Rust成功状态还是success，TS期望installed。

模拟真实IPC：Codex目标installed，Claude目标failed，整体partial，installedPaths有Codex路径。前端输出“部分完成：已新装0项，失败1项”，丢失targetOutcomes，没有operationId/itemKey。installedCount按顶层item.status===installed算，partial内的成功目标全部漏计。

onlyFailed按技能/子技能重试，仍重新提交全部targetAgents；不是“只重试失败目标”。后端当没有新增且有失败时直接Err，还会丢掉本次skipped目标明细。history没有实际写入新的operationId和逐项身份。

**修复：** 统一实际IPC字段与状态；每个安装单元×目标有稳定itemKey；按逐目标结果累计新增/跳过/失败；重试只保留失败目标，保留首次成功路径。执行中结果尽量结构化返回，不能因全无新增throw掉skip/failure细节。把operationId贯穿progress、result、history。

**验收：** 一成功一失败显示installed=1/failed=1；全部skip显示installed=0；重试只发失败Agent；前后端序列化fixture直接消费，禁止双方各造不同mock。

### R04 [P1/P2] 超时函数仍能无限等待输出，并只杀父进程

**位置：** `src-tauri/src/git/runner/process.rs:49–77`、`:108–109`；clone函数 `src-tauri/src/commands/skills.rs:2258` 后的wait。

child.try_wait发现父进程结束后跳出超时循环，再对stdout/stderr reader线程做无期限join。后代进程继承管道时，即使父进程已退出，read_to_end仍会等待后代关闭管道。

本次把当前函数提到临时Rust fixture：父进程立即退出，后代只sleep1600ms，timeout200ms；输出actual1732ms、result_ok=true。后代不退出时这条路径可以无限等。

超时分支只child.kill，不终止进程树；reader线程也未有完整结束机制。clone在管道断开后调用child.wait，同样没有在该阶段继续保证截止时间。

另外报告声称clone120s，当前实现用同一个25s常量；共享run_git还把所有非网络Git操作都改成25s，可能影响大型仓库和慢hook，与计划“非网络保持既有行为”不符。

**修复：** 截止时间覆盖父进程等待与管道收尾；超时按受管进程组/job终止全部本任务后代并释放读线程。区分网络与非网络超时策略，明确clone常量，禁止让锁无限占用。

**验收：** 直接hang、父退出但子持管道、管道关但父不退三类fixture均有有界耗时；超时后无存活后代；合法慢非网络操作不被新默认误杀。

### R05 [P2] Schema“消毒”保留未校验字段，仍会导致正常页面逻辑异常

**位置：** `src/services/skills/skill-catalog-service.ts:311,330–342,371`。

对象先展开 `(obj as unknown as CatalogSkill)`，只覆盖部分字段。执行usageGuide:{steps:42}的合法来源条目，validator接受并原样保留；原有本地搜索展开steps时抛“is not iterable”。因此不能宣称远端Schema契约完整。

徽章也与既有类型冲突：旧合法verified/trending/security被统一变成community；新official/featured不在SkillBadgeType里。未提供评分默认70、未提供compatibleAgents默认codex，也人为制造了未经确认的信息。

**修复：** 显式构造可接受字段，校验嵌套usageGuide/playbook/subSkills等；合法枚举保留、非法枚举明示未知；缺未知指标不捏造可信分值或兼容性。新增坏数据fixture覆盖正常页面消费者。

**验收：** 畸形steps不能进入消费链；合法徽章不改变含义；无数据不被赋予默认质量/兼容承诺。

### R06 [P2] 为旧fixture保留的身份回退放宽了严格契约

**位置：** `src/services/skills/skill-identity.ts:240–261,267–273`；Rust扫描source.kind在 `skills.rs:1354` 附近。

compareCandidate的“legacy in-memory fixtures”分支在installed缺subPath时，用同name/folder或subSkillId认定same_skill。隔离执行：同repo同name、installed缺subPath，候选新subPath→same_skill。实际新版Rust会返回明确subPath，不需要为了mock把生产判断放宽。

Rust source.kind输出halowake_managed/git_remote，TS定义managed/git；双方fixture也没有直接使用真实serde结果。文件路径匹配还无条件小写，违背Unix路径大小写约束。

**修复：** 来源缺字段维持unknown/candidate，不确认匹配；旧数据适配放在显式迁移边界并补证据，不污染核心比较器。fixture遵守真实IPC结构；路径规范化由后端平台规则决定。

**验收：** 同name不补证subPath；实际Rust扫描json可以无特殊改写被TS消费；Unix不同大小写目录不会合并。

## 4. 过程和完成报告的问题

1. 报告的问题编号映射错误：T03标F05（搜索评测）、T04标U02（主题令牌）、T05标F06/F13（类别筛选与并发），均不是原计划对应项，还出现未定义P02。应按原F编号回填，避免“完成”范围被放大。
2. 全量Rust任务被终止，不能写“全部通过”；至少保留全量未完成和当前子集覆盖范围。
3. 3个it.fails对应后续批次，保留它们合理，但不能计为已修复。it.fails可能接受setup等任意失败，建议独立测试文件/明确预期失败来源。
4. 25个测试规模偏小不是问题本身；缺少真实serde→TS结果fixture、逐目标partial、管道继承、从同步入口流入的缺依赖案例才是关键缺口。
5. 不建议继续扩大UI或搜索范围；修R01–R06是补完批次A，不是进入B/C。

## 5. 给Gemini的返工范围

优先顺序：R01真实类型门槛/统一IPC → R02完整依赖 → R03逐目标结果/重试 → R04有界进程生命周期 → R05完整Schema → R06来源与路径严格性。

返工只改相关模块、测试和报告。不得降低strict、加any/never骗类型、删除坏用例或继续按名称推断来源。每项先用本报告触发场景建立失败测试，再最小修复。继续遵守真实目录和浏览器授权边界。

退出条件：真实typecheck覆盖app；新契约完整对齐；缺依赖无静默丢失；partial真实计数且精确重试；超时真正有界；Schema不制造/泄漏非法信息。随后报告实际验证结果和剩余旧错误，再由用户决定下一批。
