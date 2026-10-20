import type { CatalogSkill } from '@/types/skills'

// 通用多语种开发领域意图与场景同义词映射表
export const SYNONYM_MAP: Record<string, string[]> = {
  // UI & 前端设计
  ui: ['ui', '界面', '前端', 'frontend', 'design', '美化', '排版', 'css', 'tailwind', 'style', '布局', '组件', '审美'],
  ux: ['ux', '体验', '交互', '用户体验', 'design', 'interface'],
  界面: ['ui', '界面', '前端', 'frontend', 'design', 'tailwind', 'css', 'style', 'layout', '排版', '美化', '审美'],
  美化: ['ui', '界面', '美化', 'polish', 'design', 'style', 'css', 'tailwind', 'aesthetic', '打磨', '视觉'],
  前端: ['frontend', '前端', 'ui', 'css', 'vue', 'react', 'tailwind', 'component', 'html', 'web'],
  设计: ['design', '设计', 'ui', 'ux', 'theme', 'layout', 'typography', 'figma', '视觉'],
  排版: ['typography', 'layout', '排版', '间距', 'font', 'spacing', '对齐'],
  样式: ['style', 'styling', 'css', 'tailwind', '样式', '主题', '配色'],
  组件: ['component', '组件', 'widget', 'button', 'card', 'table', 'form', 'controls'],
  暗黑: ['dark', 'dark-mode', 'theme', '暗黑', '深色', '夜间'],
  主题: ['theme', 'palette', 'color', '主题', '调色盘', '配色'],

  // 测试 & 质量验收
  测试: ['test', 'testing', 'qa', 'e2e', 'unit', 'vitest', 'jest', 'playwright', 'cypress', '测试', 'mock', '验收'],
  自动化测试: ['automation', 'playwright', 'cypress', 'cdp', 'browser', 'e2e', '自动化测试'],
  单元测试: ['unit', 'unit-test', 'vitest', 'jest', 'tdd', '单元测试', '单测'],
  验收: ['verification', 'verify', 'qa', 'acceptance', '验收', '回归'],

  // 调试 & 排查
  调试: ['debug', 'debugging', 'troubleshoot', 'diagnos', 'error', 'bug', '调试', '排错', '断点'],
  排查: ['debug', 'troubleshoot', 'diagnos', 'investigate', '排查', '根因', '定位'],
  排错: ['troubleshoot', 'debug', 'diagnos', 'error', 'exception', '排错', '修复'],
  诊断: ['diagnos', 'diagnostic', 'analyze', '诊断', '分析'],
  日志: ['log', 'logging', 'trace', '日志', '切片'],

  // 重构 & 架构
  重构: ['refactor', 'refactoring', 'clean', 'architecture', '重构', '规范', '整洁'],
  架构: ['architecture', 'system', 'spec', 'rfc', 'design', '架构', '方案'],
  规范: ['standard', 'convention', 'guide', '规范', '准则', '最佳实践'],
  需求: ['requirement', 'spec', 'prd', 'story', 'planning', '需求', '头脑风暴', '分析'],

  // 接口 & 后端
  api: ['api', 'rest', 'graphql', 'interface', 'endpoint', '接口', '后端', '数据'],
  接口: ['api', 'interface', 'rest', 'rpc', 'grpc', 'endpoint', '接口'],
  后端: ['backend', 'api', 'server', 'database', 'sql', 'orm', '后端'],
  数据库: ['database', 'db', 'sql', 'postgres', 'mysql', 'prisma', '数据库', '表结构'],

  // 运维 & DevOps
  部署: ['deploy', 'deployment', 'devops', 'docker', 'k8s', 'ci', 'cd', 'kubernetes', '部署', '发布'],
  发布: ['release', 'publish', 'deploy', '发布', '上线', '交付'],
  容器: ['docker', 'container', 'k8s', 'kubernetes', '容器', '集群']
}

/**
 * 将用户输入的查询串扩展为二维同义词词群：
 * 外层为 AND 关系（每个 term 必须命中），内层为 OR 关系（同义词群任一命中即可）
 */
export function expandSearchTerms(query: string): string[][] {
  const rawTerms = query.trim().toLowerCase().split(/\s+/).filter(Boolean)
  if (!rawTerms.length) return []

  return rawTerms.map(term => {
    const group = new Set<string>([term])

    for (const [key, synonyms] of Object.entries(SYNONYM_MAP)) {
      if (term.includes(key) || key.includes(term)) {
        for (const s of synonyms) group.add(s.toLowerCase())
      }
    }

    return Array.from(group)
  })
}

/**
 * 计算技能与用户查询的相关度匹配得分 (Relevance Score)
 */
export function computeRelevanceScore(
  skill: CatalogSkill,
  rawTerms: string[],
  expandedGroups: string[][]
): number {
  if (!rawTerms.length) return 0

  const name = skill.name.toLowerCase()
  const title = (skill.titleZh || '').toLowerCase()
  const category = (skill.category || '').toLowerCase()
  const tags = (skill.tags || []).join(' ').toLowerCase()
  const desc = (skill.description || '').toLowerCase()
  const howToUse = (skill.howToUse || '').toLowerCase()

  let score = 0

  // 1. 原始关键词精确匹配打分 (最高优先级)
  for (const term of rawTerms) {
    if (name === term) score += 150
    else if (name.startsWith(term)) score += 90
    else if (name.includes(term)) score += 60

    if (title.includes(term)) score += 80
    if (tags.includes(term)) score += 40
    if (category.includes(term)) score += 30
    if (desc.includes(term)) score += 20
    if (howToUse.includes(term)) score += 15
  }

  // 2. 同义词意图召回打分 (跨语言与近义词)
  for (const group of expandedGroups) {
    for (const synonym of group) {
      if (rawTerms.includes(synonym)) continue // 原始词已计分，跳过

      if (title.includes(synonym)) score += 35
      if (tags.includes(synonym)) score += 25
      if (name.includes(synonym)) score += 20
      if (desc.includes(synonym)) score += 10
    }
  }

  // 3. 质量与可信度微调加权
  if (skill.badge === 'verified') score += 15
  if (skill.usageGuide?.steps && skill.usageGuide.steps.length > 0) score += 10

  return score
}
