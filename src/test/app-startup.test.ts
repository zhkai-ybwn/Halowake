import { shallowMount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { createMemoryHistory, createRouter } from 'vue-router'
import { expect, it, vi } from 'vitest'
import App from '@/App.vue'
import naive from '@/plugins/naive'
import { invokeMock, setTauriInvokeHandler } from '@/test/setup'

const { checkForUpdates } = vi.hoisted(() => ({ checkForUpdates: vi.fn() }))
vi.mock('@/stores/updater', () => ({ useUpdaterStore: () => ({ checkForUpdates }) }))

it.each([true, false])('runs app-wide startup only in the main window (%s)', async mainWindow => {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [{ path: '/', component: { template: '<div />' } }],
  })
  await router.push('/')
  setTauriInvokeHandler(() => null)
  checkForUpdates.mockClear()
  vi.useFakeTimers()
  const wrapper = shallowMount(App, {
    props: { mainWindow },
    global: { plugins: [createPinia(), router, naive] },
  })
  try {
    await vi.advanceTimersByTimeAsync(3000)
    expect(invokeMock.mock.calls.filter(([cmd]) => cmd === 'get_initial_markdown_file'))
      .toHaveLength(mainWindow ? 1 : 0)
    expect(checkForUpdates).toHaveBeenCalledTimes(mainWindow ? 1 : 0)
  } finally {
    wrapper.unmount()
    vi.useRealTimers()
  }
})
