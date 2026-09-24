<template>
  <NModal
    v-model:show="exploreStore.sourcesModalVisible"
    :mask-closable="!exploreStore.isSyncing"
    :close-on-esc="!exploreStore.isSyncing"
    transform-origin="center"
  >
    <div class="sources-manager-modal" role="dialog" aria-label="技能数据源与注册表设置">
      <!-- Modal Header -->
      <header class="sources-header">
        <div class="header-icon-box">
          <Icon icon="solar:database-bold" class="header-icon" />
        </div>
        <div class="header-text">
          <h3 class="sources-title">开源技能注册表与数据源</h3>
          <p class="sources-subtitle">
            订阅由独立数据中枢（halowake-skills-registry）清洗与认证的真实技能库，开箱即用。
          </p>
        </div>
        <button
          type="button"
          class="sources-close-btn"
          :disabled="exploreStore.isSyncing"
          @click="exploreStore.closeSourcesModal"
        >
          <Icon icon="solar:close-circle-linear" />
        </button>
      </header>

      <!-- Modal Body -->
      <div class="sources-body">
        <!-- Section 1: 技能注册表服务地址配置 -->
        <section class="registry-config-section">
          <div class="section-title-row">
            <span class="section-label">
              <Icon icon="solar:planet-2-linear" />
              <span>技能市场数据中枢源 (Registry Endpoint)</span>
            </span>
            <span class="section-hint">静态 JSON 索引分发，免除客户端采集开销</span>
          </div>

          <div class="registry-url-box">
            <div class="url-input-wrap">
              <input
                v-model="tempRegistryUrl"
                type="text"
                class="macos-input"
                placeholder="https://.../registry.json"
                :disabled="exploreStore.isSyncing || isTesting"
              />
              <button
                type="button"
                class="test-conn-btn"
                :disabled="!tempRegistryUrl.trim() || isTesting || exploreStore.isSyncing"
                @click="handleTestConnection"
              >
                <Icon
                  :icon="isTesting ? 'solar:restart-linear' : 'solar:link-circle-linear'"
                  :class="{ 'spin-glyph': isTesting }"
                />
                <span>{{ isTesting ? '测试中…' : '测试连通性' }}</span>
              </button>
            </div>

            <!-- 快捷预设地址切换 -->
            <div class="presets-row">
              <span class="preset-label">快捷预设源：</span>
              <button
                type="button"
                class="preset-chip"
                :class="{ active: tempRegistryUrl === DEFAULT_REGISTRY_URL }"
                @click="tempRegistryUrl = DEFAULT_REGISTRY_URL"
              >
                官方 GitHub (Raw)
              </button>
              <button
                type="button"
                class="preset-chip"
                :class="{ active: tempRegistryUrl === CDN_REGISTRY_URL }"
                @click="tempRegistryUrl = CDN_REGISTRY_URL"
              >
                jsDelivr CDN (国内镜像)
              </button>
              <button
                v-if="tempRegistryUrl !== DEFAULT_REGISTRY_URL"
                type="button"
                class="preset-chip preset-chip--reset"
                @click="handleResetDefault"
              >
                恢复默认
              </button>
            </div>

            <!-- 连通性测试结果提示 -->
            <div v-if="testResult" class="test-feedback-pill" :class="testResult.ok ? 'success' : 'error'">
              <Icon :icon="testResult.ok ? 'solar:check-circle-bold' : 'solar:close-circle-bold'" />
              <span>{{ testResult.message }}</span>
            </div>
          </div>
        </section>

        <!-- Section 2: 独立采集架构说明卡片 -->
        <section class="architecture-banner">
          <div class="banner-badge">
            <Icon icon="solar:shield-check-bold" />
            <span>架构设计理念</span>
          </div>
          <h4 class="banner-title">数据中枢与客户端运行时彻底解耦</h4>
          <p class="banner-desc">
            为了避免用户花心智手动全网翻找、鉴别与克隆技能，Halowake 采用<strong>“货架模式”</strong>：
          </p>
          <ul class="banner-points">
            <li>
              <strong>后台自动搜集</strong>：由独立的开源数据仓库（halowake-skills-registry）定时运行 GitHub Actions 全网采集。
            </li>
            <li>
              <strong>真实指标背书</strong>：通过 GitHub 官方 API 实时拉取真实 Stars、Forks、最近提交与 Issue 关闭率，杜绝人工伪造数据。
            </li>
            <li>
              <strong>开箱即用体验</strong>：Halowake 客户端专注本地装配与更新，用户一键安装即可无缝分发至 Antigravity、Cursor、Claude 等环境。
            </li>
          </ul>
        </section>

        <!-- Section 3: GitHub API 访问鉴权与频控扩容 (可选) -->
        <section class="token-settings-section">
          <div class="token-header-row" @click="tokenCollapsible = !tokenCollapsible">
            <div class="token-title-group">
              <Icon icon="solar:shield-keyhole-linear" class="key-icon" />
              <span class="token-title">GitHub API 访问令牌 (可选)</span>
              <span class="token-badge">{{ hasCustomToken ? '已配置 Token' : '公共匿名模式' }}</span>
            </div>
            <Icon
              :icon="tokenCollapsible ? 'solar:alt-arrow-up-linear' : 'solar:alt-arrow-down-linear'"
              class="collapse-icon"
            />
          </div>

          <div v-show="tokenCollapsible" class="token-body">
            <p class="token-desc">
              未鉴权匿名请求限额为 60 次/小时。配置个人访问令牌（Personal Access Token，仅需只读 public_repo 权限）可提升至
              <strong>5,000 次/小时</strong>，用于大批量或高频访问。
            </p>

            <div class="token-input-row">
              <input
                v-model="tempToken"
                :type="showTokenPlain ? 'text' : 'password'"
                class="macos-input token-input"
                placeholder="ghp_xxxxxxxxxxxxxxxxxxxx"
              />
              <button
                type="button"
                class="toggle-eye-btn"
                :title="showTokenPlain ? '隐藏' : '显示'"
                @click="showTokenPlain = !showTokenPlain"
              >
                <Icon :icon="showTokenPlain ? 'solar:eye-closed-linear' : 'solar:eye-linear'" />
              </button>
              <button
                type="button"
                class="save-token-btn"
                @click="handleSaveToken"
              >
                保存令牌
              </button>
            </div>
          </div>
        </section>
      </div>

      <!-- Modal Footer -->
      <footer class="sources-footer">
        <div class="footer-left">
          <template v-if="exploreStore.syncStatusMessage">
            <span class="sync-summary-text">{{ exploreStore.syncStatusMessage }}</span>
          </template>
          <template v-else-if="exploreStore.lastSyncTime">
            <span class="sync-summary-text">
              本地最近同步：{{ formatTimeAgo(exploreStore.lastSyncTime) }}
            </span>
          </template>
        </div>

        <div class="footer-right">
          <button
            type="button"
            class="footer-btn footer-btn--secondary"
            :disabled="exploreStore.isSyncing"
            @click="exploreStore.closeSourcesModal"
          >
            完成
          </button>

          <button
            type="button"
            class="footer-btn footer-btn--primary"
            :disabled="exploreStore.isSyncing || isTesting"
            @click="handleSaveAndSync"
          >
            <Icon icon="solar:restart-linear" :class="{ 'spin-glyph': exploreStore.isSyncing }" />
            <span>{{ exploreStore.isSyncing ? '正在同步注册表…' : '保存并立即同步' }}</span>
          </button>
        </div>
      </footer>
    </div>
  </NModal>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { NModal, useMessage } from 'naive-ui'
import { Icon } from '@iconify/vue'
import { useSkillExploreStore } from '@/stores/skill-explore'
import { CDN_REGISTRY_URL, DEFAULT_REGISTRY_URL } from '@/services/skills/skill-catalog-service'

const exploreStore = useSkillExploreStore()
const message = useMessage()

// 注册表地址状态
const tempRegistryUrl = ref(exploreStore.registryUrl || DEFAULT_REGISTRY_URL)
const isTesting = ref(false)
const testResult = ref<{ ok: boolean; message: string } | null>(null)

// Token 设置状态
const tokenCollapsible = ref(false)
const tempToken = ref(exploreStore.githubToken || '')
const showTokenPlain = ref(false)

const hasCustomToken = computed(() => Boolean(exploreStore.githubToken))

watch(
  () => exploreStore.sourcesModalVisible,
  visible => {
    if (visible) {
      tempRegistryUrl.value = exploreStore.registryUrl || DEFAULT_REGISTRY_URL
      testResult.value = null
    }
  }
)

function formatTimeAgo(timestamp: number): string {
  const diffSec = Math.floor((Date.now() - timestamp) / 1000)
  if (diffSec < 60) return '刚刚'
  if (diffSec < 3600) return `${Math.floor(diffSec / 60)} 分钟前`
  if (diffSec < 86400) return `${Math.floor(diffSec / 3600)} 小时前`
  return `${Math.floor(diffSec / 86400)} 天前`
}

async function handleTestConnection() {
  const url = tempRegistryUrl.value.trim()
  if (!url) return

  isTesting.value = true
  testResult.value = null
  try {
    const res = await exploreStore.testRegistry(url)
    testResult.value = {
      ok: res.ok,
      message: res.message,
    }
    if (res.ok) {
      message.success(res.message)
    } else {
      message.warning(res.message)
    }
  } catch (err: unknown) {
    testResult.value = {
      ok: false,
      message: (err as Error)?.message || '请求失败，请检查网络或地址有效性',
    }
  } finally {
    isTesting.value = false
  }
}

function handleResetDefault() {
  tempRegistryUrl.value = DEFAULT_REGISTRY_URL
  testResult.value = null
}

function handleSaveToken() {
  exploreStore.updateGitHubToken(tempToken.value.trim())
  message.success(
    tempToken.value.trim()
      ? '已保存 GitHub Token，API 频控已扩展至 5,000次/小时'
      : '已清除自定义 Token'
  )
}

async function handleSaveAndSync() {
  const cleanUrl = tempRegistryUrl.value.trim() || DEFAULT_REGISTRY_URL
  exploreStore.updateRegistryUrl(cleanUrl)

  try {
    const res = await exploreStore.syncRegistry(cleanUrl)
    message.success(res.message || `同步完成！已汇聚 ${res.totalCount} 个技能。`)
    exploreStore.closeSourcesModal()
  } catch (err: unknown) {
    message.error((err as Error)?.message || '同步注册表失败')
  }
}
</script>

<style scoped lang="scss">
.sources-manager-modal {
  background: var(--lumina-surface-elevated, #ffffff);
  border: 0.5px solid var(--lumina-separator-strong);
  border-radius: var(--lumina-radius-lg, 12px);
  box-shadow: var(--lumina-shadow-lg, 0 24px 64px rgba(0, 0, 0, 0.28));
  color: var(--lumina-text);
  display: flex;
  flex-direction: column;
  max-width: 94vw;
  overflow: hidden;
  width: 620px;
}

.sources-header {
  align-items: center;
  border-bottom: 0.5px solid var(--lumina-separator);
  display: flex;
  gap: 12px;
  padding: 16px 20px;
}

.header-icon-box {
  align-items: center;
  background: var(--lumina-primary-soft);
  border-radius: var(--lumina-radius-md, 8px);
  color: var(--lumina-primary);
  display: flex;
  flex-shrink: 0;
  font-size: 20px;
  height: 38px;
  justify-content: center;
  width: 38px;
}

.header-text {
  flex: 1;
  min-width: 0;
}

.sources-title {
  font-size: 15px;
  font-weight: 650;
  margin: 0;
}

.sources-subtitle {
  color: var(--lumina-text-secondary);
  font-size: 12px;
  line-height: 1.4;
  margin: 3px 0 0;
}

.sources-close-btn {
  align-items: center;
  background: transparent;
  border: 0;
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: flex;
  font-size: 20px;
  height: 28px;
  justify-content: center;
  padding: 0;
  transition: color 0.15s ease;
  width: 28px;

  &:hover {
    color: var(--lumina-text);
  }
}

.sources-body {
  display: flex;
  flex-direction: column;
  gap: 18px;
  max-height: 68vh;
  overflow-y: auto;
  padding: 18px 20px;
}

.section-title-row {
  align-items: center;
  display: flex;
  justify-content: space-between;
  margin-bottom: 10px;
}

.section-label {
  align-items: center;
  color: var(--lumina-text);
  display: inline-flex;
  font-size: 13px;
  font-weight: 600;
  gap: 6px;
}

.section-hint {
  color: var(--lumina-text-tertiary);
  font-size: 11px;
}

.registry-config-section {
  background: var(--lumina-surface-2, rgba(0, 0, 0, 0.02));
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-md, 8px);
  padding: 14px 16px;
}

.url-input-wrap {
  display: flex;
  gap: 8px;
}

.macos-input {
  background: var(--lumina-surface-1, #ffffff);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-sm, 6px);
  color: var(--lumina-text);
  flex: 1;
  font-family: var(--lumina-font-mono, monospace);
  font-size: 12px;
  height: 32px;
  padding: 0 10px;
  transition: all 0.15s ease;

  &:focus {
    border-color: var(--lumina-primary);
    box-shadow: 0 0 0 2px var(--lumina-primary-soft);
    outline: none;
  }
}

.test-conn-btn {
  align-items: center;
  background: var(--lumina-surface-3, var(--lumina-surface-2));
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-sm, 6px);
  color: var(--lumina-text);
  cursor: pointer;
  display: inline-flex;
  flex-shrink: 0;
  font-size: 12px;
  gap: 5px;
  height: 32px;
  padding: 0 12px;
  transition: all 0.15s ease;

  &:hover:not(:disabled) {
    background: var(--lumina-control-hover);
    border-color: var(--lumina-separator-strong);
  }

  &:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }
}

.presets-row {
  align-items: center;
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 10px;
}

.preset-label {
  color: var(--lumina-text-secondary);
  font-size: 11px;
}

.preset-chip {
  background: var(--lumina-surface-1, #ffffff);
  border: 0.5px solid var(--lumina-separator);
  border-radius: 4px;
  color: var(--lumina-text-secondary);
  cursor: pointer;
  font-size: 11px;
  padding: 3px 8px;
  transition: all 0.15s ease;

  &:hover {
    border-color: var(--lumina-primary);
    color: var(--lumina-primary);
  }

  &.active {
    background: var(--lumina-primary-soft);
    border-color: var(--lumina-primary);
    color: var(--lumina-primary);
    font-weight: 550;
  }

  &--reset {
    color: var(--lumina-text-tertiary);
    margin-left: auto;
  }
}

.test-feedback-pill {
  align-items: center;
  border-radius: var(--lumina-radius-sm, 6px);
  display: flex;
  font-size: 11px;
  gap: 6px;
  margin-top: 10px;
  padding: 6px 10px;

  &.success {
    background: rgba(16, 185, 129, 0.1);
    border: 0.5px solid rgba(16, 185, 129, 0.3);
    color: #10b981;
  }

  &.error {
    background: rgba(239, 68, 68, 0.1);
    border: 0.5px solid rgba(239, 68, 68, 0.3);
    color: #ef4444;
  }
}

.architecture-banner {
  background: linear-gradient(
    135deg,
    color-mix(in srgb, var(--lumina-primary) 8%, var(--lumina-surface-2)),
    var(--lumina-surface-2)
  );
  border: 0.5px solid color-mix(in srgb, var(--lumina-primary) 20%, var(--lumina-separator));
  border-radius: var(--lumina-radius-md, 8px);
  padding: 14px 16px;
}

.banner-badge {
  align-items: center;
  color: var(--lumina-primary);
  display: inline-flex;
  font-size: 11px;
  font-weight: 600;
  gap: 5px;
  margin-bottom: 6px;
}

.banner-title {
  font-size: 13px;
  font-weight: 650;
  margin: 0 0 6px;
}

.banner-desc {
  color: var(--lumina-text-secondary);
  font-size: 11.5px;
  line-height: 1.5;
  margin: 0 0 8px;
}

.banner-points {
  color: var(--lumina-text-secondary);
  display: flex;
  flex-direction: column;
  font-size: 11.5px;
  gap: 6px;
  line-height: 1.5;
  margin: 0;
  padding-left: 18px;

  li strong {
    color: var(--lumina-text);
  }
}

.token-settings-section {
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-md, 8px);
  overflow: hidden;
}

.token-header-row {
  align-items: center;
  background: var(--lumina-surface-2, rgba(0, 0, 0, 0.02));
  cursor: pointer;
  display: flex;
  justify-content: space-between;
  padding: 10px 14px;
  user-select: none;
}

.token-title-group {
  align-items: center;
  display: flex;
  gap: 8px;
}

.key-icon {
  color: var(--lumina-primary);
}

.token-title {
  font-size: 12px;
  font-weight: 600;
}

.token-badge {
  background: var(--lumina-control-bg);
  border: 0.5px solid var(--lumina-separator);
  border-radius: 4px;
  color: var(--lumina-text-secondary);
  font-size: 10px;
  padding: 1px 6px;
}

.collapse-icon {
  color: var(--lumina-text-tertiary);
  font-size: 14px;
}

.token-body {
  border-top: 0.5px solid var(--lumina-separator);
  padding: 12px 14px;
}

.token-desc {
  color: var(--lumina-text-secondary);
  font-size: 11.5px;
  line-height: 1.5;
  margin: 0 0 10px;
}

.token-input-row {
  display: flex;
  gap: 8px;
}

.token-input {
  font-family: var(--lumina-font-mono, monospace);
  letter-spacing: 0.5px;
}

.toggle-eye-btn {
  align-items: center;
  background: var(--lumina-surface-1, #ffffff);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-sm, 6px);
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: flex;
  font-size: 15px;
  height: 32px;
  justify-content: center;
  width: 32px;

  &:hover {
    color: var(--lumina-text);
  }
}

.save-token-btn {
  background: var(--lumina-primary);
  border: 0;
  border-radius: var(--lumina-radius-sm, 6px);
  color: var(--lumina-on-accent, #ffffff);
  cursor: pointer;
  font-size: 12px;
  font-weight: 550;
  height: 32px;
  padding: 0 14px;
  transition: opacity 0.15s ease;

  &:hover {
    opacity: 0.9;
  }
}

.sources-footer {
  align-items: center;
  border-top: 0.5px solid var(--lumina-separator);
  display: flex;
  justify-content: space-between;
  padding: 12px 20px;
}

.sync-summary-text {
  color: var(--lumina-text-secondary);
  font-size: 11px;
}

.footer-right {
  display: flex;
  gap: 10px;
}

.footer-btn {
  align-items: center;
  border: 0;
  border-radius: var(--lumina-radius-sm, 6px);
  cursor: pointer;
  display: inline-flex;
  font-size: 12px;
  font-weight: 550;
  gap: 6px;
  height: 32px;
  padding: 0 14px;
  transition: all 0.15s ease;

  &--secondary {
    background: var(--lumina-surface-2);
    border: 0.5px solid var(--lumina-separator);
    color: var(--lumina-text);

    &:hover:not(:disabled) {
      background: var(--lumina-control-hover);
    }
  }

  &--primary {
    background: var(--lumina-primary);
    color: var(--lumina-on-accent, #ffffff);

    &:hover:not(:disabled) {
      opacity: 0.9;
    }
  }

  &:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }
}

.spin-glyph {
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}
</style>
