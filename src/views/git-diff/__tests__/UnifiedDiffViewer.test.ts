import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import UnifiedDiffViewer from '../UnifiedDiffViewer.vue'
import { DIFF_ROW_HEIGHT, getDiffWindow, parseUnifiedDiff } from '../unified-diff'

describe('large diff window', () => {
  it('renders a bounded number of rows and permits reaching the last line', async () => {
    const count = 50_000
    const diff = `diff --git a/test b/test\n--- a/test\n+++ b/test\n@@ -0,0 +1,${count} @@\n` + Array.from({ length: count }, (_, i) => `+line ${i}`).join('\n')
    const wrapper = mount(UnifiedDiffViewer, { props: { diff } })
    await nextTick()
    expect(wrapper.findAll('.diff-line').length).toBeLessThan(100)
    const pane = wrapper.find('.code-pane').element as HTMLElement
    pane.scrollTop = count * DIFF_ROW_HEIGHT - 600
    await wrapper.find('.code-pane').trigger('scroll')
    await new Promise(resolve => requestAnimationFrame(resolve))
    await nextTick()
    expect(wrapper.findAll('.diff-line').length).toBeLessThan(100)
    expect(wrapper.text()).toContain('line 49999')
    wrapper.unmount()
  })
  it('preserves changed lines that look like file headers', () => {
    const hunks = parseUnifiedDiff('diff --git a/test b/test\n--- a/test\n+++ b/test\n@@ -1 +1 @@\n--- old content\n+++ new content\n')
    expect(hunks[0]?.lines.map(line => line.text)).toEqual(['-- old content', '++ new content'])
  })
  it('keeps spacers consistent at both bounds', () => {
    expect(getDiffWindow(0, 0, 600)).toEqual({ start: 0, end: 0, top: 0, bottom: 0 })
    const range = getDiffWindow(10_000, 199_400, 600)
    expect(range.end).toBe(10_000)
    expect(range.top + (range.end - range.start) * DIFF_ROW_HEIGHT + range.bottom).toBe(200_000)
  })
})
