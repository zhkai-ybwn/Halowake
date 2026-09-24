<template>
  <section class="filter-strip">
    <!-- 1. Date Range Control with Inline Segmented Shortcuts -->
    <div class="filter-item filter-item-date">
      <div class="inline-date-segmented">
        <button
          type="button"
          :class="{ active: isCurrentDay(0) }"
          @click="setDateRange(0)"
        >
          {{ t('codexReport.today') }}
        </button>
        <button
          type="button"
          :class="{ active: isCurrentDay(-1) }"
          @click="setDateRange(-1)"
        >
          {{ t('codexReport.yesterday') }}
        </button>
      </div>
      <NDatePicker
        :value="dateRange"
        type="daterange"
        size="small"
        clearable
        class="compact-date-picker"
        @update:value="handleDateRangeUpdate"
      />
    </div>

    <!-- 2. AI Tool Provider Filter -->
    <div class="filter-item filter-item-provider">
      <div class="inline-provider-segmented" role="tablist">
        <button
          type="button"
          :class="{ active: selectedProvider === 'all' }"
          :title="t('codexReport.allTools')"
          @click="emit('update:selectedProvider', 'all')"
        >
          <Icon icon="solar:layers-minimalistic-linear" class="provider-btn-icon" />
          <span>{{ t('codexReport.allTools') }}</span>
        </button>
        <button
          type="button"
          :class="{ active: selectedProvider === 'codex' }"
          title="Codex CLI"
          @click="emit('update:selectedProvider', 'codex')"
        >
          <Icon icon="solar:code-square-linear" class="provider-btn-icon icon-codex" />
          <span>Codex</span>
        </button>
        <button
          type="button"
          :class="{ active: selectedProvider === 'claude' }"
          title="Claude Code"
          @click="emit('update:selectedProvider', 'claude')"
        >
          <Icon icon="solar:magic-stick-3-linear" class="provider-btn-icon icon-claude" />
          <span>Claude</span>
        </button>
        <button
          type="button"
          :class="{ active: selectedProvider === 'antigravity' }"
          title="Antigravity"
          @click="emit('update:selectedProvider', 'antigravity')"
        >
          <Icon icon="solar:planet-linear" class="provider-btn-icon icon-agy" />
          <span>Antigravity</span>
        </button>
        <button
          type="button"
          :class="{ active: selectedProvider === 'opencode' }"
          title="OpenCode"
          @click="emit('update:selectedProvider', 'opencode')"
        >
          <Icon icon="solar:terminal-linear" class="provider-btn-icon icon-opencode" />
          <span>OpenCode</span>
        </button>
      </div>
    </div>

    <!-- 3. Project Filter Button -->
    <div class="filter-item filter-item-project">
      <button
        type="button"
        class="project-config-trigger"
        :class="{ 'is-filtered': selectedProjects.length && selectedProjects.length < allProjectCount }"
        :title="t('codexReport.manageProjects')"
        @click="emit('open-project-modal')"
      >
        <div class="trigger-main">
          <Icon icon="solar:folder-with-files-linear" class="trigger-icon" />
          <span class="trigger-text">
            {{
              !selectedProjects.length || selectedProjects.length === allProjectCount
                ? t('codexReport.allProjectsSelected', { count: allProjectCount })
                : t('codexReport.selectedProjectsCount', { count: selectedProjects.length })
            }}
          </span>
        </div>
        <div class="trigger-badge">
          <Icon icon="solar:tuning-2-linear" />
        </div>
      </button>
    </div>

    <!-- 4. Keyword Search -->
    <div class="filter-item filter-item-search">
      <NInput
        :value="keyword"
        size="small"
        clearable
        :placeholder="t('codexReport.searchPlaceholder')"
        class="compact-search-input"
        @update:value="emit('update:keyword', $event)"
      >
        <template #prefix>
          <Icon icon="solar:magnifer-linear" class="search-prefix-icon" />
        </template>
      </NInput>
    </div>

    <!-- 5. Auto Exclude Toggle -->
    <div class="filter-item filter-item-auto-exclude">
      <NCheckbox
        :checked="autoExcludeInvalid"
        size="small"
        @update:checked="emit('update:autoExcludeInvalid', $event)"
      >
        <span class="checkbox-label" :title="t('codexReport.autoExcludeHint')">
          {{ t('codexReport.autoExclude') }}
        </span>
      </NCheckbox>
    </div>
  </section>
</template>

<script setup lang="ts">
import { NCheckbox, NDatePicker, NInput } from 'naive-ui'
import { useLocale } from '@/composables/useLocale'
import type { AiToolProvider } from '@/types/codex-report'

const props = defineProps<{
  dateRange: [number, number] | null
  selectedProvider: AiToolProvider
  selectedProjects: string[]
  allProjectCount: number
  keyword: string
  autoExcludeInvalid: boolean
}>()

const emit = defineEmits<{
  (e: 'update:dateRange', value: [number, number] | null): void
  (e: 'update:selectedProvider', value: AiToolProvider): void
  (e: 'update:keyword', value: string): void
  (e: 'update:autoExcludeInvalid', value: boolean): void
  (e: 'open-project-modal'): void
}>()

const { t } = useLocale()

function handleDateRangeUpdate(val: [number, number] | null) {
  if (!val || !val[0] || !val[1]) {
    emit('update:dateRange', val)
    return
  }
  const start = val[0]
  const end = new Date(val[1])
  if (
    end.getHours() === 0 &&
    end.getMinutes() === 0 &&
    end.getSeconds() === 0 &&
    end.getMilliseconds() === 0
  ) {
    end.setHours(23, 59, 59, 999)
  }
  emit('update:dateRange', [start, end.getTime()])
}

function setDateRange(dayOffset: number) {
  const day = new Date()
  day.setDate(day.getDate() + dayOffset)
  day.setHours(0, 0, 0, 0)
  emit('update:dateRange', [day.getTime(), day.getTime() + 86_399_999])
}

function isCurrentDay(dayOffset: number) {
  if (!props.dateRange) return false
  const target = new Date()
  target.setDate(target.getDate() + dayOffset)
  target.setHours(0, 0, 0, 0)
  const start = target.getTime()
  const end = start + 86_399_999
  return Math.abs(props.dateRange[0] - start) < 1000 && Math.abs(props.dateRange[1] - end) < 1000
}
</script>

<style scoped lang="scss">
.filter-strip {
  align-items: center;
  background: var(--lumina-surface-2);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-md);
  box-shadow: var(--lumina-shadow-sm);
  display: flex;
  flex: 0 0 auto;
  gap: 10px;
  min-height: 42px;
  padding: 6px 12px;
}

.filter-item {
  align-items: center;
  display: inline-flex;
}

.filter-item-date {
  align-items: center;
  display: flex;
  gap: 6px;
}

.inline-date-segmented {
  background: var(--lumina-surface-3);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-sm);
  display: inline-flex;
  gap: 2px;
  padding: 2px;

  button {
    background: transparent;
    border: none;
    border-radius: var(--lumina-radius-xs);
    color: var(--lumina-text-secondary);
    cursor: pointer;
    font-size: 11px;
    font-weight: 500;
    padding: 3px 8px;
    transition: all var(--lumina-duration-fast) var(--lumina-ease-out);

    &:hover {
      color: var(--lumina-text);
    }

    &.active {
      background: var(--lumina-surface-elevated);
      box-shadow: var(--lumina-shadow-xs);
      color: var(--lumina-primary);
      font-weight: 600;
    }
  }
}

.compact-date-picker {
  width: 220px;
}

.inline-provider-segmented {
  background: var(--lumina-surface-3);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-sm);
  display: inline-flex;
  gap: 2px;
  padding: 2px;

  button {
    align-items: center;
    background: transparent;
    border: none;
    border-radius: var(--lumina-radius-xs);
    color: var(--lumina-text-secondary);
    cursor: pointer;
    display: inline-flex;
    font-size: 11px;
    font-weight: 500;
    gap: 4px;
    padding: 3px 8px;
    transition: all var(--lumina-duration-fast) var(--lumina-ease-out);

    &:hover {
      color: var(--lumina-text);
    }

    &.active {
      background: var(--lumina-surface-elevated);
      box-shadow: var(--lumina-shadow-xs);
      color: var(--lumina-primary);
      font-weight: 600;

      .icon-codex {
        color: #10b981;
      }
      .icon-claude {
        color: #f97316;
      }
      .icon-agy {
        color: #6366f1;
      }
      .icon-opencode {
        color: #a855f7;
      }
    }

    .provider-btn-icon {
      font-size: 12px;
    }
  }
}

.filter-item-project {
  display: flex;
}

.project-config-trigger {
  align-items: center;
  background: var(--lumina-surface-3);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-sm);
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: inline-flex;
  gap: 8px;
  height: 28px;
  padding: 0 8px;
  transition: all var(--lumina-duration-fast) var(--lumina-ease-out);

  &:hover {
    background: var(--lumina-surface-elevated);
    border-color: var(--lumina-separator-strong);
    color: var(--lumina-text);
  }

  &.is-filtered {
    background: var(--lumina-primary-soft);
    border-color: var(--lumina-primary);
    color: var(--lumina-primary);
    font-weight: 600;
  }

  .trigger-main {
    align-items: center;
    display: inline-flex;
    gap: 5px;
    max-width: 140px;
  }

  .trigger-icon {
    font-size: 12px;
  }

  .trigger-text {
    font-size: 11px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .trigger-badge {
    align-items: center;
    display: flex;
    font-size: 11px;
    opacity: 0.7;
  }
}

.filter-item-search {
  flex: 1;
  min-width: 140px;
}

.compact-search-input {
  width: 100%;
}

.search-prefix-icon {
  color: var(--lumina-text-tertiary);
  font-size: 12px;
}

.filter-item-auto-exclude {
  margin-left: auto;
}

.checkbox-label {
  color: var(--lumina-text-secondary);
  font-size: 11px;
  user-select: none;
  white-space: nowrap;
}

@media (max-width: 1024px) {
  .filter-strip {
    flex-wrap: wrap;
  }
}
</style>
