import { defineStore } from 'pinia'
import { ref } from 'vue'
import { GIT_REPO_STORAGE_KEY } from '@/constants/git'

function readStoredProjectPath(): string {
  try {
    return localStorage.getItem(GIT_REPO_STORAGE_KEY)?.trim() || ''
  } catch {
    return ''
  }
}

export const useWorkspaceStore = defineStore('workspace', () => {
  const currentProjectPath = ref<string>(readStoredProjectPath())

  function setCurrentProjectPath(path: string | null | undefined) {
    const clean = (path ?? '').trim()
    currentProjectPath.value = clean
    try {
      if (clean) {
        localStorage.setItem(GIT_REPO_STORAGE_KEY, clean)
      } else {
        localStorage.removeItem(GIT_REPO_STORAGE_KEY)
      }
    } catch {
      // ignore storage errors in restricted environments
    }
  }

  function refreshFromStorage(): string {
    const latest = readStoredProjectPath()
    currentProjectPath.value = latest
    return latest
  }

  return {
    currentProjectPath,
    setCurrentProjectPath,
    refreshFromStorage,
  }
})
