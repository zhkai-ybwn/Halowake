<template>
  <div class="sidebar">
    <!-- 顶部头像 -->
    <div class="sidebar-top">
      <img src="@/assets/logo.png" class="avatar" />
    </div>

    <!-- 中部分组 -->
    <div class="sidebar-menu">
      <SidebarItem
        v-for="g in groups"
        :key="g.id"
        :icon="g.icon"
        :label="g.name"
        :active="g.id === activeGroup"
        @click="changeGroup(g.id)"
      />
    </div>

    <!-- 底部系统按钮 -->
    <div class="sidebar-bottom">
      <SidebarItem
        :icon="theme === 'light' ? 'solar:sun-2-linear' : 'solar:moon-linear'"
        label=""
        @click="toggleTheme"
      />

      <SidebarItem icon="solar:settings-linear" label="" @click="openSettings" />
    </div>
  </div>
</template>

<script setup lang="ts">
import SidebarItem from './SidebarItem.vue'
import { ref } from 'vue'
import { useTheme } from '@/hooks/useTheme'
import groups from '@/config/groups.json'

const { theme, toggleTheme } = useTheme()

// 默认选中第一个分组
const activeGroup = ref(groups[0].id)
const changeGroup = (id: string) => {
  activeGroup.value = id
  emit('update:groupId', id)
}

const emit = defineEmits(['update:groupId', 'open-settings'])

const openSettings = () => {
  emit('open-settings')
}
</script>

<style scoped lang="scss">
.sidebar {
  width: var(--lumina-sidebar-width);
  height: 100vh;

  display: flex;
  flex-direction: column;

  padding: var(--lumina-gap-lg) 0;

  background: var(--lumina-bg-soft);
  backdrop-filter: blur(14px);
  border-right: 1px solid var(--lumina-card-border);

  position: relative;
  z-index: 10;
}

.sidebar-top {
  display: flex;
  justify-content: center;
  margin-bottom: var(--lumina-gap-xl);

  .avatar {
    width: 42px;
    height: 42px;
    border-radius: 50%;
    border: 2px solid var(--lumina-separator-strong);
    object-fit: cover;
  }
}

.sidebar-menu {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: var(--lumina-gap-md);
}

.sidebar-bottom {
  display: flex;
  flex-direction: column;
  gap: var(--lumina-gap-lg);
  padding-bottom: var(--lumina-gap-lg);
  justify-content: center;
  align-items: center;
}
</style>
