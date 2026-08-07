<template>
  <div class="layout-root">
    <header class="window-titlebar" data-tauri-drag-region @dblclick="toggleMaximize">
      <div class="window-title" data-tauri-drag-region>{{ windowTitle }}</div>

      <div class="window-actions" @mousedown.stop @dblclick.stop>
        <button
          class="window-button"
          type="button"
          :title="t('topbar.minimize')"
          @click="minimizeWindow"
        >
          <span class="caption-icon caption-icon--minimize" aria-hidden="true"></span>
        </button>
        <button class="window-button" type="button" :title="maximizeTitle" @click="toggleMaximize">
          <span
            class="caption-icon"
            :class="isMaximized ? 'caption-icon--restore' : 'caption-icon--maximize'"
            aria-hidden="true"
          ></span>
        </button>
        <button
          class="window-button window-button--close"
          type="button"
          :title="t('topbar.close')"
          @click="closeWindow"
        >
          <span class="caption-icon caption-icon--close" aria-hidden="true"></span>
        </button>
      </div>
    </header>

    <div class="app-area">
      <nav class="sidebar">
        <div class="sidebar-top">
          <img src="@/assets/logo.png" class="sidebar-logo" alt="Lumina" />
        </div>

        <div class="sidebar-nav">
          <button
            v-for="item in navItems"
            :key="item.route"
            class="sidebar-item"
            :class="{ active: route.name === item.route }"
            type="button"
            :title="item.label"
            @click="router.push({ name: item.route })"
          >
            <Icon :icon="item.icon" />
          </button>
        </div>

        <div class="sidebar-bottom">
          <button
            class="sidebar-item"
            :class="{ active: route.name === 'settings' }"
            type="button"
            :title="t('topbar.settings')"
            @click="toggleSettings"
          >
            <Icon icon="solar:settings-linear" />
          </button>
        </div>
      </nav>

      <div class="view-host">
        <router-view v-slot="{ Component }">
          <keep-alive>
            <component :is="Component" />
          </keep-alive>
        </router-view>
      </div>
    </div>

    <n-modal
      v-model:show="exitDialogOpen"
      :auto-focus="false"
      :mask-closable="false"
      :close-on-esc="true"
    >
      <section
        class="exit-dialog"
        role="dialog"
        aria-modal="true"
        :aria-label="t('topbar.exitTitle')"
      >
        <button
          class="exit-dialog-close"
          type="button"
          :aria-label="t('topbar.cancel')"
          @click="exitDialogOpen = false"
        >
          <span class="exit-dialog-close-mark" aria-hidden="true">×</span>
        </button>
        <header class="exit-dialog-heading">
          <span class="exit-dialog-icon" aria-hidden="true"
            ><Icon icon="solar:power-linear"
          /></span>
          <div>
            <h3>{{ t('topbar.exitTitle') }}</h3>
            <p>
              {{ runningProcesses.length ? t('topbar.exitRunningHint') : t('topbar.exitIdleHint') }}
            </p>
          </div>
        </header>
        <div v-if="runningProcesses.length" class="exit-process-list">
          <div v-for="process in runningProcesses" :key="process.id" class="exit-process-row">
            <strong>{{ process.projectName }}</strong>
            <span>{{ process.scriptName }} · PID {{ process.pid }}</span>
          </div>
        </div>
        <footer class="exit-dialog-footer">
          <label class="exit-remember">
            <NCheckbox v-model:checked="rememberChoice">{{ t('topbar.rememberChoice') }}</NCheckbox>
            <span>{{ t('topbar.rememberHint') }}</span>
          </label>
          <div class="exit-actions">
            <button class="secondary" type="button" @click="handleHideToTray">
              <Icon icon="solar:monitor-smartphone-linear" />
              {{ t('topbar.hideToTray') }}
            </button>
            <button class="danger" type="button" :disabled="exiting" @click="exitApplication">
              <Icon icon="solar:power-linear" />
              {{ exiting ? t('topbar.exiting') : t('topbar.exitAndStop') }}
            </button>
          </div>
        </footer>
      </section>
    </n-modal>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useMessage } from 'naive-ui'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import {
  listProjectProcesses,
  stopAllProjectProcesses,
  type ProjectProcessSnapshot,
} from '@/services/project/project-service'
import { usePreferencesStore } from '@/stores/preferences'

const router = useRouter()
const route = useRoute()
const { t } = useI18n({ useScope: 'global' })
const message = useMessage()
const appWindow = getCurrentWindow()
const isMaximized = ref(false)
let unlistenResize: UnlistenFn | null = null
let unlistenCloseRequested: UnlistenFn | null = null
let unlistenTrayExit: UnlistenFn | null = null
const exitDialogOpen = ref(false)
const exiting = ref(false)
const runningProcesses = ref<ProjectProcessSnapshot[]>([])
const rememberChoice = ref(false)
const preferencesStore = usePreferencesStore()

const navItems = computed(() => [
  { route: 'devdock', label: t('workbench.devdock'), icon: 'solar:folder-linear' },
  { route: 'git-assistant', label: t('workbench.git'), icon: 'solar:code-square-linear' },
])

const windowTitle = computed(() => {
  if (route.name === 'devdock') return t('topbar.titleDevDock')
  if (route.name === 'settings') return t('topbar.titleSettings')
  return t('topbar.titleGit')
})
const maximizeTitle = computed(() =>
  isMaximized.value ? t('topbar.restore') : t('topbar.maximize')
)

onMounted(async () => {
  await refreshMaximizedState()
  unlistenResize = await appWindow.onResized(refreshMaximizedState)
  unlistenCloseRequested = await appWindow.onCloseRequested(event => {
    event.preventDefault()
    void handleCloseRequest()
  })
  unlistenTrayExit = await listen('lumina://request-exit', () => {
    void requestExit()
  })
})

onUnmounted(() => {
  unlistenResize?.()
  unlistenCloseRequested?.()
  unlistenTrayExit?.()
})

function toggleSettings() {
  if (route.name === 'settings') {
    router.push({ name: 'devdock' })
    return
  }
  router.push({ name: 'settings' })
}

async function minimizeWindow() {
  await appWindow.minimize()
}

async function toggleMaximize() {
  await appWindow.toggleMaximize()
  await refreshMaximizedState()
}

async function closeWindow() {
  await handleCloseRequest()
}

async function handleCloseRequest() {
  const savedAction = preferencesStore.closeAction

  if (savedAction === 'hideToTray') {
    await hideToTray()
    return
  }

  if (savedAction === 'exit') {
    await exitApplication()
    return
  }

  // Default: show dialog
  await requestExit()
}

async function requestExit() {
  try {
    rememberChoice.value = false
    runningProcesses.value = await listProjectProcesses()
    exitDialogOpen.value = true
  } catch (error) {
    message.error(error instanceof Error ? error.message : String(error))
  }
}

async function hideToTray() {
  exitDialogOpen.value = false
  await appWindow.hide()
}

async function handleHideToTray() {
  if (rememberChoice.value) {
    preferencesStore.setCloseAction('hideToTray')
  }
  await hideToTray()
}

async function exitApplication() {
  if (rememberChoice.value) {
    preferencesStore.setCloseAction('exit')
  }
  exiting.value = true
  try {
    await stopAllProjectProcesses()
    await appWindow.destroy()
  } catch (error) {
    message.error(error instanceof Error ? error.message : String(error), { duration: 8000 })
  } finally {
    exiting.value = false
  }
}

async function refreshMaximizedState() {
  isMaximized.value = await appWindow.isMaximized()
}
</script>

<style scoped lang="scss">
.layout-root {
  background: var(--lumina-bg);
  color: var(--lumina-text);
  display: flex;
  flex-direction: column;
  height: 100vh;
  min-height: 720px;
  min-width: 1120px;
  overflow: hidden;
  width: 100%;
}

.window-titlebar {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-surface-2) 92%, var(--lumina-bg));
  border-bottom: 1px solid var(--lumina-card-border);
  display: flex;
  flex: 0 0 auto;
  height: 32px;
  justify-content: space-between;
  user-select: none;
}

.window-title {
  color: var(--lumina-text-secondary);
  flex: 1;
  font-size: 12px;
  line-height: 32px;
  min-width: 0;
  overflow: hidden;
  padding: 0 12px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.window-actions {
  align-items: center;
  display: flex;
}

.window-button {
  align-items: center;
  background: transparent;
  border: 0;
  color: var(--lumina-text);
  cursor: pointer;
  display: inline-flex;
  height: 32px;
  justify-content: center;
  transition:
    background 0.18s ease,
    color 0.18s ease;
  width: 42px;

  &:hover,
  &.active {
    background: var(--lumina-button-secondary-hover);
    color: var(--lumina-primary);
  }

  svg {
    height: 16px;
    width: 16px;
  }
}

.exit-dialog {
  background: var(--lumina-elevated-bg);
  border: 1px solid var(--lumina-card-border);
  border-radius: var(--lumina-radius-xl);
  box-shadow: var(--lumina-shadow-lg);
  color: var(--lumina-text);
  padding: 24px;
  position: relative;
  width: min(424px, calc(100vw - 32px));

  h3 {
    font-size: 17px;
    font-weight: 650;
    letter-spacing: -0.01em;
    margin: 0;
  }

  p {
    color: var(--lumina-text-secondary);
    font-size: 13px;
    line-height: 1.65;
    margin: 5px 0 0;
  }

  button {
    align-items: center;
    border-radius: var(--lumina-radius-md);
    color: var(--lumina-text);
    cursor: pointer;
    display: inline-flex;
    font-size: 13px;
    font-weight: 600;
    gap: 8px;
    height: 40px;
    justify-content: center;
    padding: 0 16px;
    transition:
      background 0.15s ease,
      border-color 0.15s ease,
      transform 0.15s ease;

    &:hover {
      transform: translateY(-1px);
    }

    &:focus-visible {
      box-shadow: 0 0 0 3px var(--lumina-accent-ring);
      outline: none;
    }

    &:disabled {
      cursor: wait;
      opacity: 0.7;
      transform: none;
    }

    svg {
      height: 17px;
      width: 17px;
    }
  }

  button.secondary {
    background: var(--lumina-surface-2);
    border: 1px solid var(--lumina-card-border);

    &:hover {
      background: var(--lumina-button-secondary-hover);
      border-color: var(--lumina-input-border);
    }
  }

  button.danger {
    background: var(--lumina-danger);
    border-color: var(--lumina-danger);
    color: #fff;

    &:hover {
      background: color-mix(in srgb, var(--lumina-danger) 88%, #000);
    }
  }
}

.exit-dialog-heading {
  align-items: flex-start;
  display: flex;
  gap: 12px;
  padding-right: 32px;
}

.exit-dialog-icon {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-danger) 12%, var(--lumina-surface-2));
  border-radius: var(--lumina-radius-md);
  color: var(--lumina-danger);
  display: inline-flex;
  flex: 0 0 auto;
  height: 32px;
  justify-content: center;
  width: 32px;

  svg {
    height: 18px;
    width: 18px;
  }
}

.exit-dialog-close {
  position: absolute;
  top: 12px;
  right: 12px;
  background: var(--lumina-surface-2) !important;
  border: 1px solid var(--lumina-card-border) !important;
  color: var(--lumina-text) !important;
  cursor: pointer;
  height: 30px;
  width: 30px;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  border-radius: var(--lumina-radius-sm);

  &:hover {
    background: color-mix(in srgb, var(--lumina-danger) 10%, var(--lumina-surface-2)) !important;
    border-color: color-mix(
      in srgb,
      var(--lumina-danger) 36%,
      var(--lumina-card-border)
    ) !important;
    color: var(--lumina-danger) !important;
    transform: none;
  }
}

.exit-dialog-close-mark {
  font-size: 23px;
  font-weight: 300;
  line-height: 1;
  margin-top: -2px;
}

.exit-remember {
  display: flex;
  align-items: center;
  gap: 8px;

  span {
    color: var(--lumina-text-secondary);
    font-size: 12px;
  }
}

.exit-actions {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
}

.exit-dialog-footer {
  display: flex;
  flex-direction: column;
  gap: 14px;
  margin-top: 20px;
}

.exit-process-list {
  background: var(--lumina-surface-2);
  border: 1px solid var(--lumina-card-border);
  border-radius: var(--lumina-radius-md);
  margin-top: 20px;
  max-height: 220px;
  overflow-y: auto;
  padding: 0 12px;
}

.exit-process-row {
  align-items: center;
  display: flex;
  justify-content: space-between;
  min-height: 42px;

  & + & {
    border-top: 1px solid var(--lumina-card-border);
  }

  span {
    color: var(--lumina-text-secondary);
    font-size: 12px;
  }
}

.caption-icon {
  display: inline-block;
  height: 12px;
  position: relative;
  width: 12px;
}

.caption-icon--minimize {
  &::before {
    background: currentcolor;
    bottom: 2px;
    content: '';
    height: 1px;
    left: 1px;
    position: absolute;
    right: 1px;
  }
}

.caption-icon--maximize {
  border: 1.4px solid currentcolor;
}

.caption-icon--restore {
  height: 13px;
  width: 13px;

  &::before,
  &::after {
    border: 1.4px solid currentcolor;
    content: '';
    height: 8px;
    position: absolute;
    width: 8px;
  }

  &::before {
    right: 0;
    top: 0;
  }

  &::after {
    background: var(--lumina-surface-2);
    bottom: 0;
    left: 0;
  }
}

.caption-icon--close {
  &::before,
  &::after {
    background: currentcolor;
    content: '';
    height: 1.4px;
    left: 0;
    position: absolute;
    top: 5px;
    width: 12px;
  }

  &::before {
    transform: rotate(45deg);
  }

  &::after {
    transform: rotate(-45deg);
  }
}

.window-button--close {
  &:hover {
    background: var(--lumina-danger);
    color: #fff;
  }
}

.app-area {
  display: flex;
  flex: 1;
  min-height: 0;
  min-width: 0;
  overflow: hidden;
}

.sidebar {
  align-items: center;
  background: color-mix(in srgb, var(--lumina-surface-2) 80%, var(--lumina-bg));
  border-right: 1px solid var(--lumina-card-border);
  display: flex;
  flex: 0 0 48px;
  flex-direction: column;
  padding: 8px 0;
  user-select: none;
}

.sidebar-top {
  display: flex;
  justify-content: center;
  margin-bottom: 12px;
}

.sidebar-logo {
  border-radius: 6px;
  height: 28px;
  width: 28px;
}

.sidebar-nav {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 4px;
}

.sidebar-bottom {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.sidebar-item {
  align-items: center;
  background: transparent;
  border: 0;
  border-radius: 8px;
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: flex;
  height: 36px;
  justify-content: center;
  transition:
    background 0.15s ease,
    color 0.15s ease;
  width: 36px;

  &:hover {
    background: var(--lumina-button-secondary-hover);
    color: var(--lumina-text);
  }

  &.active {
    background: color-mix(in srgb, var(--lumina-primary) 12%, transparent);
    color: var(--lumina-primary);
  }

  svg {
    height: 18px;
    width: 18px;
  }
}

.view-host {
  flex: 1;
  min-height: 0;
  min-width: 0;
  overflow: hidden;
}
</style>
