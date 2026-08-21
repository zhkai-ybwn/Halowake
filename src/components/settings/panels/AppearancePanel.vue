<template>
  <div class="panel">

    <!-- 主题切换 -->
    <section class="panel-section">
      <h2>Theme</h2>
      <n-radio-group v-model:value="theme" @update:value="applyTheme">
        <n-radio value="light">Light</n-radio>
        <n-radio value="dark">Dark</n-radio>
      </n-radio-group>
    </section>

    <!-- 卡片设置 -->
    <section class="panel-section">
      <h2>Card Style</h2>
      <n-select
        v-model:value="cardSize"
        :options="cardSizeOptions"
        @update:value="applyCardSize"
      />

      <n-switch v-model:value="useShadow" @update:value="applyShadow">
        Shadow
      </n-switch>
    </section>

    <!-- 背景设置 -->
    <section class="panel-section">
      <h2>Background</h2>

      <n-radio-group v-model:value="bgMode" @update:value="applyBackground">
        <n-radio value="none">Default</n-radio>
        <n-radio value="soft">Soft Gray</n-radio>
        <n-radio value="custom">Custom</n-radio>
      </n-radio-group>

      <div v-if="bgMode === 'custom'" class="bg-input">
        <n-input
          v-model:value="customBg"
          placeholder="Enter image URL…"
          @blur="applyBackground"
        />
      </div>
    </section>

  </div>
</template>

<script setup lang="ts">
import { ref } from "vue"
import { useTheme } from "@/hooks/useTheme"

/* ---------------------
  主题切换
---------------------- */
const { theme, applyTheme } = useTheme()

/* ---------------------
  卡片样式
---------------------- */
const cardSize = ref(localStorage.getItem("lumina-card-size") || "medium")
const cardSizeOptions = [
  { label: "Small", value: "small" },
  { label: "Medium", value: "medium" },
  { label: "Large", value: "large" },
]

const useShadow = ref(localStorage.getItem("lumina-card-shadow") === "1")

const applyCardSize = (size: string) => {
  localStorage.setItem("lumina-card-size", size)
  document.documentElement.setAttribute("data-card-size", size)
}

const applyShadow = (val: boolean) => {
  localStorage.setItem("lumina-card-shadow", val ? "1" : "0")
  document.documentElement.setAttribute("data-card-shadow", val ? "1" : "0")
}

/* ---------------------
  背景设置
---------------------- */
const bgMode = ref(localStorage.getItem("lumina-bg-mode") || "none")
const customBg = ref(localStorage.getItem("lumina-bg-custom") || "")

const applyBackground = () => {
  localStorage.setItem("lumina-bg-mode", bgMode.value)

  if (bgMode.value === "custom" && customBg.value) {
    localStorage.setItem("lumina-bg-custom", customBg.value)
    document.documentElement.style.setProperty(
      "--lumina-bg",
      `url(${customBg.value})`
    )
  } else if (bgMode.value === "soft") {
    document.documentElement.style.setProperty("--lumina-bg", "var(--lumina-surface-secondary)")
  } else {
    document.documentElement.style.removeProperty("--lumina-bg")
  }
}
</script>

<style scoped lang="scss">
.panel {
  display: flex;
  flex-direction: column;
  gap: var(--lumina-gap-xl);
}

.panel-section {
  background: var(--lumina-card-bg);
  padding: var(--lumina-gap-lg);
  border-radius: var(--lumina-radius-lg);
  box-shadow: var(--lumina-shadow-sm);
  border: 1px solid var(--lumina-card-border);

  h2 {
    font-size: var(--lumina-font-title);
    font-weight: 600;
    margin-bottom: var(--lumina-gap-md);
  }

  .bg-input {
    margin-top: var(--lumina-gap-md);
  }
}
</style>
