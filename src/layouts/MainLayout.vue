<template>
  <div class="layout-root" :class="{ 'sidebar-is-collapsed': sidebarCollapsed, 'platform-macos': isMac }">
    <header class="window-titlebar" data-tauri-drag-region @dblclick="toggleMaximize">
      <div class="titlebar-leading" data-tauri-drag-region>
        <div v-if="isMac" class="macos-window-controls" @mousedown.stop @dblclick.stop>
          <button class="traffic-light traffic-light--close" type="button" :aria-label="t('topbar.close')" @click="closeWindow"><span>×</span></button>
          <button class="traffic-light traffic-light--minimize" type="button" :aria-label="t('topbar.minimize')" @click="minimizeWindow"><span>−</span></button>
          <button class="traffic-light traffic-light--maximize" type="button" :aria-label="maximizeTitle" @click="toggleMaximize"><span>+</span></button>
        </div>
        <WorkbenchIconButton icon="solar:sidebar-minimalistic-linear" :label="t('topbar.toggleSidebar')" :active="!sidebarCollapsed" @click.stop="toggleSidebar" />
        <div class="titlebar-identity" data-tauri-drag-region>
          <img src="@/assets/logo.png" alt="" />
          <div data-tauri-drag-region><strong data-tauri-drag-region>Lumina</strong><span data-tauri-drag-region>{{ currentModuleLabel }}</span></div>
        </div>
      </div>

      <button class="command-trigger" type="button" @click.stop="openCommandPalette">
        <Icon icon="solar:magnifer-linear" /><span>{{ t('topbar.commandPalette') }}</span>
      </button>

      <div class="titlebar-actions" @mousedown.stop @dblclick.stop>
        <div v-if="!isMac" class="windows-window-controls">
          <button class="window-button" type="button" :title="t('topbar.minimize')" @click="minimizeWindow"><span class="caption-icon caption-icon--minimize" aria-hidden="true"></span></button>
          <button class="window-button" type="button" :title="maximizeTitle" @click="toggleMaximize"><span class="caption-icon" :class="isMaximized ? 'caption-icon--restore' : 'caption-icon--maximize'" aria-hidden="true"></span></button>
          <button class="window-button window-button--close" type="button" :title="t('topbar.close')" @click="closeWindow"><span class="caption-icon caption-icon--close" aria-hidden="true"></span></button>
        </div>
      </div>
    </header>

    <div class="app-area">
      <aside class="sidebar">
        <div class="sidebar-section-label">{{ t('workbench.navigation') }}</div>
        <nav class="sidebar-nav" :aria-label="t('workbench.navigation')">
          <button v-for="item in navItems" :key="item.route" class="sidebar-item" :class="{ active: route.name === item.route }" type="button" :title="sidebarCollapsed ? item.label : undefined" @click="router.push({ name: item.route })">
            <Icon :icon="item.icon" /><span>{{ item.label }}</span>
          </button>
        </nav>
        <footer class="sidebar-footer">
          <button class="sidebar-item" :class="{ active: route.name === 'settings' }" type="button" :title="sidebarCollapsed ? t('topbar.settings') : undefined" @click="toggleSettings"><Icon icon="solar:settings-linear" /><span>{{ t('topbar.settings') }}</span></button>
        </footer>
      </aside>

      <main class="view-host">
        <router-view v-slot="{ Component }">
          <transition name="route-fade" mode="out-in"><keep-alive><component :is="Component" /></keep-alive></transition>
        </router-view>
      </main>
    </div>

    <n-modal v-model:show="commandPaletteOpen" :auto-focus="false">
      <section class="command-palette" role="dialog" :aria-label="t('topbar.commandPalette')">
        <header>
          <Icon icon="solar:magnifer-linear" />
          <NInput ref="commandInput" v-model:value="commandQuery" :placeholder="t('topbar.commandPlaceholder')" @keydown.down.prevent="moveCommandSelection(1)" @keydown.up.prevent="moveCommandSelection(-1)" @keydown.enter.prevent="runSelectedCommand" />
        </header>
        <div class="command-results">
          <p class="command-group-label">{{ t('topbar.commandGroupNavigation') }}</p>
          <button v-for="(command, index) in filteredCommands" :key="command.route" type="button" :class="{ selected: index === selectedCommandIndex }" @mouseenter="selectedCommandIndex = index" @click="runCommand(command.route)">
            <span class="command-icon"><Icon :icon="command.icon" /></span>
            <span><strong>{{ command.label }}</strong><small>{{ command.description }}</small></span>
            <Icon icon="solar:arrow-right-linear" />
          </button>
          <p v-if="!filteredCommands.length" class="command-empty">{{ t('topbar.noCommands') }}</p>
        </div>
      </section>
    </n-modal>

    <n-modal v-model:show="exitDialogOpen" :auto-focus="false" :mask-closable="false" :close-on-esc="true">
      <section class="exit-dialog" role="dialog" aria-modal="true" :aria-label="t('topbar.exitTitle')">
        <button class="exit-dialog-close" type="button" :aria-label="t('topbar.cancel')" @click="exitDialogOpen = false">×</button>
        <header class="exit-dialog-heading">
          <span class="exit-dialog-icon" aria-hidden="true"><Icon icon="solar:power-linear" /></span>
          <div><h3>{{ t('topbar.exitTitle') }}</h3><p>{{ runningProcesses.length ? t('topbar.exitRunningHint') : t('topbar.exitIdleHint') }}</p></div>
        </header>
        <div v-if="runningProcesses.length" class="exit-process-list">
          <div v-for="process in runningProcesses" :key="process.id" class="exit-process-row"><strong>{{ process.projectName }}</strong><span>{{ process.scriptName }} · PID {{ process.pid }}</span></div>
        </div>
        <footer class="exit-dialog-footer">
          <label class="exit-remember"><NCheckbox v-model:checked="rememberChoice">{{ t('topbar.rememberChoice') }}</NCheckbox><span>{{ t('topbar.rememberHint') }}</span></label>
          <div class="exit-actions">
            <WorkbenchButton size="large" @click="handleHideToTray"><Icon icon="solar:monitor-smartphone-linear" />{{ t('topbar.hideToTray') }}</WorkbenchButton>
            <WorkbenchButton size="large" variant="danger" :disabled="exiting" @click="exitApplication"><Icon icon="solar:power-linear" />{{ exiting ? t('topbar.exiting') : t('topbar.exitAndStop') }}</WorkbenchButton>
          </div>
        </footer>
      </section>
    </n-modal>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { NInput, useMessage } from 'naive-ui'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import WorkbenchButton from '@/components/workbench/WorkbenchButton.vue'
import WorkbenchIconButton from '@/components/workbench/WorkbenchIconButton.vue'
import { listProjectProcesses, stopAllProjectProcesses, type ProjectProcessSnapshot } from '@/services/project/project-service'
import { usePreferencesStore } from '@/stores/preferences'
import { hasPrimaryModifier, isMacPlatform } from '@/utils/platform-shortcuts'

const router = useRouter()
const route = useRoute()
const { t } = useI18n({ useScope: 'global' })
const message = useMessage()
const appWindow = getCurrentWindow()
const preferencesStore = usePreferencesStore()
const isMac = isMacPlatform
const SIDEBAR_STORAGE_KEY = 'lumina.shell.sidebarCollapsed'
const isMaximized = ref(false)
const sidebarCollapsed = ref(localStorage.getItem(SIDEBAR_STORAGE_KEY) === '1')
const commandPaletteOpen = ref(false)
const commandQuery = ref('')
const selectedCommandIndex = ref(0)
const commandInput = ref<InstanceType<typeof NInput> | null>(null)
const exitDialogOpen = ref(false)
const exiting = ref(false)
const runningProcesses = ref<ProjectProcessSnapshot[]>([])
const rememberChoice = ref(false)
let unlistenResize: UnlistenFn | null = null
let unlistenCloseRequested: UnlistenFn | null = null
let unlistenTrayExit: UnlistenFn | null = null

const navItems = computed(() => [
  { route: 'devdock', label: t('workbench.devdock'), icon: 'solar:folder-with-files-linear', description: t('workbench.devdockDescription') },
  { route: 'git-assistant', label: t('workbench.git'), icon: 'solar:code-square-linear', description: t('workbench.gitDescription') },
  { route: 'codex-report', label: t('workbench.codexReport'), icon: 'solar:notes-linear', description: t('workbench.codexReportDescription') },
  { route: 'ai-quota', label: t('workbench.aiQuota'), icon: 'solar:wallet-money-linear', description: t('workbench.aiQuotaDescription') },
])
const commands = computed(() => [...navItems.value, { route: 'settings', label: t('topbar.settings'), icon: 'solar:settings-linear', description: t('workbench.settingsDescription') }])
const filteredCommands = computed(() => {
  const query = commandQuery.value.trim().toLocaleLowerCase()
  return query ? commands.value.filter(command => `${command.label} ${command.description}`.toLocaleLowerCase().includes(query)) : commands.value
})
const currentModuleLabel = computed(() => commands.value.find(item => item.route === route.name)?.label ?? t('workbench.git'))
const windowTitle = computed(() => route.name === 'devdock' ? t('topbar.titleDevDock') : route.name === 'codex-report' ? t('topbar.titleCodexReport') : route.name === 'ai-quota' ? t('topbar.titleAiQuota') : route.name === 'settings' ? t('topbar.titleSettings') : t('topbar.titleGit'))
const maximizeTitle = computed(() => isMaximized.value ? t('topbar.restore') : t('topbar.maximize'))

watch(windowTitle, title => { document.title = title }, { immediate: true })
watch(filteredCommands, () => { selectedCommandIndex.value = 0 })
watch(commandPaletteOpen, open => { if (open) void nextTick(() => commandInput.value?.focus()); else commandQuery.value = '' })

onMounted(async () => {
  await refreshMaximizedState()
  unlistenResize = await appWindow.onResized(refreshMaximizedState)
  unlistenCloseRequested = await appWindow.onCloseRequested(event => { event.preventDefault(); void handleCloseRequest() })
  unlistenTrayExit = await listen('lumina://request-exit', () => { void requestExit() })
  window.addEventListener('keydown', handleGlobalKeydown)
})
onUnmounted(() => { unlistenResize?.(); unlistenCloseRequested?.(); unlistenTrayExit?.(); window.removeEventListener('keydown', handleGlobalKeydown) })

function handleGlobalKeydown(event: KeyboardEvent) {
  const mod = hasPrimaryModifier(event)
  if (mod && event.key.toLowerCase() === 'k') { event.preventDefault(); openCommandPalette() }
  if (mod && event.key === ',') { event.preventDefault(); void router.push({ name: 'settings' }) }
  if (event.key === 'Escape' && commandPaletteOpen.value) commandPaletteOpen.value = false
}
function toggleSidebar() { sidebarCollapsed.value = !sidebarCollapsed.value; localStorage.setItem(SIDEBAR_STORAGE_KEY, sidebarCollapsed.value ? '1' : '0') }
function openCommandPalette() { commandPaletteOpen.value = true }
function moveCommandSelection(offset: number) { const count = filteredCommands.value.length; if (count) selectedCommandIndex.value = (selectedCommandIndex.value + offset + count) % count }
function runSelectedCommand() { const command = filteredCommands.value[selectedCommandIndex.value]; if (command) runCommand(command.route) }
function runCommand(routeName: string) { commandPaletteOpen.value = false; void router.push({ name: routeName }) }
function toggleSettings() { void router.push({ name: route.name === 'settings' ? 'devdock' : 'settings' }) }
async function minimizeWindow() { await appWindow.minimize() }
async function toggleMaximize() { await appWindow.toggleMaximize(); await refreshMaximizedState() }
async function closeWindow() { await handleCloseRequest() }
async function handleCloseRequest() { if (preferencesStore.closeAction === 'hideToTray') return hideToTray(); if (preferencesStore.closeAction === 'exit') return exitApplication(); await requestExit() }
async function requestExit() { try { rememberChoice.value = false; runningProcesses.value = (await listProjectProcesses()).filter(process => process.status.state === 'running'); exitDialogOpen.value = true } catch (error) { message.error(error instanceof Error ? error.message : String(error)) } }
async function hideToTray() { exitDialogOpen.value = false; await appWindow.hide() }
async function handleHideToTray() { if (rememberChoice.value) preferencesStore.setCloseAction('hideToTray'); await hideToTray() }
async function exitApplication() { if (rememberChoice.value) preferencesStore.setCloseAction('exit'); exiting.value = true; try { await stopAllProjectProcesses(); await appWindow.destroy() } catch (error) { message.error(error instanceof Error ? error.message : String(error), { duration: 8000 }) } finally { exiting.value = false } }
async function refreshMaximizedState() { isMaximized.value = await appWindow.isMaximized() }
</script>

<style scoped lang="scss">
.layout-root { background: var(--lumina-window-bg); color: var(--lumina-text); display: flex; flex-direction: column; height: 100vh; min-height: 720px; min-width: 1120px; overflow: hidden; width: 100%; }
.window-titlebar { align-items: center; background: var(--lumina-toolbar-bg); border-bottom: 0.5px solid var(--lumina-separator); display: grid; flex: 0 0 var(--lumina-titlebar-height); grid-template-columns: minmax(280px, 1fr) minmax(220px, 360px) minmax(280px, 1fr); height: var(--lumina-titlebar-height); padding-left: 10px; user-select: none; backdrop-filter: var(--lumina-vibrancy); }
.titlebar-leading, .titlebar-actions { align-items: center; display: flex; gap: 8px; min-width: 0; }
.titlebar-actions { height: 100%; justify-content: flex-end; }
.titlebar-identity { align-items: center; display: flex; gap: 9px; min-width: 0; }
.titlebar-identity img { border-radius: 6px; height: 26px; width: 26px; }
.titlebar-identity div { display: flex; flex-direction: column; min-width: 0; }
.titlebar-identity strong { font-size: 12px; font-weight: 600; line-height: 15px; }
.titlebar-identity span { color: var(--lumina-text-secondary); font-size: 10px; line-height: 13px; }
.command-trigger { align-items: center; background: var(--lumina-control-bg); border: 0.5px solid var(--lumina-separator); border-radius: var(--lumina-radius-sm); color: var(--lumina-text-secondary); cursor: text; display: grid; gap: 8px; grid-template-columns: auto 1fr auto; height: 30px; padding: 0 8px; transition: background var(--lumina-duration-fast) var(--lumina-ease-out); }
.command-trigger:hover { background: var(--lumina-control-hover); color: var(--lumina-text); }
.command-trigger svg { height: 15px; width: 15px; }
.command-trigger > span { overflow: hidden; text-align: left; text-overflow: ellipsis; white-space: nowrap; }
.macos-window-controls { align-items: center; display: flex; gap: 8px; margin: 0 6px 0 2px; }
.traffic-light { align-items: center; border: 0; border-radius: 50%; color: rgb(50 50 50 / 78%); cursor: default; display: flex; font-size: 10px; height: 12px; justify-content: center; padding: 0; width: 12px; }
.traffic-light span { opacity: 0; transform: translateY(-0.5px); }
.macos-window-controls:hover .traffic-light span { opacity: 1; }
.traffic-light--close { background: #ff5f57; }.traffic-light--minimize { background: #febc2e; }.traffic-light--maximize { background: #28c840; }
.windows-window-controls { align-items: center; align-self: stretch; display: flex; margin-left: 4px; }
.window-button { align-items: center; align-self: stretch; background: transparent; border: 0; color: var(--lumina-text-secondary); cursor: pointer; display: flex; justify-content: center; width: 46px; }
.window-button:hover { background: var(--lumina-control-hover); color: var(--lumina-text); }.window-button--close:hover { background: #c42b1c; color: var(--lumina-on-danger); }
.caption-icon { display: inline-block; height: 12px; position: relative; width: 12px; }.caption-icon--minimize::before { background: currentcolor; bottom: 2px; content: ''; height: 1px; left: 1px; position: absolute; right: 1px; }.caption-icon--maximize { border: 1.2px solid currentcolor; }
.caption-icon--restore::before, .caption-icon--restore::after { border: 1.2px solid currentcolor; content: ''; height: 8px; position: absolute; width: 8px; }.caption-icon--restore::before { right: 0; top: 0; }.caption-icon--restore::after { background: var(--lumina-toolbar-bg); bottom: 0; left: 0; }
.caption-icon--close::before, .caption-icon--close::after { background: currentcolor; content: ''; height: 1.2px; left: 0; position: absolute; top: 5px; width: 12px; }.caption-icon--close::before { transform: rotate(45deg); }.caption-icon--close::after { transform: rotate(-45deg); }
.app-area { display: flex; flex: 1; min-height: 0; min-width: 0; overflow: hidden; }
.sidebar { background: var(--lumina-sidebar-bg); border-right: 0.5px solid var(--lumina-separator); display: flex; flex: 0 0 var(--lumina-sidebar-width); flex-direction: column; min-width: 0; overflow: hidden; padding: 14px 10px 10px; transition: flex-basis var(--lumina-duration-normal) var(--lumina-ease-out); user-select: none; backdrop-filter: var(--lumina-vibrancy); }
.sidebar-section-label { color: var(--lumina-text-tertiary); font-size: 10px; font-weight: 600; letter-spacing: 0.05em; padding: 0 9px 7px; text-transform: uppercase; }.sidebar-nav { display: flex; flex: 1; flex-direction: column; gap: 3px; }.sidebar-footer { border-top: 0.5px solid var(--lumina-separator); display: flex; flex-direction: column; gap: 3px; padding-top: 8px; }
.sidebar-item { align-items: center; background: transparent; border: 0; border-radius: var(--lumina-radius-sm); color: var(--lumina-text-secondary); cursor: pointer; display: flex; gap: 9px; height: 32px; padding: 0 9px; text-align: left; transition: background var(--lumina-duration-fast) var(--lumina-ease-out), color var(--lumina-duration-fast) var(--lumina-ease-out); width: 100%; }
.sidebar-item:hover { background: var(--lumina-control-hover); color: var(--lumina-text); }.sidebar-item.active { background: var(--lumina-control-active); color: var(--lumina-text); font-weight: 500; }.sidebar-item svg { flex: 0 0 auto; height: 17px; width: 17px; }.sidebar-item span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.sidebar-is-collapsed .sidebar { flex-basis: var(--lumina-sidebar-collapsed-width); padding-inline: 10px; }.sidebar-is-collapsed .sidebar-section-label, .sidebar-is-collapsed .sidebar-item span { display: none; }.sidebar-is-collapsed .sidebar-item { justify-content: center; padding: 0; }
.view-host { background: var(--lumina-content-bg); flex: 1; min-height: 0; min-width: 0; overflow: hidden; position: relative; }.route-fade-enter-active, .route-fade-leave-active { transition: opacity var(--lumina-duration-fast) var(--lumina-ease-out), transform var(--lumina-duration-fast) var(--lumina-ease-out); }.route-fade-enter-from { opacity: 0; transform: translateY(3px); }.route-fade-leave-to { opacity: 0; }
.command-palette { background: color-mix(in srgb, var(--lumina-surface-elevated) 92%, transparent); border: 0.5px solid var(--lumina-separator-strong); border-radius: var(--lumina-radius-xl); box-shadow: var(--lumina-shadow-lg); overflow: hidden; width: min(620px, calc(100vw - 48px)); backdrop-filter: var(--lumina-vibrancy); }.command-palette > header { align-items: center; border-bottom: 0.5px solid var(--lumina-separator); display: grid; gap: 10px; grid-template-columns: auto 1fr auto; min-height: 54px; padding: 8px 12px; }.command-palette > header > svg { color: var(--lumina-text-secondary); height: 19px; width: 19px; }.command-palette :deep(.n-input) { --n-border: 0; --n-border-hover: 0; --n-border-focus: 0; --n-box-shadow-focus: none; --n-color: transparent; --n-color-focus: transparent; font-size: 15px; }
.command-results { max-height: 360px; overflow: auto; padding: 8px; }.command-group-label { color: var(--lumina-text-tertiary); font-size: 10px; font-weight: 600; letter-spacing: 0.04em; margin: 4px 8px 6px; text-transform: uppercase; }.command-results button { align-items: center; background: transparent; border: 0; border-radius: var(--lumina-radius-md); color: var(--lumina-text); cursor: pointer; display: grid; gap: 10px; grid-template-columns: auto 1fr auto; min-height: 48px; padding: 6px 9px; text-align: left; width: 100%; }.command-results button.selected { background: var(--lumina-primary-soft); }.command-results button > svg { color: var(--lumina-text-tertiary); height: 15px; width: 15px; }.command-icon { align-items: center; background: var(--lumina-control-bg); border-radius: var(--lumina-radius-sm); display: flex; height: 30px; justify-content: center; width: 30px; }.command-icon svg { height: 17px; width: 17px; }.command-results button > span:nth-child(2) { display: flex; flex-direction: column; gap: 2px; }.command-results strong { font-size: 13px; font-weight: 550; }.command-results small { color: var(--lumina-text-secondary); font-size: 11px; }.command-empty { color: var(--lumina-text-secondary); padding: 32px; text-align: center; }
.exit-dialog { background: var(--lumina-surface-elevated); border: 0.5px solid var(--lumina-separator-strong); border-radius: var(--lumina-radius-xl); box-shadow: var(--lumina-shadow-lg); color: var(--lumina-text); padding: 22px; position: relative; width: min(430px, calc(100vw - 32px)); }.exit-dialog-close { align-items: center; background: var(--lumina-control-bg); border: 0; border-radius: var(--lumina-radius-sm); color: var(--lumina-text-secondary); cursor: pointer; display: flex; font-size: 20px; height: 28px; justify-content: center; position: absolute; right: 12px; top: 12px; width: 28px; }.exit-dialog-close:hover { background: var(--lumina-control-hover); color: var(--lumina-text); }
.exit-dialog-heading { align-items: flex-start; display: flex; gap: 12px; padding-right: 34px; }.exit-dialog-heading h3 { font-size: 17px; margin: 0; }.exit-dialog-heading p { color: var(--lumina-text-secondary); font-size: 12px; line-height: 1.5; margin: 5px 0 0; }.exit-dialog-icon { align-items: center; background: color-mix(in srgb, var(--lumina-danger) 12%, var(--lumina-control-bg)); border-radius: var(--lumina-radius-md); color: var(--lumina-danger); display: flex; height: 34px; justify-content: center; width: 34px; }.exit-dialog-icon svg { height: 18px; width: 18px; }
.exit-process-list { background: var(--lumina-surface-secondary); border-radius: var(--lumina-radius-md); margin-top: 18px; max-height: 220px; overflow: auto; padding: 0 12px; }.exit-process-row { align-items: center; display: flex; justify-content: space-between; min-height: 42px; }.exit-process-row + .exit-process-row { border-top: 0.5px solid var(--lumina-separator); }.exit-process-row span { color: var(--lumina-text-secondary); font-family: var(--lumina-font-mono); font-size: 11px; }.exit-dialog-footer { display: flex; flex-direction: column; gap: 14px; margin-top: 18px; }.exit-remember { align-items: center; display: flex; gap: 8px; }.exit-remember > span { color: var(--lumina-text-secondary); font-size: 11px; }.exit-actions { display: grid; gap: 8px; grid-template-columns: 1fr 1fr; }
</style>
