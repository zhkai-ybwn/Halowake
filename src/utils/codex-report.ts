import type { CodexReportSession } from '@/types/codex-report'

export const DEFAULT_WEB_AI_PROMPT = `请将下方 AI 工具（Codex / Claude Code / Antigravity / OpenCode）工作记录整理为一份**简洁、专业、适合日报及绩效评估的中文工作总结**。

要求：

1. **严格基于事实**
   * 不编造未完成事项、业务收益、效率数据、工作时长或个人贡献。
   * 不根据 AI 会话时间推断实际工作量。

2. **只写“今日完成”**
   * 不生成“待跟进”“明日计划”等无事实依据的内容。
   * 只整理已有明确结果的工作事项。

3. **突出成果，不写过程**
   * 优先表达：**完成了什么、解决了什么、覆盖哪些范围、最终结果如何**。
   * 不罗列文件路径、代码行号、命令执行过程等无关细节。

4. **体现实际工作价值**
   * 在事实支持的前提下，优先体现：
     * 前后端、接口、数据库等跨层修改；
     * 查询、列表、详情、搜索等覆盖范围；
     * 多语言、多状态、多场景适配；
     * 兼容性处理；
     * 测试、校验等明确结果。
   * 不夸大，不使用“显著提升”“重大突破”“极大优化”等无事实依据的表述。

5. **合并跨工具重复事项**
   * 按项目归类。
   * 多个工具相同目标下的修改合并成一条成果。
   * 一个会话包含多个独立成果时可以拆分。

6. **尽可能精简**
   * 每个项目优先控制在 **1～3 条**。
   * 每条尽量控制在 **1～2 句话**。
   * 删除重复描述和实现细节。
   * 保留能体现工作范围、技术处理和交付结果的信息。

## 输出格式

# 工作日报｜YYYY-MM-DD

## 今日完成

### 项目名称

1. **成果标题**：简洁描述完成事项、覆盖范围和结果。
2. **成果标题**：……

## 最终要求

* 内容可直接提交日报；
* 专业、客观、有成果感；
* 在不改变事实的前提下，尽可能准确体现实际工作贡献；
* **宁可少写，不要长篇大论。**`

export const STANDUP_PROMPT_TEMPLATE = `请根据下方 AI 辅助编程工作记录，整理为一份**敏捷开发每日站会 (Daily Standup)** 发言：

## 格式要求：
- **【昨日/今日完成】**：按项目列出核心产出与解决的问题（1-3条，精简有成果感）。
- **【今日计划】**：根据已完成上下文简要列出合理的下一步跟进项。
- **【风险与阻塞】**：无（或如有未解决报错则简要提炼）。`

export const TECH_SUMMARY_PROMPT_TEMPLATE = `请将下方工作记录整理为一份**技术攻坚与重构小结**，分模块提炼核心技术点、修改范围、架构/逻辑变动及验证结果。语言风格严谨、技术向、要点清晰。`

export function getProviderLabel(provider: string) {
  switch (provider?.toLowerCase()) {
    case 'codex':
      return 'Codex CLI'
    case 'claude':
      return 'Claude Code'
    case 'antigravity':
      return 'Antigravity'
    case 'opencode':
      return 'OpenCode'
    default:
      return provider || 'AI Tool'
  }
}

export function getProviderIcon(provider: string) {
  switch (provider?.toLowerCase()) {
    case 'codex':
      return 'solar:code-square-linear'
    case 'claude':
      return 'solar:magic-stick-3-linear'
    case 'antigravity':
      return 'solar:planet-linear'
    case 'opencode':
      return 'solar:terminal-linear'
    default:
      return 'solar:cpu-bolt-linear'
  }
}

export function getProjectName(cwd: string | null) {
  if (!cwd) return '未识别项目'
  const segments = cwd.split(/[\\/]/).filter(Boolean)
  return segments.at(-1) || cwd
}

export function hasWorkContent(session: CodexReportSession) {
  return session.userMessages.length > 0 && session.assistantMessages.length > 0
}

export function matchesKeyword(session: CodexReportSession, keyword: string) {
  const query = keyword.trim().toLocaleLowerCase()
  if (!query) return true
  return [session.cwd, session.projectName, session.provider, ...session.userMessages, ...session.assistantMessages]
    .filter((value): value is string => Boolean(value))
    .some(value => value.toLocaleLowerCase().includes(query))
}

export function formatSessionTime(timestamp: string) {
  try {
    return new Intl.DateTimeFormat('zh-CN', {
      hour: '2-digit',
      minute: '2-digit',
      hour12: false,
    }).format(new Date(timestamp))
  } catch {
    return timestamp
  }
}

export function formatReportDate(timestamp: number) {
  try {
    return new Intl.DateTimeFormat('zh-CN', { year: 'numeric', month: '2-digit', day: '2-digit' })
      .format(new Date(timestamp))
      .replaceAll('/', '-')
  } catch {
    return String(timestamp)
  }
}

function sessionLabel(session: CodexReportSession) {
  const firstRequest = session.userMessages[0]?.replaceAll(/\s+/g, ' ').trim()
  if (!firstRequest) return session.id || '未命名会话'
  return firstRequest.length > 72 ? `${firstRequest.slice(0, 72)}…` : firstRequest
}

export function renderWorkRecord(sessions: CodexReportSession[], reportDate: number) {
  const projectGroups = new Map<string, CodexReportSession[]>()
  const providerStats = new Map<string, number>()

  for (const session of sessions) {
    const project = session.projectName || getProjectName(session.cwd)
    projectGroups.set(project, [...(projectGroups.get(project) ?? []), session])
    
    const prov = getProviderLabel(session.provider)
    providerStats.set(prov, (providerStats.get(prov) ?? 0) + 1)
  }

  const times = sessions
    .flatMap(session => [session.startedAt, session.endedAt])
    .filter(Boolean)
    .sort()

  const providerSummaryStr = [...providerStats.entries()]
    .map(([p, count]) => `${p} (${count})`)
    .join(' · ')

  const header = [
    `# AI 辅助编程工作记录｜${formatReportDate(reportDate)}`,
    '',
    '## 汇总概况',
    `- 纳入会话总数：${sessions.length} 个`,
    `- 涉及项目：${projectGroups.size} 个`,
    providerSummaryStr ? `- 工具来源分布：${providerSummaryStr}` : '',
    times.length
      ? `- 记录时间跨度：${formatSessionTime(times[0])}–${formatSessionTime(times.at(-1) ?? times[0])}`
      : '',
  ].filter(Boolean)

  const body = [...projectGroups.entries()].flatMap(([project, groupedSessions]) => [
    '',
    `## 📁 项目：${project}`,
    ...groupedSessions.flatMap(session => [
      '',
      `### [${getProviderLabel(session.provider)}] ${formatSessionTime(session.startedAt)}–${formatSessionTime(session.endedAt)}｜${sessionLabel(session)}`,
      ...session.userMessages.map(message => `- 任务意图：${message}`),
      ...session.assistantMessages.map(message => `- 完成结论：${message}`),
    ]),
  ])

  return [...header, ...body].join('\n').trim()
}

