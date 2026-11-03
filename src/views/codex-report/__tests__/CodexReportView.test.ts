import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, shallowMount } from '@vue/test-utils'
import CodexReportView from '../CodexReportView.vue'
import { loadCodexProjects, loadCodexReportSessions } from '@/services/codex-report-service'
import type { CodexReportSession } from '@/types/codex-report'

vi.mock('@/composables/useLocale', () => ({ useLocale: () => ({ t: (key: string) => key }) }))
vi.mock('naive-ui', async importOriginal => ({
  ...await importOriginal<typeof import('naive-ui')>(),
  useMessage: () => ({ warning: vi.fn(), error: vi.fn() }),
}))
vi.mock('@/services/codex-report-service', () => ({
  loadCodexProjects: vi.fn(), loadCodexReportSessions: vi.fn(), loadCodexReportTemplates: vi.fn(),
}))

const makeSession = (projectName: string, assistantMessages = ['answer']): CodexReportSession => ({
  id: projectName, provider: 'codex', projectName, cwd: `D:/${projectName}`,
  startedAt: '2026-07-01T01:00:00.000Z', endedAt: '2026-07-01T02:00:00.000Z',
  userMessages: ['question'], assistantMessages,
})
const mountView = () => shallowMount(CodexReportView, { global: { stubs: { Icon: true } } })
const record = (wrapper: ReturnType<typeof mountView>) => wrapper.findComponent({ name: 'CodexReportEditor' }).props('workRecord') as string

beforeEach(() => {
  vi.mocked(loadCodexProjects).mockResolvedValue([])
  vi.mocked(loadCodexReportSessions).mockReset()
  vi.mocked(loadCodexReportSessions).mockResolvedValue([makeSession('first'), makeSession('second'), makeSession('partial', [])])
})

describe('work report filter state', () => {
  it('refreshes generated text after project/keyword filters and manual exclusion', async () => {
    const wrapper = mountView()
    await flushPromises()
    expect(record(wrapper)).toContain('纳入会话总数：2')
    wrapper.findComponent({ name: 'CodexProjectFilterModal' }).vm.$emit('update:selectedProjects', ['second'])
    await flushPromises()
    expect(record(wrapper)).toContain('纳入会话总数：1')
    expect(record(wrapper)).not.toContain('项目：first')
    wrapper.findComponent({ name: 'CodexReportFilterStrip' }).vm.$emit('update:keyword', 'no-match')
    await flushPromises()
    expect(record(wrapper)).toContain('纳入会话总数：0')
    wrapper.findComponent({ name: 'CodexReportFilterStrip' }).vm.$emit('update:keyword', '')
    await flushPromises()
    expect(record(wrapper)).toContain('纳入会话总数：1')
    wrapper.findComponent({ name: 'CodexSessionList' }).vm.$emit('update:includedIds', [])
    await flushPromises()
    expect(record(wrapper)).toContain('纳入会话总数：0')
    wrapper.unmount()
  })

  it('applies auto exclusion immediately and preserves manual Markdown', async () => {
    const wrapper = mountView()
    await flushPromises()
    const strip = wrapper.findComponent({ name: 'CodexReportFilterStrip' })
    strip.vm.$emit('update:autoExcludeInvalid', false)
    await flushPromises()
    expect(record(wrapper)).toContain('纳入会话总数：3')
    wrapper.findComponent({ name: 'CodexReportEditor' }).vm.$emit('update:workRecord', 'manually edited')
    strip.vm.$emit('update:autoExcludeInvalid', true)
    await flushPromises()
    expect(record(wrapper)).toBe('manually edited')
    expect(wrapper.findComponent({ name: 'CodexSessionList' }).props('includedSessions')).toHaveLength(2)
    wrapper.unmount()
  })

  it('ignores a slower obsolete response', async () => {
    let resolveOld!: (sessions: CodexReportSession[]) => void
    vi.mocked(loadCodexReportSessions).mockReturnValueOnce(new Promise(resolve => { resolveOld = resolve }))
    const wrapper = mountView()
    await flushPromises()
    wrapper.findComponent({ name: 'CodexReportFilterStrip' }).vm.$emit('update:dateRange', [new Date(2026, 6, 1).getTime(), new Date(2026, 8, 30).getTime()])
    await flushPromises()
    resolveOld([makeSession('obsolete')])
    await flushPromises()
    expect(record(wrapper)).not.toContain('obsolete')
    expect(record(wrapper)).toContain('2026-07-01～2026-09-30')
    wrapper.unmount()
  })
})
