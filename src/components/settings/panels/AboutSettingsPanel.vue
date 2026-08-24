<template>
  <section class="panel-section about-panel">
    <header class="about-identity">
      <img src="@/assets/logo.png" alt="" />
      <div>
        <h2>Lumina</h2>
        <p>{{ t('settings.about.tagline') }}</p>
      </div>
    </header>

    <dl class="about-details">
      <div class="about-row">
        <dt>
          <span>{{ t('settings.about.version') }}</span>
          <small class="version-value">v{{ applicationVersion }}</small>
        </dt>
        <dd class="version-actions">
          <button
            v-if="!updaterStore.updateAvailable"
            type="button"
            class="check-update-button"
            :disabled="updaterStore.isChecking"
            @click="handleCheckForUpdates"
          >
            <Icon
              :icon="updaterStore.isChecking ? 'solar:spinner-linear' : 'solar:refresh-linear'"
              :class="{ 'spinning': updaterStore.isChecking }"
            />
            {{ updaterStore.isChecking ? t('settings.about.checkingUpdates') : t('settings.about.checkUpdates') }}
          </button>
          <button
            v-else
            type="button"
            class="update-available-button"
            @click="updaterStore.openModal()"
          >
            <span class="pulse-dot" />
            <Icon icon="solar:cloud-download-bold-duotone" />
            {{ t('updater.newVersionBadge', { version: updaterStore.newVersion }) }}
          </button>
        </dd>
      </div>
      <div class="about-row">
        <dt>
          <span>{{ t('settings.about.developer') }}</span>
          <small>zhkai-ybwn</small>
        </dt>
        <dd>
          <button type="button" @click="openLink(DEVELOPER_URL)">
            <Icon icon="mdi:github" />
            {{ t('settings.about.visitGitHub') }}
            <Icon icon="solar:arrow-up-linear" class="external-icon" />
          </button>
        </dd>
      </div>
      <div class="about-row">
        <dt>
          <span>{{ t('settings.about.repository') }}</span>
          <small>zhkai-ybwn/Lumina</small>
        </dt>
        <dd>
          <button type="button" @click="openLink(REPOSITORY_URL)">
            <Icon icon="solar:code-square-linear" />
            {{ t('settings.about.viewSource') }}
            <Icon icon="solar:arrow-up-linear" class="external-icon" />
          </button>
        </dd>
      </div>
      <div class="about-row">
        <dt>{{ t('settings.about.license') }}</dt>
        <dd class="license-value">MIT License</dd>
      </div>
    </dl>
  </section>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useMessage } from 'naive-ui'
import { useI18n } from 'vue-i18n'
import { Icon } from '@iconify/vue'
import { getApplicationVersion, openExternalUrl } from '@/services/app-service'
import { useUpdaterStore } from '@/stores/updater'

const DEVELOPER_URL = 'https://github.com/zhkai-ybwn'
const REPOSITORY_URL = 'https://github.com/zhkai-ybwn/Lumina'

const { t } = useI18n({ useScope: 'global' })
const message = useMessage()
const updaterStore = useUpdaterStore()
const applicationVersion = ref('—')

onMounted(async () => {
  try {
    applicationVersion.value = await getApplicationVersion()
  } catch {
    applicationVersion.value = t('settings.about.versionUnavailable')
  }
})

async function handleCheckForUpdates() {
  try {
    const hasUpdate = await updaterStore.checkForUpdates({ silent: false, openModalIfAvailable: true })
    if (!hasUpdate) {
      message.success(t('updater.alreadyLatest'))
    }
  } catch (error) {
    const detail = error instanceof Error ? error.message : String(error)
    message.error(t('updater.checkFailed', { error: detail }))
  }
}

async function openLink(url: string) {
  try {
    await openExternalUrl(url)
  } catch (error) {
    const detail = error instanceof Error ? error.message : String(error)
    message.error(t('settings.about.openFailed', { error: detail }))
  }
}
</script>

<style scoped lang="scss">
.panel-section {
  background: var(--lumina-surface-secondary);
  border: 1px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-md);
  overflow: hidden;
}

.about-identity {
  align-items: center;
  display: flex;
  gap: 14px;
  padding: 20px;

  img {
    border-radius: var(--lumina-radius-md);
    box-shadow: var(--lumina-shadow-sm);
    height: 52px;
    width: 52px;
  }

  h2 {
    font-size: 20px;
    line-height: 1.25;
    margin: 0 0 4px;
  }

  p {
    color: var(--lumina-text-secondary);
    font-size: 12px;
    margin: 0;
  }
}

.about-details {
  border-top: 1px solid var(--lumina-separator);
  margin: 0;
}

.about-row {
  align-items: center;
  display: grid;
  gap: 24px;
  grid-template-columns: minmax(160px, 1fr) auto;
  min-height: 54px;
  padding: 9px 14px 9px 20px;

  & + & {
    border-top: 1px solid var(--lumina-separator);
  }

  dt {
    display: flex;
    flex-direction: column;
    font-size: 13px;
    gap: 2px;
  }

  dd {
    margin: 0;
  }

  small {
    color: var(--lumina-text-secondary);
    font-family: var(--lumina-font-mono);
    font-size: 10px;
  }

  button {
    align-items: center;
    background: var(--lumina-control-bg);
    border: 1px solid var(--lumina-separator);
    border-radius: var(--lumina-radius-sm);
    color: var(--lumina-text);
    cursor: pointer;
    display: inline-flex;
    font: inherit;
    gap: 6px;
    min-height: 30px;
    padding: 0 10px;
    transition: background var(--lumina-duration-fast) var(--lumina-ease-out), border-color var(--lumina-duration-fast) var(--lumina-ease-out);

    &:hover {
      background: var(--lumina-control-hover);
      border-color: var(--lumina-separator-strong);
    }

    &:focus-visible {
      outline: 2px solid var(--lumina-primary);
      outline-offset: 2px;
    }

    svg {
      height: 15px;
      width: 15px;
    }
  }
}

.external-icon {
  color: var(--lumina-text-tertiary);
  height: 12px !important;
  transform: rotate(45deg);
  width: 12px !important;
}

.version-value,
.license-value {
  color: var(--lumina-text-secondary);
  font-family: var(--lumina-font-mono);
  font-size: 11px;
}

.update-available-button {
  background: color-mix(in srgb, var(--lumina-accent, #39786f) 15%, transparent) !important;
  border-color: color-mix(in srgb, var(--lumina-accent, #39786f) 40%, transparent) !important;
  color: var(--lumina-accent, #39786f) !important;
  font-weight: 500;
  position: relative;

  &:hover {
    background: color-mix(in srgb, var(--lumina-accent, #39786f) 25%, transparent) !important;
  }
}

.pulse-dot {
  animation: pulse-ring 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
  background-color: var(--lumina-accent, #39786f);
  border-radius: 50%;
  height: 6px;
  width: 6px;
}

.spinning {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}

@keyframes pulse-ring {
  0%, 100% {
    opacity: 1;
    transform: scale(1);
  }
  50% {
    opacity: 0.4;
    transform: scale(1.3);
  }
}

@media (max-width: 760px) {
  .about-row {
    align-items: flex-start;
    gap: 8px;
    grid-template-columns: 1fr;
    padding-block: 12px;
  }
}
</style>
