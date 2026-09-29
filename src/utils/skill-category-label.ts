import { i18n } from '@/i18n'

const CATEGORY_KEYS: Record<string, string> = {
  'UI & UX Design': 'ui',
  'Requirements & Architecture': 'requirements',
  'DevOps & Delivery': 'devops',
  'Testing & QA': 'testing',
  other: 'other',
  'writing-docs-content': 'writing',
  'backend-api': 'backend',
  'devtools-automation': 'tools',
  'code-review-quality': 'review',
  security: 'security',
  'agent-workflow-meta': 'workflow',
  'ai-ml-llm': 'ai',
  'devops-cloud-infra': 'cloud',
  'productivity-business': 'business',
  'web-frontend': 'frontend',
  database: 'database',
  'data-analytics': 'analytics',
  'testing-qa': 'testing',
  'crypto-defi-finance': 'finance',
  mobile: 'mobile',
}

export function skillCategoryLabel(category: string): string {
  const key = CATEGORY_KEYS[category]
  return key ? i18n.global.t(`skills.review.categories.${key}`) : category.replace(/[-_]/g, ' ')
}
