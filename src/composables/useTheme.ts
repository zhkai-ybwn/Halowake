import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import { usePreferencesStore, type ThemeMode } from '@/stores/preferences'

export function useTheme() {
  const preferencesStore = usePreferencesStore()
  const { themeMode } = storeToRefs(preferencesStore)

  const theme = computed(() => preferencesStore.resolvedTheme)

  function applyTheme(mode: ThemeMode) {
    preferencesStore.setThemeMode(mode)
  }

  function toggleTheme() {
    preferencesStore.setThemeMode(theme.value === 'light' ? 'dark' : 'light')
  }

  return {
    theme,
    themeMode,
    applyTheme,
    toggleTheme,
  }
}
