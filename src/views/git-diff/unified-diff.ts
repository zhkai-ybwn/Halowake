export type UnifiedDiffLineKind = 'normal' | 'added' | 'deleted' | 'hunk' | 'note'

export interface UnifiedDiffLine {
  id: string
  kind: UnifiedDiffLineKind
  oldLine: number | null
  newLine: number | null
  text: string
}

export interface UnifiedDiffHunk {
  id: string
  header: string
  lines: UnifiedDiffLine[]
}

export function parseUnifiedDiff(diff: string): UnifiedDiffHunk[] {
  const hunks: UnifiedDiffHunk[] = []
  let current: UnifiedDiffHunk | null = null
  let oldLine = 0
  let newLine = 0

  for (const raw of diff.split(/\r?\n/)) {
    const hunk = raw.match(/^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@(.*)$/)
    if (hunk) {
      oldLine = Number(hunk[1])
      newLine = Number(hunk[2])
      current = { id: `hunk-${hunks.length}`, header: raw, lines: [] }
      hunks.push(current)
      continue
    }

    if (raw.startsWith('diff --git ')) { current = null; continue }
    if (!current) continue

    const id = `${current.id}-line-${current.lines.length}`
    if (raw.startsWith('+')) {
      current.lines.push({ id, kind: 'added', oldLine: null, newLine, text: raw.slice(1) })
      newLine++
    } else if (raw.startsWith('-')) {
      current.lines.push({ id, kind: 'deleted', oldLine, newLine: null, text: raw.slice(1) })
      oldLine++
    } else if (raw.startsWith(' ')) {
      current.lines.push({ id, kind: 'normal', oldLine, newLine, text: raw.slice(1) })
      oldLine++
      newLine++
    } else if (raw.startsWith('\\ No newline at end of file')) {
      current.lines.push({ id, kind: 'note', oldLine: null, newLine: null, text: raw })
    }
  }

  return hunks
}

export const DIFF_ROW_HEIGHT = 20
export function getDiffWindow(total: number, scrollTop: number, height: number, overscan = 16) {
  const first = Math.max(0, Math.floor(scrollTop / DIFF_ROW_HEIGHT) - overscan)
  const start = Math.min(first, Math.max(total - 1, 0))
  const end = Math.min(total, Math.ceil((scrollTop + height) / DIFF_ROW_HEIGHT) + overscan)
  return { start, end: Math.max(start, end), top: start * DIFF_ROW_HEIGHT, bottom: Math.max(0, total - end) * DIFF_ROW_HEIGHT }
}
