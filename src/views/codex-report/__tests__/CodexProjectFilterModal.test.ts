import { describe, expect, it, vi } from 'vitest'
import { shallowMount } from '@vue/test-utils'
import CodexProjectFilterModal from '../components/CodexProjectFilterModal.vue'

vi.mock('@/composables/useLocale', () => ({ useLocale: () => ({ t: (key: string) => key }) }))
const projects = ['A', 'B'].map(name => ({ name, cwd: `D:/${name}`, sessionCount: 1 }))
function mountFilter(selectedProjects: string[] | null) {
  return shallowMount(CodexProjectFilterModal, {
    props: { show: true, projects, selectedProjects },
    global: { renderStubDefaultSlot: true, stubs: { Icon: true } },
  })
}

describe('explicit project selection', () => {
  it('represents no selection independently of all projects', async () => {
    const wrapper = mountFilter(null)
    const group = () => wrapper.findComponent({ name: 'CheckboxGroup' })
    expect(group().props('value')).toEqual(['A', 'B'])
    await wrapper.findAll('.shortcut-btn')[1]!.trigger('click')
    expect(wrapper.emitted('update:selectedProjects')?.at(-1)).toEqual([[]])
    await wrapper.setProps({ selectedProjects: [] })
    expect(group().props('value')).toEqual([])
    await wrapper.findAll('.shortcut-btn')[0]!.trigger('click')
    expect(wrapper.emitted('update:selectedProjects')?.at(-1)).toEqual([null])
    wrapper.unmount()
  })
  it('inverts the effective all selection and permits unchecking the last project', async () => {
    const wrapper = mountFilter(null)
    await wrapper.findAll('.shortcut-btn')[2]!.trigger('click')
    expect(wrapper.emitted('update:selectedProjects')?.at(-1)).toEqual([[]])
    await wrapper.setProps({ selectedProjects: ['A'] })
    wrapper.findComponent({ name: 'CheckboxGroup' }).vm.$emit('update:value', [])
    expect(wrapper.emitted('update:selectedProjects')?.at(-1)).toEqual([[]])
    wrapper.unmount()
  })
})
