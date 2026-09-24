<template>
  <div class="agent-pills-wrap">
    <span
      v-for="agent in agents"
      :key="agent"
      class="agent-micro-pill"
      :data-agent="agent"
      :title="getLabel(agent)"
    >
      {{ getLabel(agent) }}
    </span>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  agents: string[]
}>()

function getLabel(agent: string): string {
  switch (agent.toLowerCase()) {
    case 'antigravity':
      return 'Antigravity'
    case 'claude':
      return 'Claude Code'
    case 'cursor':
      return 'Cursor'
    case 'trae':
      return 'Trae'
    case 'cline':
      return 'Cline'
    case 'windsurf':
      return 'Windsurf'
    case 'project':
      return '项目'
    default:
      return agent
  }
}
</script>

<style scoped lang="scss">
.agent-pills-wrap {
  display: inline-flex;
  flex-wrap: wrap;
  gap: 4px;
}

.agent-micro-pill {
  align-items: center;
  background: var(--lumina-surface-secondary);
  border: 0.5px solid var(--lumina-separator);
  border-radius: 4px;
  color: var(--lumina-text-secondary);
  display: inline-flex;
  font-size: 10.5px;
  font-weight: 500;
  line-height: 1.2;
  padding: 1.5px 6px;
}

.agent-micro-pill[data-agent='antigravity'] {
  border-color: color-mix(in srgb, var(--lumina-primary) 30%, transparent);
  color: var(--lumina-primary);
}

.agent-micro-pill[data-agent='claude'] {
  border-color: color-mix(in srgb, var(--lumina-warning, #b76500) 30%, transparent);
  color: var(--lumina-warning, #b76500);
}

.agent-micro-pill[data-agent='cursor'] {
  border-color: color-mix(in srgb, #8b5cf6 30%, transparent);
  color: #8b5cf6;
}
</style>
