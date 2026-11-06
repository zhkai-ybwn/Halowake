import { describe, expect, it, vi } from 'vitest'
import { setTauriInvokeHandler } from '@/test/setup'
import { loadCodexProjects, loadCodexReportSessions } from '../codex-report-service'
import { loadProjectProcessLogs, type ProjectProcessLogs } from '../project/project-service'

describe('partial report IPC', () => {
  it('returns usable data and exposes source warnings', async () => {
    const onWarnings = vi.fn()
    setTauriInvokeHandler(() => ({ data: [], warnings: ['OpenCode unavailable'] }))
    expect(await loadCodexProjects(onWarnings)).toEqual([])
    expect(onWarnings).toHaveBeenCalledWith(['OpenCode unavailable'])
    expect(await loadCodexReportSessions({ from: '2026-07-01', to: '2026-07-02' }, onWarnings)).toEqual([])
  })
})

describe('versioned process logs', () => {
  it('keeps the same lines when unchanged and requests a full snapshot for another process', async () => {
    const lines = [{ stream: 'stdout' as const, text: 'same', timestamp: 1 }]
    const previous = { revision: 'version-1', process: { id: 'a' }, lines } as ProjectProcessLogs
    const request = vi.fn((_cmd: string, args?: Record<string, unknown>) => ({ revision: 'version-1', process: { id: args?.processId }, lines: args?.previousRevision ? null : lines }))
    setTauriInvokeHandler(request)
    expect((await loadProjectProcessLogs('a', previous)).lines).toBe(lines)
    expect(request.mock.calls[0]?.[1]?.previousRevision).toBe('version-1')
    await loadProjectProcessLogs('b', previous)
    expect(request.mock.calls[1]?.[1]?.previousRevision).toBeNull()
  })
})
