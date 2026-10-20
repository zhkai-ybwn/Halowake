# Antigravity 批次 B 复核

日期：2026-10-15。对象：当前工作区、用户粘贴的执行记录、T06–T07 原验收要求。

结论：批次 B 部分完成，不能整批验收。原 F07 标签筛选与 F09 比较器问题有进展；生产搜索固定回归和 CI 已加入。但搜索分页/排序存在实际错误，目录版本仅传递未校验，真实目录质量门槛仍失败。本次未修改业务代码，未启动浏览器。

用户粘贴记录结束于启动 Rust lib 测试，并没有最终执行摘要。要求的 `docs/audits/2026-10-15-batch-b-completion-report.md` 当前不存在，因此不能将日志理解为已提交完整验收材料，更不能认定 Antigravity 已报告全部通过。

## 独立验证

| 检查 | 结果 |
| --- | --- |
| `npm run test:unit` | 5 文件通过，32 passed + 1 expected fail，共 33 项 |
| `npm run lint` | 退出 0，含 app/node 类型检查、ESLint、Stylelint |
| `npm run build` | 退出 0，执行了 prebuild；Vite 大包警告仍存在，属后续范围 |
| `cargo test --manifest-path src-tauri/Cargo.toml --test skill_registry_search` | 8 passed，退出 0 |
| `cargo test --manifest-path src-tauri/Cargo.toml --lib -- --test-threads=1` | 128 passed，1 failed，退出 1 |
| 真实目录生产入口评测 test_9 | 盲测 Strict P@1 = 18/26 = 69.2%，目标 88%，退出 101 |
| 隔离 Rust 候选窗口探针 | 复现总数变化、错误 hasMore、跨页重复、热门首位随请求大小变化 |
| 模拟 DOM / IPC 两项针对性断言 | 均失败：接受旧目录版本；同分相关度结果被前端改序 |
| `git diff --check` | 退出 1，4 处空白行问题；工作区混有前序修改，不能全部归因本轮 |

Rust lib 唯一失败项是 `commands::codex_report::tests::test_sep_11_sessions_loads_continued_codex_sessions`，依赖本机 2026-09-11 的真实会话，并断言必须出现 ecogrid-mdm / ami-insight。本次实际只加载到 Antigravity 的 Lumina 会话；对应源文件无当前 git diff。记录为环境耦合的既有测试问题，不据此认定搜索代码回归，也不扩大本批次去重构该模块。

test_9 使用 3,176 条本机目录数据，生产搜索入口在隔离数据库内评测；仅执行该测试，其他 8 项被过滤。黄金集 26/26、盲测 18/26，二者不能混算。测试为 debug 构建，耗时数据不能当作 release 性能验收。

## B01 [P1] 候选窗口随 offset 扩大，导致搜索排序与分页不一致

位置：`src-tauri/src/storage/skill_registry_repository.rs:1717–1725,2118–2135`。

非空查询先按 BM25 取 `(offset + limit).max(500)` 个候选，再对该窗口按 popular/name/relevance 排序与分页。不同页使用不同候选集合，排序并非针对同一完整匹配集合。`total_matched` 只是候选内通过打分的数量，`has_more` 也只检查此窗口。

使用提交的 fixture（520 条 dummy skills）经生产入口查询 `utility`，排序 popular：

| offset | limit | totalMatched | hasMore | isTruncated | 第一条 |
| --- | --- | --- | --- | --- | --- |
| 0 | 20 | 500 | true | true | dummy-skill-500 |
| 480 | 20 | 500 | false | true | dummy-skill-020 |
| 500 | 20 | 520 | false | true | dummy-skill-020 |
| 0 | 1000 | 520 | false | false | dummy-skill-520 |

第 480 条位置已声称无更多，但第 500 条位置仍有结果，且第一条重复；最高热度技能只在扩大请求后出现。原新增分页测试使用空查询浏览路径，未覆盖此非空搜索路径。

修复要求：对同一匹配集合确定排序和分页，不能以不断扩大的 BM25 子集作为热门/名称排序全集。精确总数、候选上限、是否还有结果需要真实一致；若确需搜索上限，须明确有限集合的边界与稳定性，不能把已截断数量当真实总数。浏览/回退路径另有 10,000 条硬上限且返回未截断标志，也应在同一契约修正中处理。

回归要求：使用非空查询覆盖超过 500 个匹配，验证总数不随 offset/limit 改变、跨页无重复遗漏、popular/name 首位不随请求大小变化。

## B02 [P2] 前端仍只分页前 500 条，新增响应字段没有形成用户可用行为

位置：`src/stores/skill-explore.ts:307–318,343–345`；`src/components/skills/explore/SkillExploreView.vue:170,396–397`。

搜索请求固定 `limit:500`，未随页面传 offset。视图仍通过 `sortedSkills.length` 计算数量、页数并做数组 slice。虽然 store 保存 totalMatched/hasMore/isTruncated，视图没有使用它们展示总数、截断或加载后续结果。

影响：即使后端修好契约，用户仍只能访问前 500 条；界面也未说明结果被截断。本条与 B01 分属后端稳定集合和前端访问链路两个修复点。

修复要求：将分页接入生产查询链路；如明确采用有界搜索结果展示，则显示已加载/匹配总量及上限，并在验收报告说明范围。不能继续无提示展示部分集合。通过模拟 IPC/DOM 验证下一页请求、数量和末页状态。

## B03 [P2] catalogRevision 是请求回显，不是已发布目录版本

位置：`src-tauri/src/storage/skill_registry_repository.rs:1677,2148,2285`；`src/stores/skill-explore.ts:317–345,764`。

后端返回 `request.catalog_revision.clone()`，没有从数据库已发布 snapshot 得到实际版本。前端只检查请求序号，不检查响应目录版本；目录版本是从 0 开始的 store 内计数，未与后端持久化目录绑定。

隔离 Rust 调用传 `invented-version`，原样返回该版本；模拟 IPC 返回 `stale-version` 和 old 技能，当前 store 仍接受 old。字段存在不等于具备版本一致性保护；多窗口更新或持久化目录变化无法靠请求字段回显识别。

修复要求：从真实发布的目录 snapshot 产生并读取版本；响应注明实际查询版本；前端在接受结果时处理版本不符并失效旧请求。对目录变化、旧响应以及重新初始化的情况添加行为测试。

## B04 [P2] 相关度同分时前端仍混入热度重排

位置：`src/components/skills/explore/SkillExploreView.vue:370–393`。

后端相关度分数相同后，前端继续落入热度、收藏、名称排序。后端相关度同分规则是 name/id，二者并不一致。

模拟后端返回同分顺序 `[test-a, test-b]`，星数分别 1/100，实际 DOM 顺序变为 `[test-b, test-a]`。这违反本轮“relevance 保留生产搜索顺序”的契约；也会使未来服务端分页和页面本地排序产生不一致。

修复要求：后端成功返回时直接保留其顺序；只有明确进入本地 fallback/本地浏览时才使用对应本地规则。新增相同相关度但热度不同的视图测试。

## T07 尚未闭环的交付项

- 真实盲测质量门槛本轮实际失败：69.2% < 88%。保持原门槛、记录失败查询与评分依据，继续针对生产逻辑修复；存在标签争议时给出证据请求人工判断，不能降低门槛或硬编码答案。
- 现有 8 项固定回归调用真实生产入口，并使用隔离数据库，方向正确。但分页只覆盖空查询；本轮无新增前端搜索竞态/版本/分页测试，F07/F09 主要是将既有预期失败转为普通测试。
- CI 确实增加 unit、完整 build 和确定性 integration，没有发现通过 continue-on-error 掩盖失败的写法。CI 通过这 8 项不等于真实目录盲测达到门槛。
- 缺少要求的完成报告。先补修 B01–B04，再生成真实结果与剩余项报告；完成后停在批次 B，勿进入 C–E。

## 可保留的成果

1. Installed 过滤按钮显式设置 kind/value，tag 与 stage 的实际点击分支已经分离；标签读取传入完整技能对象，F07 原路径通过。
2. 去除了阈值切换造成的非传递比较，F09 原排列测试通过。
3. 类别进入后端请求，SQL 在候选截取前执行 category/stage/allowedIds 过滤；提交的第 501 条类别召回测试通过。
4. 生产入口固定 fixture 和 CI 检查已经建立，应继续扩展，不推倒重来。
5. U03 保留为后续主题批次的唯一 expected fail，未越界处理。

## 证据与复跑

本次临时探针已从测试发现目录移除，保存为文本：

- `docs/audits/evidence/2026-10-15-batch-b-review-probe.rs.txt`：复制回 `src-tauri/tests/batch_b_review_probe.rs` 后执行 `cargo test --manifest-path src-tauri/Cargo.toml --test batch_b_review_probe -- --nocapture`。使用现有 fixture；执行后删除临时测试源文件。
- `docs/audits/evidence/2026-10-15-batch-b-review-probe.test.ts.txt`：复制回 `src/test/batch-b-review-probe.test.ts` 后执行 `npx vitest run src/test/batch-b-review-probe.test.ts -t 'Review:'`。包含原基线辅助代码，两项 Review 断言当前应失败；执行后删除临时测试源文件。

真实目录质量检查命令：

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --test skill_search_poc test_9_production_v32_30_gold_plus_30_blind_set_and_release_benchmark -- --nocapture
```

修复优先级：B01 → B02 → B03/B04 → 扩展回归、复测盲测、补报告。不扩大到安装/进程/UI 重构。
