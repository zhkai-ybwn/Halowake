<template>
  <div class="settings-root">
    <SettingsNav v-model="active" :sections="navSections" />

    <div class="settings-content">
      <header class="settings-header">
        <div>
          <h1>{{ t('settings.title') }}</h1>
          <p>{{ t('settings.description') }}</p>
        </div>
      </header>

      <component :is="activeComponent" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import SettingsNav from './SettingsNav.vue'
import AboutSettingsPanel from './panels/AboutSettingsPanel.vue'
import KeyboardShortcutsSettingsPanel from './panels/KeyboardShortcutsSettingsPanel.vue'
import LocalizationSettingsPanel from './panels/LocalizationSettingsPanel.vue'
import CloseBehaviorSettingsPanel from './panels/CloseBehaviorSettingsPanel.vue'
import ModelSettingsPanel from './panels/ModelSettingsPanel.vue'
import StorageSettingsPanel from './panels/StorageSettingsPanel.vue'
import TaskRoutingSettingsPanel from './panels/TaskRoutingSettingsPanel.vue'
import ThemeSettingsPanel from './panels/ThemeSettingsPanel.vue'

const { t } = useI18n({ useScope: 'global' })
const active = ref('language')

const navSections = computed(() => [
  {
    key: 'app',
    label: t('settings.navSections.app'),
    items: [
      { key: 'language', label: t('settings.nav.language'), icon: 'solar:global-linear' },
      { key: 'theme', label: t('settings.nav.theme'), icon: 'solar:moon-linear' },
      { key: 'closeBehavior', label: t('settings.nav.closeBehavior'), icon: 'solar:power-linear' },
      { key: 'shortcuts', label: t('settings.nav.shortcuts'), icon: 'solar:keyboard-linear' },
      { key: 'storage', label: t('settings.nav.storage'), icon: 'solar:database-linear' },
      { key: 'models', label: t('settings.nav.models'), icon: 'solar:cpu-bolt-linear' },
      { key: 'routing', label: t('settings.nav.routing'), icon: 'solar:routing-3-linear' },
      { key: 'about', label: t('settings.nav.about'), icon: 'solar:info-circle-linear' },
    ],
  },
])

const activeComponent = computed(() => {
  switch (active.value) {
    case 'theme':
      return ThemeSettingsPanel
    case 'closeBehavior':
      return CloseBehaviorSettingsPanel
    case 'shortcuts':
      return KeyboardShortcutsSettingsPanel
    case 'models':
      return ModelSettingsPanel
    case 'storage':
      return StorageSettingsPanel
    case 'routing':
      return TaskRoutingSettingsPanel
    case 'about':
      return AboutSettingsPanel
    default:
      return LocalizationSettingsPanel
  }
})
</script>

<style scoped lang="scss">
.settings-root {
  background: var(--lumina-content-bg);
  border: 0;
  border-radius: 0;
  display: flex;
  height: 100%;
  overflow: hidden;
  width: 100%;
}

.settings-content {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 18px;
  min-height: 0;
  overflow-y: auto;
  padding: 24px clamp(24px, 5vw, 64px);
}

.settings-header {
  max-width: 820px;

  h1 {
    font-size: 22px;
    line-height: 1.2;
    margin: 0 0 8px;
  }

  p {
    color: var(--lumina-text-secondary);
    margin: 0;
  }
}

.settings-content :deep(.panel-section) {
  background: var(--lumina-surface-secondary);
  border-color: var(--lumina-separator);
  border-radius: var(--lumina-radius-md);
  box-shadow: none;
  max-width: 820px;
}
</style>
