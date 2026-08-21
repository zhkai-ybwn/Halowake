<template>
  <section class="panel-section shortcuts-panel">
    <header><span class="panel-icon"><Icon icon="solar:keyboard-linear" /></span><div><h2>{{ t('settings.shortcuts.title') }}</h2><p>{{ t('settings.shortcuts.description') }}</p></div></header>
    <section v-for="group in groups" :key="group.title" class="shortcut-group">
      <h3>{{ group.title }}</h3>
      <dl><div v-for="item in group.items" :key="item.label"><dt>{{ item.label }}</dt><dd><kbd>{{ item.keys }}</kbd></dd></div></dl>
    </section>
    <footer>{{ t('settings.shortcuts.platformHint') }}</footer>
  </section>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { formatPrimaryShortcut } from '@/utils/platform-shortcuts'

const { t } = useI18n({ useScope: 'global' })
const groups = computed(() => [
  { title: t('settings.shortcuts.general'), items: [
    { label: t('settings.shortcuts.commandPalette'), keys: formatPrimaryShortcut('K') },
    { label: t('settings.shortcuts.settings'), keys: formatPrimaryShortcut(',') },
    { label: t('settings.shortcuts.closePanel'), keys: 'Esc' },
  ] },
  { title: t('settings.shortcuts.git'), items: [
    { label: t('settings.shortcuts.searchFiles'), keys: formatPrimaryShortcut('F') },
    { label: t('settings.shortcuts.refreshRepository'), keys: formatPrimaryShortcut('R') },
    { label: t('settings.shortcuts.commit'), keys: formatPrimaryShortcut('Enter') },
    { label: t('settings.shortcuts.startReview'), keys: formatPrimaryShortcut('Enter') },
  ] },
])
</script>

<style scoped lang="scss">
.shortcuts-panel{overflow:hidden}.shortcuts-panel>header{align-items:center;display:flex;gap:12px;padding:18px 20px}.panel-icon{align-items:center;background:var(--lumina-control-bg);border-radius:8px;color:var(--lumina-primary);display:flex;height:38px;justify-content:center;width:38px}.panel-icon svg{height:20px;width:20px}.shortcuts-panel h2{font-size:16px;margin:0 0 3px}.shortcuts-panel header p{color:var(--lumina-text-secondary);font-size:11px;margin:0}.shortcut-group{border-top:.5px solid var(--lumina-separator);padding:12px 20px}.shortcut-group h3{color:var(--lumina-text-secondary);font-size:10px;letter-spacing:.05em;margin:0 0 6px;text-transform:uppercase}.shortcut-group dl{margin:0}.shortcut-group dl>div{align-items:center;display:flex;justify-content:space-between;min-height:36px}.shortcut-group dl>div+div{border-top:.5px solid var(--lumina-separator)}.shortcut-group dt{font-size:12px}.shortcut-group dd{margin:0}.shortcut-group kbd{background:var(--lumina-control-bg);border:.5px solid var(--lumina-separator-strong);border-radius:5px;box-shadow:0 1px 1px rgb(0 0 0 / 6%);color:var(--lumina-text-secondary);font:10px var(--lumina-font-sans);padding:3px 7px}.shortcuts-panel>footer{border-top:.5px solid var(--lumina-separator);color:var(--lumina-text-tertiary);font-size:10px;padding:10px 20px}
</style>
