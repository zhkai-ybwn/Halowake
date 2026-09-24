import { useI18n } from 'vue-i18n'
import { usePreferencesStore } from '@/stores/preferences'

export function useLocale() {
  const preferencesStore = usePreferencesStore()
  const { t, locale } = useI18n({ useScope: 'global' })

  function applyLocale(nextLocale: 'zh-CN' | 'en-US') {
    preferencesStore.setLocale(nextLocale)
  }

  function toggleLocale() {
    applyLocale(preferencesStore.locale === 'zh-CN' ? 'en-US' : 'zh-CN')
  }

  return {
    locale,
    t,
    applyLocale,
    toggleLocale,
  }
}
