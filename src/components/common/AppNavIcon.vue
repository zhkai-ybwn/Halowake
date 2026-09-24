<template>
  <span class="app-nav-icon" :style="{ width: `${size}px`, height: `${size}px` }">
    <!-- 1. Git Assistant: Red-Orange Git Diamond with White Branch Graph -->
    <svg v-if="name === 'git-assistant'" viewBox="0 0 24 24" class="nav-svg" aria-hidden="true">
      <defs>
        <linearGradient id="git-grad" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stop-color="#F45A38" />
          <stop offset="100%" stop-color="#DE3F24" />
        </linearGradient>
      </defs>
      <rect x="3.4" y="3.4" width="15" height="15" rx="3.2" transform="rotate(45 12 12)" fill="url(#git-grad)" />
      <!-- Git branch graph -->
      <line x1="9.8" y1="7.6" x2="9.8" y2="16.4" stroke="#FFFFFF" stroke-width="1.8" stroke-linecap="round" />
      <path d="M9.8 13.5 C10.2 11.5 12.2 10.2 15 10.2" stroke="#FFFFFF" stroke-width="1.8" stroke-linecap="round" fill="none" />
      <circle cx="9.8" cy="7.6" r="1.6" fill="#FFFFFF" />
      <circle cx="9.8" cy="16.4" r="1.6" fill="#FFFFFF" />
      <circle cx="15" cy="10.2" r="1.6" fill="#FFFFFF" />
    </svg>

    <!-- 2. DevDock: Emerald/Mint-Green Project Container & Terminal Dock -->
    <svg v-else-if="name === 'devdock'" viewBox="0 0 24 24" class="nav-svg" aria-hidden="true">
      <defs>
        <linearGradient id="devdock-grad" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stop-color="#10B981" />
          <stop offset="100%" stop-color="#059669" />
        </linearGradient>
      </defs>
      <rect x="2" y="2" width="20" height="20" rx="4.8" fill="url(#devdock-grad)" />
      <!-- Project Terminal Window & Dock Prompt -->
      <rect x="5.5" y="6" width="13" height="12" rx="2.2" fill="#FFFFFF" fill-opacity="0.18" stroke="#FFFFFF" stroke-width="1.3" />
      <circle cx="8" cy="8.5" r="0.8" fill="#FFFFFF" />
      <circle cx="10.5" cy="8.5" r="0.8" fill="#FFFFFF" />
      <path d="M8.5 12.5 L11 14.5 L8.5 16.5" stroke="#FFFFFF" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" fill="none" />
      <line x1="12.8" y1="16.5" x2="15.5" y2="16.5" stroke="#FFFFFF" stroke-width="1.5" stroke-linecap="round" />
    </svg>

    <!-- 3. Markdown Preview: Amber/Golden-Yellow Rounded Badge with Bold White "M" -->
    <svg v-else-if="name === 'markdown-preview'" viewBox="0 0 24 24" class="nav-svg" aria-hidden="true">
      <defs>
        <linearGradient id="md-grad" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stop-color="#FBBF24" />
          <stop offset="100%" stop-color="#F59E0B" />
        </linearGradient>
      </defs>
      <rect x="2" y="2" width="20" height="20" rx="4.8" fill="url(#md-grad)" />
      <!-- Geometric White "M" -->
      <path d="M7 16.5 V7.5 L12 13.2 L17 7.5 V16.5" stroke="#FFFFFF" stroke-width="2.3" stroke-linecap="round" stroke-linejoin="round" fill="none" />
    </svg>

    <!-- 4. Codex Report: Warm Orange Calendar / Daily Report Notebook -->
    <svg v-else-if="name === 'codex-report'" viewBox="0 0 24 24" class="nav-svg" aria-hidden="true">
      <defs>
        <linearGradient id="codex-grad" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stop-color="#FB923C" />
          <stop offset="100%" stop-color="#EA580C" />
        </linearGradient>
      </defs>
      <rect x="2" y="2" width="20" height="20" rx="4.8" fill="url(#codex-grad)" />
      <!-- Calendar Notepad with Checkmark -->
      <rect x="5.5" y="6" width="13" height="12.5" rx="2.2" stroke="#FFFFFF" stroke-width="1.4" fill="#FFFFFF" fill-opacity="0.16" />
      <line x1="8.5" y1="4.5" x2="8.5" y2="7" stroke="#FFFFFF" stroke-width="1.4" stroke-linecap="round" />
      <line x1="15.5" y1="4.5" x2="15.5" y2="7" stroke="#FFFFFF" stroke-width="1.4" stroke-linecap="round" />
      <line x1="8.5" y1="10" x2="15.5" y2="10" stroke="#FFFFFF" stroke-width="1.2" stroke-linecap="round" />
      <path d="M8.5 13.8 L10.5 15.6 L15.5 11.8" stroke="#FFFFFF" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" fill="none" />
    </svg>

    <!-- 5. AI Quota: Violet / Neon-Cyan AI Token Sparkle Badge -->
    <svg v-else-if="name === 'ai-quota'" viewBox="0 0 24 24" class="nav-svg" aria-hidden="true">
      <defs>
        <linearGradient id="quota-grad" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stop-color="#8B5CF6" />
          <stop offset="100%" stop-color="#06B6D4" />
        </linearGradient>
      </defs>
      <rect x="2" y="2" width="20" height="20" rx="4.8" fill="url(#quota-grad)" />
      <!-- AI Sparkle Star & Intelligence Node -->
      <path d="M12 5.5 C12 8.2 13.5 9.8 16.5 9.8 C13.5 9.8 12 11.4 12 14.1 C12 11.4 10.5 9.8 7.5 9.8 C10.5 9.8 12 8.2 12 5.5 Z" fill="#FFFFFF" />
      <path d="M16.5 14.2 C16.5 15.6 17.2 16.4 18.6 16.4 C17.2 16.4 16.5 17.2 16.5 18.6 C16.5 17.2 15.7 16.4 14.4 16.4 C15.7 16.4 16.5 15.6 16.5 14.2 Z" fill="#FFFFFF" opacity="0.9" />
      <circle cx="7.5" cy="16.5" r="1.1" fill="#FFFFFF" opacity="0.85" />
    </svg>

    <!-- 6. Settings: Subtle Slate Blue-Gray Badge -->
    <svg v-else-if="name === 'settings'" viewBox="0 0 24 24" class="nav-svg" aria-hidden="true">
      <defs>
        <linearGradient id="settings-grad" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stop-color="#64748B" />
          <stop offset="100%" stop-color="#475569" />
        </linearGradient>
      </defs>
      <rect x="2" y="2" width="20" height="20" rx="4.8" fill="url(#settings-grad)" />
      <path
        d="M12 14.5a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5zm6.5-2.5a6.5 6.5 0 0 0-.08-.94l1.37-1.07a.33.33 0 0 0 .08-.42l-1.3-2.25a.33.33 0 0 0-.4-.15l-1.62.65a6.45 6.45 0 0 0-1.62-.94l-.25-1.72a.33.33 0 0 0-.33-.29h-2.6a.33.33 0 0 0-.33.29l-.25 1.72a6.45 6.45 0 0 0-1.62.94l-1.62-.65a.33.33 0 0 0-.4.15l-1.3 2.25a.33.33 0 0 0 .08.42l1.37 1.07A6.5 6.5 0 0 0 5.5 12c0 .32.03.63.08.94l-1.37 1.07a.33.33 0 0 0-.08.42l1.3 2.25c.08.14.25.2.4.15l1.62-.65c.49.38 1.04.7 1.62.94l.25 1.72c.03.17.17.29.33.29h2.6c.17 0 .3-.12.33-.29l.25-1.72c.58-.24 1.13-.56 1.62-.94l1.62.65c.15.05.32 0 .4-.15l1.3-2.25a.33.33 0 0 0-.08-.42l-1.37-1.07c.05-.31.08-.62.08-.94z"
        fill="#FFFFFF"
      />
    </svg>

    <!-- Fallback to Iconify component -->
    <Icon v-else :icon="fallbackIcon || 'solar:widget-2-linear'" />
  </span>
</template>

<script setup lang="ts">
import { Icon } from '@iconify/vue'

withDefaults(
  defineProps<{
    name: string
    size?: number | string
    fallbackIcon?: string
  }>(),
  {
    size: 20,
    fallbackIcon: '',
  }
)
</script>

<style scoped lang="scss">
.app-nav-icon {
  align-items: center;
  display: inline-flex;
  flex-shrink: 0;
  justify-content: center;
  line-height: 1;
  position: relative;
  transition: transform 0.16s ease, filter 0.16s ease;
  user-select: none;
}

.nav-svg {
  display: block;
  filter: drop-shadow(0 1px 2px rgba(0, 0, 0, 0.15));
  height: 100%;
  width: 100%;
}

.app-nav-icon:hover {
  filter: drop-shadow(0 2px 4px rgba(0, 0, 0, 0.24));
  transform: translateY(-0.5px);
}
</style>
