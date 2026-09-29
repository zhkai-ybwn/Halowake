import type { CatalogSkill } from '@/types/skills'

export interface SkillSourceGuide {
  kind: 'SKILL.md' | 'README'
  path: string
  text: string
  url: string
}

const MAX_GUIDE_BYTES = 500_000
const guideCache = new Map<string, SkillSourceGuide[]>()
const pending = new Map<string, Promise<SkillSourceGuide[]>>()

function githubRepository(gitUrl: string): string | null {
  const match = /^https:\/\/github\.com\/([A-Za-z0-9_.-]+)\/([A-Za-z0-9_.-]+?)(?:\.git)?\/?$/.exec(gitUrl)
  return match ? `${match[1]}/${match[2]}` : null
}

async function readFile(repository: string, path: string, kind: SkillSourceGuide['kind']): Promise<SkillSourceGuide | null> {
  const encodedPath = path.split('/').map(encodeURIComponent).join('/')
  let response: Response
  try {
    response = await fetch(`https://raw.githubusercontent.com/${repository}/HEAD/${encodedPath}`, {
      signal: AbortSignal.timeout(5000),
    })
    if (!response.ok && response.status !== 404) throw new Error(`GitHub Raw HTTP ${response.status}`)
  } catch {
    // Omitting a revision makes jsDelivr resolve the repository's default branch.
    response = await fetch(`https://cdn.jsdelivr.net/gh/${repository}/${encodedPath}`, {
      signal: AbortSignal.timeout(8000),
    })
  }
  if (response.status === 404) return null
  if (!response.ok) throw new Error(`Skill guide HTTP ${response.status}`)
  if (Number(response.headers.get('content-length') || 0) > MAX_GUIDE_BYTES) return null

  const text = await response.text()
  if (!text || new TextEncoder().encode(text).length > MAX_GUIDE_BYTES) return null
  return {
    kind,
    path,
    text,
    url: `https://github.com/${repository}/blob/HEAD/${encodedPath}`,
  }
}

async function firstAvailable(repository: string, paths: string[], kind: SkillSourceGuide['kind']): Promise<SkillSourceGuide | null> {
  for (const path of paths) {
    const guide = await readFile(repository, path, kind)
    if (guide) return guide
  }
  return null
}

export async function loadSkillSourceGuides(skill: CatalogSkill): Promise<SkillSourceGuide[]> {
  const repository = githubRepository(skill.gitUrl)
  if (!repository) return []
  const subPath = skill.subPath?.split('/').filter(Boolean) || []
  if (subPath.some(part => part === '.' || part === '..')) return []

  const key = `${repository}/${subPath.join('/')}`
  const cached = guideCache.get(key)
  if (cached) return cached
  const existing = pending.get(key)
  if (existing) return existing

  const task = (async () => {
    const prefix = subPath.length ? `${subPath.join('/')}/` : ''
    const [skillGuide, readme] = await Promise.allSettled([
      firstAvailable(repository, [`${prefix}SKILL.md`], 'SKILL.md'),
      firstAvailable(repository, [
        ...(prefix ? [`${prefix}README.md`, `${prefix}readme.md`] : []),
        'README.md',
        'readme.md',
      ], 'README'),
    ])
    const guides = [skillGuide, readme]
      .filter((result): result is PromiseFulfilledResult<SkillSourceGuide | null> => result.status === 'fulfilled')
      .map(result => result.value)
      .filter((guide): guide is SkillSourceGuide => guide !== null)

    if (guides.length) {
      guideCache.set(key, guides)
      return guides
    }
    const error = [skillGuide, readme].find(result => result.status === 'rejected')
    if (error?.status === 'rejected') throw error.reason
    return []
  })()
  pending.set(key, task)
  try {
    return await task
  } finally {
    pending.delete(key)
  }
}
