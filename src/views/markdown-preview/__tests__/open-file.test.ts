import { flushPromises, shallowMount } from '@vue/test-utils'
import { NDropdown } from 'naive-ui'
import { createPinia } from 'pinia'
import { createMemoryHistory, createRouter } from 'vue-router'
import { expect, it, vi } from 'vitest'
import { i18n } from '@/i18n'
import { useMarkdownHistoryStore } from '@/stores/markdown-history'
import { invokeMock, setTauriInvokeHandler } from '@/test/setup'
import MarkdownPreviewView from '../MarkdownPreviewView.vue'

vi.mock('naive-ui', async importOriginal => ({
  ...await importOriginal<typeof import('naive-ui')>(),
  useMessage: () => ({ error: vi.fn(), success: vi.fn() }),
}))
vi.mock('@/composables/useVisiblePolling', () => ({ useVisiblePolling: vi.fn() }))

function fileInfo(path: string, name: string) {
  return { path, name, content: `# ${name}`, size: name.length + 2, modifiedAt: 1 }
}

async function mountPreview(file?: string) {
  const pinia = createPinia()
  const history = useMarkdownHistoryStore(pinia)
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [{ path: '/markdown', name: 'markdown-preview', component: MarkdownPreviewView }],
  })
  await router.push({ name: 'markdown-preview', query: { file } })
  const wrapper = shallowMount(MarkdownPreviewView, { global: { plugins: [pinia, router, i18n] } })
  return { wrapper, router, history }
}

it('reads a recent file once through the preview route', async () => {
  const file = fileInfo('/notes/recent.md', 'recent.md')
  let finishRead!: (value: typeof file) => void
  const pendingRead = new Promise<typeof file>(resolve => { finishRead = resolve })
  setTauriInvokeHandler(() => pendingRead)
  const { wrapper, router, history } = await mountPreview()
  try {
    history.addRecentFile(file.path, file.name)
    await wrapper.vm.$nextTick()
    const menu = wrapper.getComponent(NDropdown)
    const option = { key: file.path, label: file.name }
    menu.vm.$emit('select', file.path, option)
    await flushPromises()
    expect(router.currentRoute.value.query.file).toBe(file.path)

    menu.vm.$emit('select', file.path, option)
    finishRead(file)
    await flushPromises()
    expect(invokeMock.mock.calls.filter(([cmd]) => cmd === 'read_markdown_file')).toEqual([
      ['read_markdown_file', { filePath: file.path }],
    ])
    expect(wrapper.get('.md-file-name').text()).toBe(file.name)
  } finally {
    wrapper.unmount()
  }
})

it('keeps a newer document when an older read resolves afterwards', async () => {
  const older = fileInfo('/notes/older.md', 'older.md')
  const newer = fileInfo('/notes/newer.md', 'newer.md')
  let finishOlder!: (file: typeof older) => void
  const olderRead = new Promise<typeof older>(resolve => { finishOlder = resolve })
  setTauriInvokeHandler((_command, args) => args?.filePath === older.path ? olderRead : newer)

  const { wrapper, router, history } = await mountPreview(older.path)
  try {
    await router.replace({ query: { file: newer.path } })
    await flushPromises()
    expect(wrapper.get('.md-file-name').text()).toBe(newer.name)

    finishOlder(older)
    await flushPromises()
    expect(wrapper.get('.md-file-name').text()).toBe(newer.name)
    expect(history.recentFiles.map(file => file.path)).toEqual([newer.path])
  } finally {
    wrapper.unmount()
  }
})
