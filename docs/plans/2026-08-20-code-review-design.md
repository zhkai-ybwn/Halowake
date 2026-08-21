# Lumina 本地 Code Review 设计方案

## 1. 产品目标

Lumina Code Review 的完整流程是：

```text
本地零 AI 关注度评分
        ↓
用户查看高分文件并手动勾选
        ↓
仅对勾选文件执行低 Token AI Review
        ↓
校验、去重并生成结构化 Review Report
        ↓
SQLite 保存任务、规则快照、问题和用户处理状态
```

产品优先级按以下顺序排列：

1. 审查结果有具体证据，可定位到文件和行。
2. 仅消耗必要 Token，不发送未勾选文件和大量无关上下文。
3. 用户可以添加项目或全局自定义 Review 规则。
4. 结果必须有稳定 schema 和固定界面结构，不展示一段无结构的 AI 文本。

本方案不设计 CI、合并门禁、PR 评论或远程发布，也不生成整体代码质量分。

## 2. 最重要的概念边界

### Attention Score 不是质量分

现有 0–100 分的唯一含义是：**这个文件在本次变更中是否值得用户优先关注。**

- 高分：建议优先阅读和勾选 Review。
- 低分：建议稍后阅读，或默认不消耗 AI Token。
- 它不表示好代码、坏代码、缺陷数或 Review 结果。
- 它不参与 finding 严重度或 Review Report 结论计算。

### Review 不生成总分

AI Review 输出的是结构化 findings、已审文件、规则覆盖和局限性。结果按严重度和置信度排序，但不把它们压缩成一个“整体质量 82 分”。

## 3. 当前能力与可复用资产

当前代码已有：

- `score_git_review_files` 和 `build_review_attention_with_progress`；
- 文件 role/kind/action/risk category 识别；
- diff 清理、候选证据行打分、单文件候选行上限；
- `MAX_TOTAL_EVIDENCE_CHARS = 12000` 的全局内容预算思路；
- 用户手动勾选 `reviewSelectedRaws`的界面状态；
- `light-review` AI 模型路由；
- 本地存储设置、保留期、占用统计和 `lumina.db` 路径预留。

当前仍需补齐：

- SQLite 驱动、migration 和 repository；
- 专用 Review Prompt Builder 和结构化响应解析；
- Token 预算计划、分批策略和使用量记录；
- 自定义规则模型；
- Review Report 的固定数据结构和 UI。

## 4. 整体架构

```text
Git Change Explorer
  ├─ 本地 Attention Scorer（0 AI Token）
  └─ 用户勾选文件
                 │
                 ▼
Review Planner
  ├─ 合并内置 / 全局 / 项目规则
  ├─ 只保留匹配选中文件的规则
  ├─ 提取 diff hunk / 所属符号 / 必要依赖签名
  ├─ 按模块和依赖关系分批
  └─ 分配 Token Budget
                 │
                 ▼
AI Review Executor
  ├─ 主 Review 调用（结构化 JSON）
  └─ 仅对重要但证据不足的 finding 小范围复核
                 │
                 ▼
Finding Validator
  ├─ schema 校验
  ├─ 文件 / 行号 / diff 证据校验
  ├─ 去重与同根因合并
  └─ 严重度与置信度校准
                 │
                 ▼
Structured Review Report + SQLite
```

## 5. Phase A：零 AI 关注度评分

现有 Attention Score 公式保留：

- 基础分 `8`。
- 角色：primary `+18`，tooling `+12`，secondary `+4`，generated/internal `-18`。
- 默认类型：source `+10`，config `+8`，style `+3`，lockfile `-5`。
- 风险类别：security `+18`，data `+14`，api/logic `+10`，config `+8`，types `+7`，markup `+5`，style `+3`，test `+2`。
- 变更类型：delete/rename `+10`，add/untracked `+6`。
- 变更行数达 20/60/180 时分别 `+4/+9/+16`。
- 有效证据达 8 条或 600 字符 `+10`，达 24 条或 1800 字符 `+18`，无证据 `-10`。
- generated/assets/i18n/lockfile/docs 默认跳过，`score >= 50` 标记为“建议优先 Review”。

需要新增：

- `scoreBreakdown: [{ factor, delta, evidence }]`，让用户知道为什么高分。
- “勾选所有推荐文件”和“清空勾选”。
- 高分只影响默认排序和勾选建议，不自动启动 AI。
- 用户可以勾选任意低分文件，Review Planner 不再二次过滤用户选择。

## 6. Phase B：低 Token、高质量 Review Planner

### 6.1 不发送什么

- 未勾选文件的 diff 和源码。
- 完整仓库文件树、完整 Git status 和无关历史。
- generated、binary、lockfile 的大段内容。
- 未匹配当前文件的自定义规则。
- 重复的系统指令、长文案和大量 few-shot 示例。

### 6.2 每个选中文件的最小上下文

```text
FileReviewContext
  path
  language / kind / action
  changed hunks（必选）
  enclosing symbols（所属函数、class、impl）
  referenced signatures（仅当 hunk 引用且能本地定位）
  matched custom rules
  local deterministic findings
```

不默认发送完整文件。小文件若完整内容低于当前文件预算，可发送全文，避免过度裁剪破坏语义。

### 6.3 分批不按“一文件一请求”

先按目录 scope、import/use 关系和变更 hunk 之间的标识符关联建立轻量分组：

- 高关联文件放在同一 batch，保留跨文件 Review 能力。
- 无关文件分批，避免一个超大 Prompt 稀释注意力。
- 一个文件只属于一个主 batch，共享签名可作为小段辅助上下文出现在多个 batch。

### 6.4 Token Budget

用户可选三档，默认为“精简”：

| 模式 |    总输入预算 |     总输出预算 | 用途                     |
| ---- | ------------: | -------------: | ------------------------ |
| 精简 |  约 8k tokens | 约 1.5k tokens | 日常小变更，优先核心问题 |
| 标准 | 约 16k tokens |   约 3k tokens | 多文件功能变更           |
| 深度 | 约 32k tokens |   约 5k tokens | 跨模块、安全或数据逻辑   |

预算是单次 Review session 的总上限，不是每个 batch 的上限。实际使用量优先读 provider 返回的 usage；没有 usage 时保存本地估算值并明确标记 `estimated`。

预算分配依据是文件大小、hunk 数和依赖关联度，不用 Attention Score 推断代码质量。Attention Score 只可在预算必须裁剪时作为“先保留哪些已选文件上下文”的次要排序信号。

### 6.5 自适应复核

主 Review 调用后，本地 validator 检查每条 finding。只有同时满足以下条件才再调用 AI：

- finding 声称是 critical/major；
- 行号可定位，但证据或触发场景不足；
- 本地上下文中确实存在可用于复核的小范围代码。

复核 Prompt 只包含该 finding、对应 hunk 和必要签名，不重发整个 batch。复核可以确认、降级或丢弃 finding。

## 7. 自定义规则

### 7.1 规则分两类

#### Deterministic Rule（0 Token）

使用路径、扩展名、新增/删除行、字符串或安全正则匹配：

- 禁止新增 `console.log`；
- 敏感目录不得出现硬编码 token；
- 特定 API 已废弃；
- 特定文件变更时需要同时选中对应测试文件。

MVP 不执行用户自定义 shell 命令，避免任意代码执行和跨平台问题。

#### Semantic Rule（AI）

使用简短的自然语言表达项目级语义约束：

- 所有 Tauri command 必须把内部错误转换为可理解的用户错误；
- 权限更改必须同时检查默认权限和升级路径；
- 更改数据库 schema 时必须提供 migration 与回滚/失败策略。

语义规则只在匹配到当前选中文件时注入 Prompt。

### 7.2 规则模型

```ts
interface ReviewRule {
  id: string
  name: string
  description?: string
  kind: 'deterministic' | 'semantic'
  enabled: boolean
  severity: 'critical' | 'major' | 'minor' | 'suggestion'
  category: ReviewCategory
  includeGlobs: string[]
  excludeGlobs: string[]
  languages: string[]
  deterministic?: {
    target: 'added-lines' | 'removed-lines' | 'changed-file-list'
    operator: 'contains' | 'regex' | 'missing-related-file'
    pattern: string
  }
  semantic?: {
    instruction: string
    evidenceRequirement: string
  }
  source: 'builtin' | 'global' | 'project'
  version: number
}
```

### 7.3 规则存储与优先级

- 内置规则在代码中版本化。
- 全局用户规则保存在 SQLite `review_rules`。
- 项目规则保存在 `.lumina/project-profile.json` 的 `review.rules`，方便项目内可见和同步。
- 同 id 时项目规则覆盖全局规则，全局规则覆盖内置默认值。
- 每次 Review 把最终生效规则写入 `rule_snapshot_json`，保证历史结果可解释。
- 单条语义规则指令建议限制在 300 字符内；默认单 batch 最多注入 12 条匹配规则，超出时要求用户收窄匹配范围。

## 8. AI Review 输出 Schema

AI 必须只返回与以下类型等价的 JSON：

```ts
interface AiReviewBatchResult {
  schemaVersion: 1
  batchId: string
  reviewedFiles: Array<{
    path: string
    status: 'reviewed' | 'partial' | 'skipped'
    limitation?: string
  }>
  findings: Array<{
    clientId: string
    ruleId?: string
    category:
      | 'correctness'
      | 'security'
      | 'data'
      | 'api'
      | 'performance'
      | 'concurrency'
      | 'reliability'
      | 'maintainability'
      | 'test'
      | 'project-rule'
    severity: 'critical' | 'major' | 'minor' | 'suggestion'
    confidence: number
    filePath: string
    startLine: number
    endLine: number
    title: string
    problem: string
    impact: string
    triggerScenario: string
    evidence: string
    suggestion?: string
  }>
  limitations: string[]
}
```

约束：

- finding 必须描述“什么输入/场景会触发”，不接受只说“建议优化”。
- critical/major 必须有可验证证据和影响，否则 validator 降级或丢弃。
- `startLine/endLine` 必须能映射到选中文件的当前 diff。
- AI 不输出 Attention Score、总质量分、pass/fail 或是否可提交。
- 没发现问题时 findings 返回空数组，不编造填充性建议。

## 9. 最终 Review Report 设计

Review Report 由本地程序组装，不直接展示 AI 原始响应。

```ts
interface ReviewReport {
  session: {
    id: string
    repoRoot: string
    diffFingerprint: string
    startedAt: number
    completedAt?: number
    status: 'running' | 'completed' | 'partial' | 'failed' | 'cancelled' | 'interrupted'
  }
  scope: {
    selectedFiles: string[]
    reviewedFiles: string[]
    partialFiles: string[]
    skippedFiles: Array<{ path: string; reason: string }>
  }
  overview: {
    critical: number
    major: number
    minor: number
    suggestion: number
    appliedRules: number
    triggeredRules: number
  }
  findings: ReviewFinding[]
  cleanFiles: string[]
  limitations: string[]
  usage: {
    mode: 'compact' | 'standard' | 'deep'
    aiCalls: number
    inputTokens: number
    outputTokens: number
    estimated: boolean
    batches: Array<{
      batchId: string
      files: string[]
      inputTokens: number
      outputTokens: number
      durationMs: number
    }>
  }
}
```

### 界面结构

1. **摘要栏**：审查状态、选中/已审文件数、各严重度数量、Token 和耗时。不显示整体质量分。
2. **重点问题**：先 critical，再 major/minor/suggestion；同级按 confidence 和是否复核确认排序。
3. **文件视图**：左侧选中文件及问题数，中间 diff 和行内标记，右侧 finding 详情。
4. **规则结果**：显示哪些内置/全局/项目规则被应用，哪些触发，哪些因 scope 不匹配未应用。
5. **审查局限**：单独展示被截断的文件、上下文不足、batch 失败和未复核的低置信结果。
6. **Token 明细**：默认折叠，可查看每个 batch 包含哪些文件、输入/输出 Token 和耗时。

Finding 本地状态为 `open / confirmed / ignored / fixed`，状态和用户备注立即写 SQLite。

## 10. SQLite 数据模型

### `review_sessions`

保存 repo、diff fingerprint、status、phase、选择范围、budget mode、rule snapshot、Token usage、时间、`expires_at` 和 `is_pinned`。

### `review_files`

保存 session id、path、change kind、Attention Score、score breakdown、是否被用户选中、review status、batch id 和 limitation。

### `review_findings`

保存结构化 finding、fingerprint、来源、匹配规则、严重度、置信度、位置、证据、建议、是否经过复核、用户状态和备注。

### `review_rules`

只保存全局用户规则。项目规则仍在 project profile，每次 session 在 `rule_snapshot_json` 中保存合并后快照。

### `review_ai_calls`

保存 session id、batch id、model id、selected file list、input/output Token、usage 是真实还是估算、耗时、status 和 error。不默认保存完整 Prompt 或 AI 原始响应。

## 11. 任务与页面生命周期

- Tauri 内存 Task Registry 持有正在运行的 Review 任务和取消句柄。
- SQLite 是 session/files/findings/usage 的事实来源。
- 切换 Git/DevDock/Settings 不取消任务，返回时先查 SQLite 快照，再订阅进度。
- 应用重启时把遗留的 running 任务标记为 interrupted，保留已完成 batch 和 findings，允许用户重新 Review。
- diff fingerprint 变化后历史结果标记 stale，仍可查看，但 UI 不把其混入当前 diff。

## 12. 参考的开源设计

- [reviewdog](https://github.com/reviewdog/reviewdog) 提供通用诊断格式，并支持 `added`/`diff_context`/`file` 等 diff 过滤模式。Lumina 借鉴“统一 finding schema + 只保留与 diff 相关的问题”，但不集成其 CI reporter。
- [PR-Agent 的 Review Prompt](https://github.com/the-pr-agent/pr-agent/blob/main/pr_agent/settings/pr_reviewer_prompts.toml) 要求只关注新引入代码，使用明确的类型化输出，并要求问题关联文件和行号。Lumina 采用同类型的结构化约束。
- [PR-Agent 配置](https://github.com/The-PR-Agent/pr-agent/blob/main/pr_agent/settings/configuration.toml) 提供 token-aware patch fitting、large patch clip/skip、finding 数量限制和可配置额外指令。Lumina 将这些能力收敛为会话级总 Token Budget、自适应分批和 scope-aware 自定义规则。
- [Semgrep 自定义规则](https://semgrep.dev/docs/writing-rules/rule-ideas) 使用项目规则编码危险 API、废弃 API 和团队不变量。Lumina MVP 借鉴“规则有 id、scope、severity、message”的思路，但只实现安全的 diff pattern 和 AI semantic rule，不复制完整语法引擎。

## 13. 安全与质量控制

- diff 和自定义规则都是不可信输入，不能触发命令执行、读取额外文件或改变系统指令。
- 云 AI 调用前显示 provider、选中文件和预估 Token；Ollama 标记为本地模型。
- secret 检测在发送 AI 之前本地执行，对高可能的凭据进行占位脱敏，同时保留一条本地 deterministic finding。
- 用户项目内的代码和完整 Prompt 不默认写入 SQLite。
- schema 失败可做一次仅包含错误信息的修复请求；仍失败则保存该 batch 错误，不把原始文本当作正常 Review 展示。
- 多 batch 中某一个失败时，其他结果保留，Report 状态为 partial 并在 limitations 里列明。

## 14. 分期实施

### Phase 1：Review 基础层

- 补齐 SQLite driver、migration、Review Repository 和 Task Registry。
- 持久化 Attention Score、breakdown 和用户勾选结果。
- 实现切页不丢任务、取消、interrupted 和查询历史。

### Phase 2：Review Planner 与结构化结果

- 实现局部符号/签名提取、文件关联分批和 Token Budget。
- 实现 AI schema、validator、去重和自适应复核。
- 实现固定结构 Review Report 和三栏界面。

### Phase 3：自定义规则与本地闭环

- 实现 deterministic/semantic 规则编辑、验证、scope 预览和规则快照。
- 实现 finding 的 confirmed/ignored/fixed 和备注。
- 实现重新 Review 时的 finding fingerprint 对比。
- 把 Review 历史和 AI call usage 纳入本地存储保留与清理。

## 15. 验收标准

- 未勾选文件的源码和 diff 不出现在 Review Prompt。
- Attention Score 只影响文件排序和勾选建议，不出现在 finding 严重度或整体质量计算中。
- 每条 critical/major finding 都有选中文件、有效行号、触发场景、证据和影响。
- Report 始终符合 schema，AI 原始长文本不直接进入正常结果 UI。
- 自定义规则只对匹配 scope 的选中文件生效，未匹配语义规则不消耗 Prompt Token。
- 报告展示真实或明确标记为估算的 input/output Token，并能下钻到 batch。
- 在评分或 AI Review 过程中切换 Git/DevDock/Settings，任务不中断，返回后从 SQLite 恢复结果。
