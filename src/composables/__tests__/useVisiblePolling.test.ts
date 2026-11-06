/* eslint-disable vue/one-component-per-file -- lifecycle harness uses a parent and cached child */
import { afterEach, describe, expect, it, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { defineComponent, h, KeepAlive, ref, nextTick } from 'vue'
import { useVisiblePolling } from '../useVisiblePolling'

afterEach(() => { vi.useRealTimers(); vi.restoreAllMocks() })

describe('visible page polling', () => {
  it('pauses on deactivation and hidden windows, and resumes only for the active page', async () => {
    vi.useFakeTimers()
    let hidden = false
    vi.spyOn(document, 'hidden', 'get').mockImplementation(() => hidden)
    const task = vi.fn().mockResolvedValue(undefined)
    const show = ref(true)
    const Poller = defineComponent({ setup() { useVisiblePolling(task, 1000); return () => h('div') } })
    const wrapper = mount(defineComponent({ setup: () => () => h(KeepAlive, null, { default: () => show.value ? h(Poller) : null }) }))
    await flushPromises()
    expect(task).toHaveBeenCalledTimes(1)
    show.value = false; await nextTick()
    hidden = true; document.dispatchEvent(new Event('visibilitychange'))
    hidden = false; document.dispatchEvent(new Event('visibilitychange'))
    await vi.advanceTimersByTimeAsync(3000)
    expect(task).toHaveBeenCalledTimes(1)
    show.value = true; await nextTick(); await flushPromises()
    expect(task).toHaveBeenCalledTimes(2)
    hidden = true; document.dispatchEvent(new Event('visibilitychange'))
    await vi.advanceTimersByTimeAsync(3000)
    expect(task).toHaveBeenCalledTimes(2)
    wrapper.unmount()
    expect(vi.getTimerCount()).toBe(0)
  })
  it('does not overlap slow requests or run while disabled', async () => {
    vi.useFakeTimers()
    vi.spyOn(document, 'hidden', 'get').mockReturnValue(false)
    let finish!: () => void
    const enabled = ref(false)
    const task = vi.fn(() => new Promise<void>(resolve => { finish = resolve }))
    const wrapper = mount(defineComponent({ setup() { useVisiblePolling(task, 1000, () => enabled.value); return () => h('div') } }))
    enabled.value = true; await nextTick()
    await vi.advanceTimersByTimeAsync(5000)
    expect(task).toHaveBeenCalledTimes(1)
    finish(); await flushPromises()
    await vi.advanceTimersByTimeAsync(1000)
    expect(task).toHaveBeenCalledTimes(2)
    wrapper.unmount(); finish(); await flushPromises()
    expect(vi.getTimerCount()).toBe(0)
  })
})
