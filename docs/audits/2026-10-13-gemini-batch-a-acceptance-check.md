# Gemini 批次 A 收尾验收复核

## 2026-10-15 最终补修复核：A-ID 关闭

当前结论：A-ID 补修通过，批次 A 可标记为“本轮工程检查通过”。下方 2026-10-13 的碰撞复现与剩余项描述保留为历史依据，不再表示当前源码仍有该问题。本轮只更新验收文档，未修改业务源码，未启动浏览器。

复核当前源码确认：

- `src/stores/skill-explore.ts:56` 使用 `crypto.randomUUID()` 生成 `op-<uuid>`；环境不支持时使用时间与随机值组合降级。主路径已消除本地计数器重置造成的确定性碰撞；降级路径属于概率唯一性，不能称为数学上的绝对唯一保证。
- `src/stores/skill-explore.ts:778` 保留整数 `requestSeq`，继续用于丢弃过期异步回调；持久化操作 ID 与请求世代号已分离。
- 单次安装生成一次 ID，沿用至 IPC 参数、进度和最终批次结果；已有后端历史写入链路继续接收该参数。本轮未进行真实桌面安装与数据库端到端重放。
- 每次重试生成新 ID，`retryOfOperationId` 取前一次实际批次结果的 ID。三次尝试中第二次 IPC reject 仍可正确关联前次，并保留已成功目标供第三次合并。
- `InstallationBatchResult` 已声明可选 `retryOfOperationId`。本轮没有新增迁移或批量重写旧历史。

本轮独立执行结果：

| 检查 | 实际结果 |
| --- | --- |
| `npm run lint` | 退出 0；包含 app/node 两份真实 typecheck、ESLint、Stylelint |
| `npm run test:unit` | 5 个文件通过；30 passed + 3 expected fail，共 33 项，退出 0 |
| 新增安装回归覆盖 | 独立 Pinia 会话 ID 不同；合集单批次 IPC/进度/结果 ID 一致；partial → reject → success 的三个 ID 与连续 retry 关联正确 |

Rust 129 项库测试通过是上一轮独立执行的结果，本轮未重复执行。Gemini 本轮报告的 skills 28 项与 migrations 3 项通过不计作本轮独立运行结果。3 项 expected fail 仍是后续批次待处理基线，不能列入已修复项。

本轮范围内未发现需要继续阻断批次 A 的问题。Gemini 摘要中的“并发安装”不够准确：当前合集测试覆盖多个子技能的同批次 ID 一致性，并不证明并发调度正确性。该表述无需引出新的实现任务。

工程检查通过不等于全项目或全部 UI 已验收。用户手动安装/失败重试/历史查看验收，以及批次 B–E 的搜索、过滤、排序、主题与 macOS UI 仍按原计划跟进。

---

日期：2026-10-13。对象：当前源码和用户提供的收尾摘要。

结论：前述S01–S03已得到修复；S04的进度、目标结果和历史字段已经贯通，但操作ID仍只有会话内唯一性，持久化关联还需一次局部补修。其余已确认改进保留，不重复整轮重构。

本次未修改业务源码，未启动浏览器。

## 本次实际验证

| 检查 | 结果 |
| --- | --- |
| `npm run lint`（含app/node真实typecheck） | 通过，退出0 |
| `npm run test:unit` | 28 passed + 3 expected fail，31项、5文件 |
| `cargo test --manifest-path src-tauri/Cargo.toml --lib -- --test-threads=1` | 129 passed，0 failed，73.72秒，退出0 |
| 三次安装隔离复现 | 首次partial→重试reject仍partial/弹窗保留→再重试成功，成功目标结果保留 |
| 当前进程函数临时fixture | 200ms配置，父退出/子持管道返回Err，不再返回假成功；含清理实际约899ms |
| 两个独立store/session隔离复现 | 两次首次安装均生成 `op-1`，确认ID碰撞 |

前端3项expected fail仍是后续批次基线，不是修复通过。本次只验证Rust库测试，不包含原搜索PoC集成质量门槛或跨平台运行。完整生产build本次未重复执行。

## S01–S04状态

| 项目 | 判断 | 依据 |
| --- | --- | --- |
| S01重试异常假成功 | 已修复本次触发路径 | catch补failed outcomes，汇总检查未解决error；实际复现第二次仍installed1/failed1、success=false、modal开启，第三次只发Claude并最终installed2 |
| S02管道超时返回成功 | 已修复本次触发路径 | 使用绝对deadline，输出超时返回Err；Job创建/登记失败返回Err；新增父退出/子持管道测试；未据此声称全部平台极端进程树情况已验收 |
| S03合法元数据丢失 | 已修复 | 显式保留sourceId，sanitizePlaybook，新增对应回归通过 |
| S04进度/历史关联 | 基本贯通，唯一性未闭环 | 参数进入执行/进度；Migration11新增字段，历史读写包含目标结果与retry关联；但operationId仍是本地计数器字符串 |

## 剩余项 A-ID [P2] 持久化operationId跨会话碰撞

位置：`src/stores/skill-explore.ts:160,771,784,962,1128`。

`installOperationSeq`在store创建时为0，操作ID生成 `op-${opId}`。该计数器适合丢弃旧回调，不能作为长期历史关联ID。新会话/应用重启会再次产生op-1、op-2；多个窗口也可能如此。

隔离执行两个独立Pinia实例，各首次安装的结果：

```text
session A operationId = op-1
session B operationId = op-1
```

现有历史是按行ID写入，碰撞不会直接覆盖两条记录。但同operationId聚合与retryOfOperationId追溯会发生歧义，无法唯一确认原操作，违背S04持久化关联目的。

### 最小修复

1. 保留installOperationSeq作为内部请求世代号，不改变它的竞态控制用途。
2. 每次逻辑操作生成独立、跨会话唯一的operationId，例如运行环境支持的 `crypto.randomUUID()`；无需新增复杂依赖或工作流框架。
3. 同批次的各单元、progress、result和history沿用同一个ID；不能在每个channel消息或每个Agent内重新生成。
4. 重试生成新唯一ID，并把retryOfOperationId指向前一次实际ID。
5. 旧记录可读且不自动批量重写。对已有非唯一legacy ID不要猜测关联；有必要时明确标为旧记录/关联不确定。

### 最小测试

- 两个独立store会话首次安装operationId不同。
- 同批次两个技能目标的调用ID相同。
- partial→reject→成功各尝试ID不同，retryOfOperationId正确连续关联。
- 正常progress/result/history的关联一致；旧记录的null/旧格式兼容不退化。

### 补修后验收

运行相关安装测试、typecheck/lint、必要的历史测试；没有Rust逻辑变化时无需为前端ID生成重复跑全部Rust测试。报告ID规则及测试，保持其他已修项和用户未提交工作。

## 后续范围

A-ID完成后可将批次A标为本轮工程检查通过，并安排用户手动验收。搜索质量、标签过滤、排序、系统theme和macOS UI仍属后续批次，不随批次A一并宣布完成。
