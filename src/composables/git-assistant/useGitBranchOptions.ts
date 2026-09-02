import { computed, type Ref } from 'vue'
import type { SelectGroupOption, SelectOption } from 'naive-ui'
import type { GitSnapshot } from '@/services/git/git-service'

type Translate = (key: string) => string
type BranchOption = SelectOption | SelectGroupOption

export function useGitBranchOptions(snapshot: Ref<GitSnapshot | null>, t: Translate) {
  const branchOptions = computed<BranchOption[]>(() => {
    const branches = snapshot.value?.branches ?? []
    const localBranches = branches.filter(branch => branch.kind === 'local')
    const remoteBranches = branches.filter(branch => branch.kind === 'remote' && !branch.name.endsWith('/HEAD'))
    const options: BranchOption[] = []

    if (localBranches.length) {
      options.push({
        type: 'group', key: 'local-group', label: t('gitAssistant.repo.localBranches'),
        children: localBranches.map(branch => ({
          label: branch.current ? `${branch.name} (${t('gitAssistant.repo.currentBranch')})` : branch.name,
          value: `${branch.kind}:${branch.name}`,
        })),
      })
    }
    if (remoteBranches.length) {
      options.push({
        type: 'group', key: 'remote-group', label: t('gitAssistant.repo.remoteBranches'),
        children: remoteBranches.map(branch => ({ label: branch.name, value: `${branch.kind}:${branch.name}` })),
      })
    }
    return options
  })

  const currentBranchValue = computed(() => {
    const branch = snapshot.value?.branches.find(item => item.current)
    return branch ? `${branch.kind}:${branch.name}` : null
  })

  const mergeSourceOptions = computed<BranchOption[]>(() => {
    const branches = (snapshot.value?.branches ?? []).filter(branch => !branch.current && !branch.name.endsWith('/HEAD'))
    const localBranches = branches.filter(branch => branch.kind === 'local')
    const remoteBranches = branches.filter(branch => branch.kind === 'remote')
    const options: BranchOption[] = []

    if (localBranches.length) {
      options.push({
        type: 'group', key: 'local-merge-group', label: t('gitAssistant.repo.localBranches'),
        children: localBranches.map(branch => ({ label: branch.name, value: branch.name })),
      })
    }
    if (remoteBranches.length) {
      options.push({
        type: 'group', key: 'remote-merge-group', label: t('gitAssistant.repo.remoteBranches'),
        children: remoteBranches.map(branch => ({ label: branch.name, value: branch.name })),
      })
    }
    return options
  })

  const mergeModeOptions = computed<SelectOption[]>(() => [
    { label: t('gitAssistant.repo.mergeModeDefault'), value: 'default' },
    { label: t('gitAssistant.repo.mergeModeNoFastForward'), value: 'no-ff' },
  ])

  return { branchOptions, currentBranchValue, mergeSourceOptions, mergeModeOptions }
}
