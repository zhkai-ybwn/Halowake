<template>
  <NConfigProvider :theme="naiveTheme" :theme-overrides="themeOverrides">
    <n-message-provider>
      <n-dialog-provider>
        <n-notification-provider>
          <router-view />
        </n-notification-provider>
      </n-dialog-provider>
    </n-message-provider>
  </NConfigProvider>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { darkTheme, type GlobalThemeOverrides, NConfigProvider } from 'naive-ui'
import { usePreferencesStore } from '@/stores/preferences'

const preferencesStore = usePreferencesStore()
const naiveTheme = computed(() => preferencesStore.resolvedTheme === 'dark' ? darkTheme : null)

const themeOverrides = computed<GlobalThemeOverrides>(() => {
  const isDark = preferencesStore.resolvedTheme === 'dark'
  const primary = isDark ? '#4fa397' : '#39786f'
  const primaryHover = isDark ? '#62b5a9' : '#2e655d'
  const primaryPressed = isDark ? '#3d867c' : '#25524b'
  const primarySuppl = isDark ? 'rgba(79, 163, 151, 0.2)' : 'rgba(57, 120, 111, 0.14)'

  return {
    common: {
      primaryColor: primary,
      primaryColorHover: primaryHover,
      primaryColorPressed: primaryPressed,
      primaryColorSuppl: primarySuppl,
      borderRadius: '6px',
      borderRadiusSmall: '4px',
    },
    Button: {
      colorPrimary: primary,
      colorHoverPrimary: primaryHover,
      colorPressedPrimary: primaryPressed,
      colorFocusPrimary: primaryHover,
      textColorPrimary: '#ffffff',
      borderPrimary: `1px solid ${primary}`,
      borderHoverPrimary: `1px solid ${primaryHover}`,
      borderPressedPrimary: `1px solid ${primaryPressed}`,
      borderFocusPrimary: `1px solid ${primaryHover}`,
    },
    DatePicker: {
      itemColorActive: primary,
      itemTextColorActive: '#ffffff',
      itemColorIncluded: primarySuppl,
      itemTextColorIncluded: isDark ? '#f5f5f7' : '#1d1d1f',
      itemColorCurrent: primarySuppl,
      itemTextColorCurrent: primary,
      itemBorderRadius: '4px',
    },
    Checkbox: {
      colorChecked: primary,
      borderChecked: `1px solid ${primary}`,
      checkMarkColor: '#ffffff',
    },
  }
})
</script>
