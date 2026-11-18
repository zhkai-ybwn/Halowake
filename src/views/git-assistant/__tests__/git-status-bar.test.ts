import { mount } from '@vue/test-utils'
import { NDropdown } from 'naive-ui'
import { createPinia } from 'pinia'
import { expect, it } from 'vitest'
import { i18n } from '@/i18n'
import GitStatusBar from '../components/GitStatusBar.vue'

it('opens the branch selector beside the current branch instead of from the repository menu', async () => {
  const wrapper = mount(GitStatusBar, {
    global: { plugins: [createPinia(), i18n], stubs: { Icon: true } },
    props: {
      repoPath: '/projects/example',
      branch: 'release_ui_TH_V2.0.0',
      branchLoading: false,
      loading: false,
      fetching: false,
      pushing: false,
      pulling: false,
      summary: {
        total: 0, modified: 0, added: 0, deleted: 0, renamed: 0, copied: 0,
        untracked: 0, conflicted: 0, staged: 0, unstaged: 0,
      },
      repositoryState: null,
      recentRepos: [],
      hasSnapshot: true,
      panelVisibility: { changes: true, diff: true, commit: true },
    },
  })

  try {
    const control = 'button[aria-haspopup="dialog"]'
    const button = wrapper.get<HTMLButtonElement>(control)
    expect(button.element.previousElementSibling?.textContent).toContain('release_ui_TH_V2.0.0')
    expect(button.text()).toBe(i18n.global.t('gitAssistant.repo.switchBranch'))
    await button.trigger('click')
    expect(wrapper.emitted('open-branch-selector')).toHaveLength(1)

    const menuKeys = wrapper.findAllComponents(NDropdown)
      .flatMap(menu => (menu.props('options') ?? []).map(option => option.key))
    expect(menuKeys).toContain('merge')
    expect(menuKeys).not.toContain('branches')

    await wrapper.setProps({ branchLoading: true })
    expect(button.element.disabled).toBe(true)
    await wrapper.setProps({ hasSnapshot: false })
    expect(wrapper.find(control).exists()).toBe(false)
  } finally {
    wrapper.unmount()
  }
})
