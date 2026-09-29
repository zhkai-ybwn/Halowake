export interface SkillVisual {
  icon: string
  color: string
  bg: string
  border: string
}

/**
 * 为技能提供基于职能、阶段或关键词的个性化视觉锚点（图标与配色）
 * 彻底告别所有技能单一绿魔法棒的同质化视觉荒漠
 */
export function getSkillVisuals(options: {
  name?: string
  description?: string
  stage?: string
  category?: string
}): SkillVisual {
  const name = (options.name || '').toLowerCase()
  const desc = (options.description || '').toLowerCase()
  const category = (options.category || '').toLowerCase()
  const stage = options.stage || ''

  // 1. 深度领域关键词精准匹配
  if (
    name.includes('git') ||
    name.includes('review') ||
    category.includes('git') ||
    category.includes('review') ||
    desc.includes('code review') ||
    desc.includes('pull request') ||
    desc.includes('commit')
  ) {
    return {
      icon: 'solar:branching-paths-up-linear',
      color: '#10b981', // 祖母绿
      bg: 'rgba(16, 185, 129, 0.12)',
      border: 'rgba(16, 185, 129, 0.26)',
    }
  }

  if (
    name.includes('test') ||
    name.includes('qa') ||
    name.includes('check') ||
    name.includes('audit') ||
    name.includes('diagnos') ||
    category.includes('test') ||
    category.includes('qa') ||
    desc.includes('test') ||
    desc.includes('verify') ||
    desc.includes('e2e')
  ) {
    return {
      icon: 'solar:shield-check-linear',
      color: '#f59e0b', // 琥珀黄
      bg: 'rgba(245, 158, 11, 0.12)',
      border: 'rgba(245, 158, 11, 0.26)',
    }
  }

  if (
    name.includes('deploy') ||
    name.includes('release') ||
    name.includes('docker') ||
    name.includes('ci') ||
    name.includes('maintain') ||
    category.includes('deploy') ||
    category.includes('release') ||
    category.includes('ops') ||
    desc.includes('release') ||
    desc.includes('publish') ||
    desc.includes('changelog')
  ) {
    return {
      icon: 'solar:rocket-2-linear',
      color: '#06b6d4', // 青天蓝
      bg: 'rgba(6, 182, 212, 0.12)',
      border: 'rgba(6, 182, 212, 0.26)',
    }
  }

  if (
    name.includes('figma') ||
    name.includes('svg') ||
    name.includes('icon') ||
    name.includes('ui') ||
    name.includes('style') ||
    name.includes('css') ||
    name.includes('animat') ||
    name.includes('ink') ||
    name.includes('frame') ||
    name.includes('vitrine') ||
    name.includes('flow') ||
    category.includes('design') ||
    category.includes('ui') ||
    desc.includes('design') ||
    desc.includes('storybook') ||
    desc.includes('typography') ||
    desc.includes('animation')
  ) {
    return {
      icon: 'solar:palette-linear',
      color: '#8b5cf6', // 梦幻紫
      bg: 'rgba(139, 92, 246, 0.12)',
      border: 'rgba(139, 92, 246, 0.26)',
    }
  }

  if (
    name.includes('prose') ||
    name.includes('echo') ||
    name.includes('brainstorm') ||
    name.includes('copy') ||
    name.includes('doc') ||
    name.includes('spec') ||
    name.includes('write') ||
    category.includes('doc') ||
    category.includes('writing') ||
    desc.includes('writing') ||
    desc.includes('microcopy') ||
    desc.includes('intent') ||
    desc.includes('jtbd')
  ) {
    return {
      icon: 'solar:document-text-linear',
      color: '#3b82f6', // 沉静蓝
      bg: 'rgba(59, 130, 246, 0.12)',
      border: 'rgba(59, 130, 246, 0.26)',
    }
  }

  // 2. 按研发阶段 (SDLC Stage) 回退
  switch (stage) {
    case 'requirement':
      return {
        icon: 'solar:clipboard-list-linear',
        color: '#3b82f6',
        bg: 'rgba(59, 130, 246, 0.12)',
        border: 'rgba(59, 130, 246, 0.26)',
      }
    case 'design':
      return {
        icon: 'solar:palette-linear',
        color: '#8b5cf6',
        bg: 'rgba(139, 92, 246, 0.12)',
        border: 'rgba(139, 92, 246, 0.26)',
      }
    case 'coding':
      return {
        icon: 'solar:code-2-linear',
        color: '#10b981',
        bg: 'rgba(16, 185, 129, 0.12)',
        border: 'rgba(16, 185, 129, 0.26)',
      }
    case 'testing':
      return {
        icon: 'solar:shield-check-linear',
        color: '#f59e0b',
        bg: 'rgba(245, 158, 11, 0.12)',
        border: 'rgba(245, 158, 11, 0.26)',
      }
    case 'delivery':
      return {
        icon: 'solar:rocket-2-linear',
        color: '#06b6d4',
        bg: 'rgba(6, 182, 212, 0.12)',
        border: 'rgba(6, 182, 212, 0.26)',
      }
    default:
      return {
        icon: 'solar:magic-stick-3-linear',
        color: '#0ea5e9',
        bg: 'rgba(14, 165, 233, 0.12)',
        border: 'rgba(14, 165, 233, 0.26)',
      }
  }
}
