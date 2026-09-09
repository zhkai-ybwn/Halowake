<template>
  <span
    class="provider-brand-logo-container"
    :class="[`provider-${normalizedProvider}`, { 'custom-size': size }]"
    :style="containerStyle"
  >
    <!-- 1. OpenAI / Codex -->
    <svg
      v-if="normalizedProvider === 'openai' || normalizedProvider === 'codex'"
      viewBox="0 0 24 24"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      class="brand-svg openai-svg"
      aria-label="OpenAI"
    >
      <path
        d="M22.2819 9.8211a5.9847 5.9847 0 0 0-.5157-4.9108 6.0462 6.0462 0 0 0-6.5098-2.9A6.0651 6.0651 0 0 0 4.9807 4.1818a5.9847 5.9847 0 0 0-3.9977 2.9 6.0462 6.0462 0 0 0 .7427 7.0966 5.98 5.98 0 0 0 .511 4.9107 6.051 6.051 0 0 0 6.5146 2.9001A5.9847 5.9847 0 0 0 13.259 24a6.0557 6.0557 0 0 0 5.7718-4.2058 5.9894 5.9894 0 0 0 3.9977-2.9001 6.0557 6.0557 0 0 0-.7466-7.0729zm-9.022 12.6081a4.4755 4.4755 0 0 1-2.8764-1.0408l.1419-.0804 4.7783-2.7582a.7948.7948 0 0 0 .3927-.6813v-6.7369l2.02 1.1683a.071.071 0 0 1 .038.052v5.5826a4.504 4.504 0 0 1-4.4945 4.4947zm-9.66-4.1254a4.4708 4.4708 0 0 1-.5346-3.0137l.142.0852 4.783 2.7582a.7712.7712 0 0 0 .7806 0l5.8428-3.3685v2.3324a.0804.0804 0 0 1-.0332.0615L9.74 19.9502a4.4992 4.4992 0 0 1-6.1402-1.6464zM2.3408 7.8956a4.485 4.485 0 0 1 2.3655-1.9728V11.6a.7664.7664 0 0 0 .3879.6765l5.8144 3.3543-2.0201 1.1683a.0757.0757 0 0 1-.071 0l-4.8303-2.7866A4.504 4.504 0 0 1 2.3408 7.8956zm16.0993 3.8558L12.5973 8.3829l2.02-1.1636a.0757.0757 0 0 1 .071 0l4.8303 2.7913a4.4944 4.4944 0 0 1-.6765 8.1042v-5.6772a.79.79 0 0 0-.4022-.6862zm2.0107-3.0231l-.142-.0852-4.7735-2.7818a.7759.7759 0 0 0-.7854 0L8.907 9.2297V6.8974a.0662.0662 0 0 1 .0331-.0615l4.835-2.7913a4.504 4.504 0 0 1 6.6743 4.6572zm-12.641 4.1443l-2.02-1.1635a.0804.0804 0 0 1-.038-.0568V6.0748a4.4992 4.4992 0 0 1 7.3757-3.4537l-.142.0805L6.184 5.4598a.7948.7948 0 0 0-.3928.6813v6.7369zm1.0976-1.708l2.607-1.5047 2.607 1.5047v3.0094l-2.607 1.5047-2.607-1.5047z"
        fill="currentColor"
      />
    </svg>

    <!-- 2. DeepSeek (Official leaping whale) -->
    <svg
      v-else-if="normalizedProvider === 'deepseek'"
      viewBox="0 0 24 24"
      class="brand-svg deepseek-svg"
      aria-label="DeepSeek"
    >
      <path
        d="M23.748 4.482c-.254-.124-.364.113-.512.234-.051.039-.094.09-.137.136-.372.397-.806.657-1.373.626-.829-.046-1.537.214-2.163.848-.133-.782-.575-1.248-1.247-1.548-.352-.156-.708-.311-.955-.65-.172-.241-.219-.51-.305-.774-.055-.16-.11-.323-.293-.35-.2-.031-.278.136-.356.276-.313.572-.434 1.202-.422 1.84.027 1.436.633 2.58 1.838 3.393.137.093.172.187.129.323-.082.28-.18.552-.266.833-.055.179-.137.217-.329.14a5.526 5.526 0 01-1.736-1.18c-.857-.828-1.631-1.742-2.597-2.458a11.365 11.365 0 00-.689-.471c-.985-.957.13-1.743.388-1.836.27-.098.093-.432-.779-.428-.872.004-1.67.295-2.687.684a3.055 3.055 0 01-.465.137 9.597 9.597 0 00-2.883-.102c-1.885.21-3.39 1.102-4.497 2.623C.082 8.606-.231 10.684.152 12.85c.403 2.284 1.569 4.175 3.36 5.653 1.858 1.533 3.997 2.284 6.438 2.14 1.482-.085 3.133-.284 4.994-1.86.47.234.962.327 1.78.397.63.059 1.236-.03 1.705-.128.735-.156.684-.837.419-.961-2.155-1.004-1.682-.595-2.113-.926 1.096-1.296 2.746-2.642 3.392-7.003.05-.347.007-.565 0-.845-.004-.17.035-.237.23-.256a4.173 4.173 0 001.545-.475c1.396-.763 1.96-2.015 2.093-3.517.02-.23-.004-.467-.247-.588zM11.581 18c-2.089-1.642-3.102-2.183-3.52-2.16-.392.024-.321.471-.235.763.09.288.207.486.371.739.114.167.192.416-.113.603-.673.416-1.842-.14-1.897-.167-1.361-.802-2.5-1.86-3.301-3.307-.774-1.393-1.224-2.887-1.298-4.482-.02-.386.093-.522.477-.592a4.696 4.696 0 011.529-.039c2.132.312 3.946 1.265 5.468 2.774.868.86 1.525 1.887 2.202 2.891.72 1.066 1.494 2.082 2.48 2.914.348.292.625.514.891.677-.802.09-2.14.11-3.054-.614zm1-6.44a.306.306 0 01.415-.287.302.302 0 01.2.288.306.306 0 01-.31.307.303.303 0 01-.304-.308zm3.11 1.596c-.2.081-.399.151-.59.16a1.245 1.245 0 01-.798-.254c-.274-.23-.47-.358-.552-.758a1.73 1.73 0 01.016-.588c.07-.327-.008-.537-.239-.727-.187-.156-.426-.199-.688-.199a.559.559 0 01-.254-.078c-.11-.054-.2-.19-.114-.358.028-.054.16-.186.192-.21.356-.202.767-.136 1.146.016.352.144.618.408 1.001.782.391.451.462.576.685.914.176.265.336.537.445.848.067.195-.019.354-.25.452z"
        fill="#4D6BFE"
      />
    </svg>

    <!-- 3. Google Gemini (Official 4-color gradient star) -->
    <svg
      v-else-if="normalizedProvider === 'gemini' || normalizedProvider === 'google'"
      viewBox="0 0 24 24"
      class="brand-svg gemini-svg"
      aria-label="Google Gemini"
    >
      <defs>
        <linearGradient :id="`gem-g0-${uid}`" gradientUnits="userSpaceOnUse" x1="7" x2="11" y1="15.5" y2="12">
          <stop stop-color="#08B962" />
          <stop offset="1" stop-color="#08B962" stop-opacity="0" />
        </linearGradient>
        <linearGradient :id="`gem-g1-${uid}`" gradientUnits="userSpaceOnUse" x1="8" x2="11.5" y1="5.5" y2="11">
          <stop stop-color="#F94543" />
          <stop offset="1" stop-color="#F94543" stop-opacity="0" />
        </linearGradient>
        <linearGradient :id="`gem-g2-${uid}`" gradientUnits="userSpaceOnUse" x1="3.5" x2="17.5" y1="13.5" y2="12">
          <stop stop-color="#FABC12" />
          <stop offset=".46" stop-color="#FABC12" stop-opacity="0" />
        </linearGradient>
      </defs>
      <path
        d="M20.616 10.835a14.147 14.147 0 01-4.45-3.001 14.111 14.111 0 01-3.678-6.452.503.503 0 00-.975 0 14.134 14.134 0 01-3.679 6.452 14.155 14.155 0 01-4.45 3.001c-.65.28-1.318.505-2.002.678a.502.502 0 000 .975c.684.172 1.35.397 2.002.677a14.147 14.147 0 014.45 3.001 14.112 14.112 0 013.679 6.453.502.502 0 00.975 0c.172-.685.397-1.351.677-2.003a14.145 14.145 0 013.001-4.45 14.113 14.113 0 016.453-3.678.503.503 0 000-.975 13.245 13.245 0 01-2.003-.678z"
        fill="#3186FF"
      />
      <path
        d="M20.616 10.835a14.147 14.147 0 01-4.45-3.001 14.111 14.111 0 01-3.678-6.452.503.503 0 00-.975 0 14.134 14.134 0 01-3.679 6.452 14.155 14.155 0 01-4.45 3.001c-.65.28-1.318.505-2.002.678a.502.502 0 000 .975c.684.172 1.35.397 2.002.677a14.147 14.147 0 014.45 3.001 14.112 14.112 0 013.679 6.453.502.502 0 00.975 0c.172-.685.397-1.351.677-2.003a14.145 14.145 0 013.001-4.45 14.113 14.113 0 016.453-3.678.503.503 0 000-.975 13.245 13.245 0 01-2.003-.678z"
        :fill="`url(#gem-g0-${uid})`"
      />
      <path
        d="M20.616 10.835a14.147 14.147 0 01-4.45-3.001 14.111 14.111 0 01-3.678-6.452.503.503 0 00-.975 0 14.134 14.134 0 01-3.679 6.452 14.155 14.155 0 01-4.45 3.001c-.65.28-1.318.505-2.002.678a.502.502 0 000 .975c.684.172 1.35.397 2.002.677a14.147 14.147 0 014.45 3.001 14.112 14.112 0 013.679 6.453.502.502 0 00.975 0c.172-.685.397-1.351.677-2.003a14.145 14.145 0 013.001-4.45 14.113 14.113 0 016.453-3.678.503.503 0 000-.975 13.245 13.245 0 01-2.003-.678z"
        :fill="`url(#gem-g1-${uid})`"
      />
      <path
        d="M20.616 10.835a14.147 14.147 0 01-4.45-3.001 14.111 14.111 0 01-3.678-6.452.503.503 0 00-.975 0 14.134 14.134 0 01-3.679 6.452 14.155 14.155 0 01-4.45 3.001c-.65.28-1.318.505-2.002.678a.502.502 0 000 .975c.684.172 1.35.397 2.002.677a14.147 14.147 0 014.45 3.001 14.112 14.112 0 013.679 6.453.502.502 0 00.975 0c.172-.685.397-1.351.677-2.003a14.145 14.145 0 013.001-4.45 14.113 14.113 0 016.453-3.678.503.503 0 000-.975 13.245 13.245 0 01-2.003-.678z"
        :fill="`url(#gem-g2-${uid})`"
      />
    </svg>

    <!-- 4. OpenRouter -->
    <svg
      v-else-if="normalizedProvider === 'openrouter'"
      viewBox="0 0 24 24"
      fill="currentColor"
      fill-rule="evenodd"
      class="brand-svg openrouter-svg"
      aria-label="OpenRouter"
    >
      <path
        d="M18.654 3.87a5.087 5.087 0 110 10.174L23.7 19.09c.64.641.187 1.737-.72 1.737H8.48a8.479 8.479 0 010-16.958h10.175zM8.479 7.26a5.087 5.087 0 100 10.176 5.087 5.087 0 000-10.175z"
      />
    </svg>

    <!-- 5. Claude / Anthropic -->
    <svg
      v-else-if="normalizedProvider === 'claude' || normalizedProvider === 'anthropic'"
      viewBox="0 0 24 24"
      class="brand-svg claude-svg"
      aria-label="Claude"
    >
      <path
        d="M4.709 15.955l4.72-2.647.08-.23-.08-.128H9.2l-.79-.048-2.698-.073-2.339-.097-2.266-.122-.571-.121L0 11.784l.055-.352.48-.321.686.06 1.52.103 2.278.158 1.652.097 2.449.255h.389l.055-.157-.134-.098-.103-.097-2.358-1.596-2.552-1.688-1.336-.972-.724-.491-.364-.462-.158-1.008.656-.722.881.06.225.061.893.686 1.908 1.476 2.491 1.833.365.304.145-.103.019-.073-.164-.274-1.355-2.446-1.446-2.49-.644-1.032-.17-.619a2.97 2.97 0 01-.104-.729L6.283.134 6.696 0l.996.134.42.364.62 1.414 1.002 2.229 1.555 3.03.456.898.243.832.091.255h.158V9.01l.128-1.706.237-2.095.23-2.695.08-.76.376-.91.747-.492.584.28.48.685-.067.444-.286 1.851-.559 2.903-.364 1.942h.212l.243-.242.985-1.306 1.652-2.064.73-.82.85-.904.547-.431h1.033l.76 1.129-.34 1.166-1.064 1.347-.881 1.142-1.264 1.7-.79 1.36.073.11.188-.02 2.856-.606 1.543-.28 1.841-.315.833.388.091.395-.328.807-1.969.486-2.309.462-3.439.813-.042.03.049.061 1.549.146.662.036h1.622l3.02.225.79.522.474.638-.079.485-1.215.62-1.64-.389-3.829-.91-1.312-.329h-.182v.11l1.093 1.068 2.006 1.81 2.509 2.33.127.578-.322.455-.34-.049-2.205-1.657-.851-.747-1.926-1.62h-.128v.17l.444.649 2.345 3.521.122 1.08-.17.353-.608.213-.668-.122-1.374-1.925-1.415-2.167-1.143-1.943-.14.08-.674 7.254-.316.37-.729.28-.607-.461-.322-.747.322-1.476.389-1.924.315-1.53.286-1.9.17-.632-.012-.042-.14.018-1.434 1.967-2.18 2.945-1.726 1.845-.414.164-.717-.37.067-.662.401-.589 2.388-3.036 1.44-1.882.93-1.086-.006-.158h-.055L4.132 18.56l-1.13.146-.487-.456.061-.746.231-.243 1.908-1.312-.006.006z"
        fill="#D97757"
      />
    </svg>

    <!-- 6. OpenCode -->
    <svg
      v-else-if="normalizedProvider === 'opencode'"
      viewBox="0 0 24 24"
      class="brand-svg opencode-svg"
      aria-label="OpenCode"
    >
      <rect x="2" y="3" width="20" height="18" rx="4" fill="#0EA5E9" fill-opacity="0.12" stroke="#0EA5E9" stroke-width="1.8" />
      <path d="M7 8.5L10.5 12L7 15.5" stroke="#0EA5E9" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
      <line x1="12.5" y1="15.5" x2="16.5" y2="15.5" stroke="#0EA5E9" stroke-width="2" stroke-linecap="round" />
    </svg>

    <!-- 7. WorkBuddy -->
    <svg
      v-else-if="normalizedProvider === 'workbuddy'"
      viewBox="0 0 24 24"
      class="brand-svg workbuddy-svg"
      aria-label="WorkBuddy"
    >
      <path
        d="M12 2C6.48 2 2 6.04 2 11.02c0 2.87 1.48 5.43 3.79 7.04l-.84 3.09c-.14.51.35.95.82.74l3.69-1.63c.81.25 1.66.38 2.54.38 5.52 0 10-4.04 10-9.02S17.52 2 12 2z"
        fill="#0066FF"
      />
      <circle cx="8.5" cy="10.5" r="1.5" fill="#FFFFFF" />
      <circle cx="15.5" cy="10.5" r="1.5" fill="#FFFFFF" />
      <path d="M8.5 14c1.1 1.2 2.4 1.8 3.5 1.8s2.4-.6 3.5-1.8" stroke="#FFFFFF" stroke-width="1.6" stroke-linecap="round" />
    </svg>

    <!-- 8. SiliconFlow (硅基流动) -->
    <svg
      v-else-if="normalizedProvider === 'siliconflow'"
      viewBox="0 0 24 24"
      class="brand-svg siliconflow-svg"
      aria-label="SiliconFlow"
    >
      <path
        d="M12 2L3.5 6.9v9.8L12 22l8.5-5.3V6.9L12 2zm0 3.6l5.3 3.3v5.8L12 18.2l-5.3-3.3V8.9L12 5.6z"
        fill="#7C3AED"
      />
      <circle cx="12" cy="12" r="2.8" fill="#A78BFA" />
      <path d="M12 5.6v6.4l4.5 2.6" stroke="#FFFFFF" stroke-width="1.2" stroke-linecap="round" />
    </svg>

    <!-- 9. Moonshot AI (月之暗面 / Kimi) -->
    <svg
      v-else-if="normalizedProvider === 'moonshot' || normalizedProvider === 'kimi'"
      viewBox="0 0 24 24"
      class="brand-svg moonshot-svg"
      aria-label="Moonshot"
    >
      <path
        d="M12.3 2a10 10 0 1 0 9.7 12.3 8 8 0 0 1-9.7-12.3z"
        fill="#6366F1"
      />
      <polygon points="16,3 17.2,5.8 20,7 17.2,8.2 16,11 14.8,8.2 12,7 14.8,5.8" fill="#818CF8" />
    </svg>

    <!-- 10. Zhipu AI (智谱 GLM) -->
    <svg
      v-else-if="normalizedProvider === 'zhipu' || normalizedProvider === 'glm'"
      viewBox="0 0 24 24"
      class="brand-svg zhipu-svg"
      aria-label="Zhipu AI"
    >
      <rect x="2" y="2" width="20" height="20" rx="5" fill="#3062F9" />
      <path
        d="M6 7.5h12l-7.5 9h7.5"
        fill="none"
        stroke="#FFFFFF"
        stroke-width="2.5"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
      <circle cx="16.5" cy="7.5" r="1.5" fill="#60A5FA" />
    </svg>

    <!-- 11. Qwen (通义千问) -->
    <svg
      v-else-if="normalizedProvider === 'qwen' || normalizedProvider === 'tongyi'"
      viewBox="0 0 24 24"
      class="brand-svg qwen-svg"
      aria-label="Qwen"
    >
      <circle cx="12" cy="12" r="10" fill="#615CED" />
      <path
        d="M12 5.5a6.5 6.5 0 0 0-4.6 11.1L6 18.5l2.4-.6A6.5 6.5 0 1 0 12 5.5zm0 10a3.5 3.5 0 1 1 3.5-3.5 3.5 3.5 0 0 1-3.5 3.5z"
        fill="#FFFFFF"
      />
      <path d="M14.5 14.5L17.5 17.5" stroke="#FFFFFF" stroke-width="2.2" stroke-linecap="round" />
    </svg>

    <!-- 12. MiniMax -->
    <svg
      v-else-if="normalizedProvider === 'minimax'"
      viewBox="0 0 24 24"
      class="brand-svg minimax-svg"
      aria-label="MiniMax"
    >
      <rect x="2" y="2" width="20" height="20" rx="5" fill="#FF4B4B" />
      <path
        d="M6 16V8l3 4 3-4v8M15 16V8l3 4 3-4v8"
        stroke="#FFFFFF"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
        fill="none"
      />
    </svg>

    <!-- Fallback AI Core -->
    <svg
      v-else
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="1.8"
      stroke-linecap="round"
      stroke-linejoin="round"
      class="brand-svg fallback-svg"
      aria-label="AI Provider"
    >
      <rect x="4" y="4" width="16" height="16" rx="3" />
      <rect x="9" y="9" width="6" height="6" rx="1" />
      <path d="M9 1v3M15 1v3M9 20v3M15 20v3M20 9h3M20 15h3M1 9h3M1 15h3" />
    </svg>
  </span>
</template>

<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(
  defineProps<{
    provider?: string
    size?: number | string
  }>(),
  {
    provider: '',
    size: undefined,
  }
)

let globalCounter = 0
const uid = computed(() => {
  globalCounter = (globalCounter + 1) % 100000
  return `${Math.random().toString(36).substring(2, 7)}_${globalCounter}`
})

const normalizedProvider = computed(() => {
  const p = (props.provider || '').trim().toLowerCase()
  if (p.includes('codex') || p.includes('openai') || p.includes('chatgpt') || p.includes('gpt')) {
    return 'openai'
  }
  if (p.includes('deepseek')) {
    return 'deepseek'
  }
  if (p.includes('gemini') || p.includes('google') || p.includes('antigravity')) {
    return 'gemini'
  }
  if (p.includes('openrouter')) {
    return 'openrouter'
  }
  if (p.includes('claude') || p.includes('anthropic')) {
    return 'claude'
  }
  if (p.includes('opencode')) {
    return 'opencode'
  }
  if (p.includes('workbuddy') || p.includes('buddy')) {
    return 'workbuddy'
  }
  if (p.includes('siliconflow') || p.includes('silicon')) {
    return 'siliconflow'
  }
  if (p.includes('moonshot') || p.includes('kimi')) {
    return 'moonshot'
  }
  if (p.includes('zhipu') || p.includes('glm') || p.includes('bigmodel')) {
    return 'zhipu'
  }
  if (p.includes('qwen') || p.includes('tongyi') || p.includes('bailian')) {
    return 'qwen'
  }
  if (p.includes('minimax')) {
    return 'minimax'
  }
  return 'default'
})

const containerStyle = computed(() => {
  if (!props.size) return {}
  const dim = typeof props.size === 'number' ? `${props.size}px` : props.size
  return {
    width: dim,
    height: dim,
  }
})
</script>

<style scoped lang="scss">
.provider-brand-logo-container {
  align-items: center;
  display: inline-flex;
  height: 1em;
  justify-content: center;
  line-height: 1;
  position: relative;
  vertical-align: -0.15em;
  width: 1em;

  &.custom-size {
    vertical-align: middle;
  }
}

.brand-svg {
  display: block;
  flex-shrink: 0;
  height: 100%;
  width: 100%;
}

.openai-svg {
  color: #10a37f;
}

.openrouter-svg {
  color: #6466e9;
}

.deepseek-svg {
  /* Whale path has fill="#4D6BFE" embedded */
}

.gemini-svg {
  /* Google Gemini 4-color gradient embedded */
}

.claude-svg {
  /* Claude #D97757 embedded */
}

.fallback-svg {
  color: var(--lumina-text-secondary);
}
</style>
