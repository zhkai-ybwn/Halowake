import { describe, expect, it } from 'vitest'
import { renderWorkRecord, sessionSelectionKey } from '../codex-report'
import type { CodexReportSession } from '@/types/codex-report'

const session: CodexReportSession = {
  id: 'shared-id', provider: 'codex', projectName: 'project', cwd: 'D:/project',
  startedAt: new Date(2026, 6, 1, 9).toISOString(), endedAt: new Date(2026, 8, 30, 18).toISOString(),
  userMessages: ['question'], assistantMessages: ['answer'],
}

describe('work report output', () => {
  it('shows the full requested date range and dates on multi-day sessions', () => {
    const report = renderWorkRecord([session], new Date(2026, 6, 1).getTime(), new Date(2026, 8, 30).getTime())
    expect(report).toContain('2026-07-01～2026-09-30')
    expect(report).toContain('2026-07-01 09:00–2026-09-30 18:00')
  })
  it('keeps the compact single-day title', () => {
    expect(renderWorkRecord([], new Date(2026, 6, 1).getTime())).toContain('工作记录｜2026-07-01')
  })
  it('keeps same IDs from different providers independent', () => {
    expect(sessionSelectionKey(session)).not.toBe(sessionSelectionKey({ ...session, provider: 'claude' }))
  })
})
