import { ref } from 'vue'

interface UseGitConflictDialogOptions {
  getConflictedFilePaths: () => string[]
  markResolved: (filePaths: string[]) => Promise<void>
}

export function useGitConflictDialog(options: UseGitConflictDialogOptions) {
  const conflictDialogOpen = ref(false)
  const conflictSelectedPaths = ref<string[]>([])

  function openConflictDialog() {
    conflictSelectedPaths.value = options.getConflictedFilePaths()
    conflictDialogOpen.value = true
  }

  function toggleConflictSelection(filePath: string, checked: boolean) {
    conflictSelectedPaths.value = checked
      ? [...new Set([...conflictSelectedPaths.value, filePath])]
      : conflictSelectedPaths.value.filter(path => path !== filePath)
  }

  async function handleMarkConflictPathsResolved() {
    await options.markResolved(conflictSelectedPaths.value)
  }

  return {
    conflictDialogOpen,
    conflictSelectedPaths,
    openConflictDialog,
    toggleConflictSelection,
    handleMarkConflictPathsResolved,
  }
}
