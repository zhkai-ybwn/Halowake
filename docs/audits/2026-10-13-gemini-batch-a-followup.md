# Gemini 批次 A 第二次返工复核

日期：2026-10-13。证据：用户提供的第二次执行记录、当前工作区源码、实际运行的检查及隔离复现。

结论：**比上一轮明显改善，真实类型门槛已修；批次 A 仍有4个具体缺口，先补齐再验收。** 不建议重复整个优化计划或扩展到下一批。

本次仅新增复核文档，未修改业务源码，未启动浏览器；文件/进程用例在临时环境执行，不操作真实技能目录。

## 1. 已核实的检查

| 检查 | 本次结果 |
| --- | --- |
| `npm run lint` | 通过，已实际包含app/node typecheck，不再是根配置空跑 |
| `npm run test:unit` | 26 passed + 3 expected fail（29项、5文件） |
| `cargo test --manifest-path src-tauri/Cargo.toml --lib -- --test-threads=1` | 128 passed，0 failed，68.06秒，退出0 |
| Vite完整生产build | Gemini记录称通过；本轮未再次执行 |
| 搜索PoC/integration | 本轮未重跑，不把128项库测试称为全部Rust集成通过 |
| 浏览器、macOS/Linux系统行为 | 未执行 |

这次全量库测试在单线程下完成；可以补上此前“全量未完成”的缺口。但并行测试是否稳定、生产搜索原有验收失败，仍不能由这一结果推导为解决。

## 2. 上轮R01–R06状态

| 项目 | 本次判断 |
| --- | --- |
| R01类型门槛/类型契约 | 真实typecheck已经接入且通过；operation/item历史贯通仍见S04 |
| R02完整流水线依赖 | 基本修复；validator保留缺失ID，安装预检可识别 |
| R03逐目标结果/重试 | 首次partial计数和只重试失败Agent基本修复；异常路径仍假成功，见S01 |
| R04完整超时 | 引入Job/进程组和更明确时限；管道超时仍返回Ok、错误处理不完整，见S02 |
| R05Schema | 畸形steps、默认评分、兼容性及合法badge已改善；丢弃合法来源/玩法元数据，见S03 |
| R06来源/路径 | 非空subPath无证据回退已收紧，Unix路径不再统一小写，实际source枚举已对齐 |

根技能缺subPath的兼容分支仍存在；后续应明确其证据规则，当前最优先是下列4项，避免无边界持续重构。

## 3. 剩余返工项

### S01 [P1] 重试IPC抛异常时，仍返回安装成功并关闭弹窗

位置：`src/stores/skill-explore.ts:1017–1075`。

触发：第一次Codex installed、Claude failed；点重试只提交Claude；这次调用抛出IPC断开/其他异常。

catch只保留此前的 `prevOutcomes`（其中失败Claude已过滤），给顶层item标partial并设置error。汇总看到targetOutcomes存在，就只按其中成功Codex计数，不再统计顶层partial错误。

本次执行实际store的隔离结果：

```text
第一次：status=partial，installed=1，failed=1
重试请求：targetAgents=[claude]
重试抛异常后：status=installed，success=true，installed=1，failed=0
item.status=partial，item.error=retry IPC disconnected
modalVisible=false
```

修复：对本次尝试的每个失败/未知目标补结构化failed/unknown结果；保留先前成功目标，计数基于完整计划。任何未解决error不能被纯成功outcomes掩盖；业务失败不能关结果页。再次重试应仍只提交Claude。

验收：补一个targetOutcomes真实结构的两次/三次调用测试：首次partial→第二次reject→第三次成功。第二次仍partial、installed1/failed1、弹窗打开；第三次保留Codex结果且最终installed2。

### S02 [P1/P2] 输出收尾超时被吞掉，仍返回成功

位置：`src-tauri/src/git/runner/process.rs:243–281`，Job登记 `:191–198`。

两处recv_timeout超时后调用kill、join，再用Vec::new替代丢失输出，最终Ok(Output)仍使用父进程之前的成功退出码。`.max(500ms)`还会把已耗尽deadline扩成额外等待。

本次把当前函数和Job实现编译到临时fixture，父进程立即退出，子进程继承输出管道sleep1600ms：

```text
configured_timeout_ms=200
actual_elapsed_ms=610
result_ok=true
```

这是已复现的超时错误语义；“无限等待”目前还取决于Job是否正确注册。创建/assign失败没有处理，assign_child的bool被忽略；失去树终止能力后仍执行无界reader join，属于静态风险。

修复：共用绝对deadline；管道收尾超时返回Err，不伪造空输出成功；明确处理Job创建/登记失败，执行前确保隔离或安全失败；所有清理有界，读线程不允许无限join。正常成功需完整收集输出或明确错误，不得静默截断。

测试缺口：当前只有直接hang和echo；名为allows_legitimate_slow_operations的测试实际仅echo，并未覆盖慢操作。本轮必须加入父退出/子持管道、管道断开/父存活、Job失败等路径，验证elapsed有界、结果Err以及后代退出。

### S03 [P2] 显式Schema构造丢失sourceId与pipeline.playbook

位置：`src/services/skills/skill-catalog-service.ts:367–417`。

不再展开未经校验原始对象是正确修复，但显式构造没有保留 `CatalogSkill.sourceId`，也没有构造流水线playbook。

隔离执行：输入sourceId=agent-skills-corpus和合法playbook，输出二者都为undefined；合法verified徽章、steps数组化、完整依赖保留已正常。

实际影响：`SkillCompactRow.vue`社区标识、`SkillInstallModal.vue:39`未审查来源提示、`SkillDetailDrawer.vue:347`社区来源/使用说明处理依赖sourceId。同步后这些提示消失；pipeline.playbook详情区域会空。

修复：显式校验并保留所有当前消费者使用的合法字段，尤其来源标识与playbook；未知来源不提升信任。补字段级fixture与UI/service联合测试。不要恢复...obj，也不要仅删除UI提示规避。

验收：社区条目同步后仍被标为社区索引、安装仍提示未审查来源；合法playbook保留全部规范化步骤、非法步骤有明确策略。

### S04 [P2] operationId/itemKey只加声明，进度和历史仍没有关联

位置：`src-tauri/src/commands/skills.rs:2059,2124,2176–2177`，前端payload `src/stores/skill-explore.ts:962`。

前端传operationId，但Rust执行链没有读取它；send_install_progress始终operation_id=None、item_key=None。历史INSERT/SELECT和数据库schema未增加关联字段，也未保存逐目标结果。前端itemKey仍是技能/子技能键，不是安装单元×目标的键。

因此“字段加入”不能报告为“进度、结果、历史透传”。一次重试还生成op-2，原op-1和重试没有持久关联；重启后只能按历史重装全部Agent，不能根据记录精确恢复失败目标。

修复：最小实现operation上下文传递，目标结果具有itemKey或targetKey；历史保存operationId、逐目标结果及重试关联，幂等迁移旧记录。明确历史未知细节不能臆造。测试直接验证发出的进度与读取历史，不能仅验证结构体serde含可选字段。

验收：单操作progress/result/history有同一关联ID；两子技能×两Agent结果可唯一识别；失败重试与原记录关联；旧历史仍可读且不被自动改写。

## 4. 报告准确性

- 新报告仍混淆原F编号（例如把超时写成F02、Schema写成F06），应引用返工R或本轮S编号，避免错误宣称原功能已修。
- 报告称operationId/itemKey透传，但源码字段始终None，历史没有保存；这是需要修正的实质完成声明。
- 不要称工作区“整洁稳定”：仍有大量未提交和未跟踪文件；可以准确称“既有工作已保留”。
- 128项库测试通过、26项普通前端测试通过值得记录；它们不覆盖S01/S02，也不覆盖生产搜索质量和实际UI行为。

## 5. 下一次返工范围

仅补S01–S04，优先S01/S02。先加失败回归，再做局部修复，保持已通过的typecheck与现有保护。不重做R01/R02/R06，不进入搜索或UI改版。

交付应提供每项实际测试、修复前后结果、完整检查状态，以及仍需人工验收的界面/平台项。完成后停在批次A供用户验收。
