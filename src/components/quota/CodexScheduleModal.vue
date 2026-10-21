<template>
  <NModal :show="show" :mask-closable="!saving" @update:show="emit('update:show', $event)">
    <section
      class="schedule-panel"
      role="dialog"
      aria-modal="true"
      aria-labelledby="codex-schedule-title"
      :aria-busy="saving"
      @keydown="handleShortcut"
    >
      <header class="schedule-header">
        <div class="header-identity">
          <span class="schedule-symbol"><Icon icon="solar:clock-circle-linear" /></span>
          <div>
            <h2 id="codex-schedule-title">{{ t('codexSchedule.title') }}</h2>
            <p>{{ t('codexSchedule.subtitle') }}</p>
          </div>
        </div>
        <WorkbenchCloseButton :disabled="saving" @close="emit('update:show', false)" />
      </header>

      <div v-if="draft" class="schedule-body">
        <form id="codex-schedule-form" class="schedule-form" @submit.prevent="save">
          <div class="enable-row">
            <div>
              <strong>{{ t('codexSchedule.enable') }}</strong>
              <span>{{ t('codexSchedule.localOnly') }}</span>
            </div>
            <NSwitch
              v-model:value="draft.enabled"
              :disabled="saving"
              :aria-label="t('codexSchedule.enable')"
            />
          </div>

          <fieldset class="schedule-group time-group">
            <legend class="sr-only">{{ t('codexSchedule.times') }}</legend>
            <div class="section-heading">
              <span>{{ t('codexSchedule.times') }}</span>
              <button
                v-if="draft.times.length < 8"
                class="add-time"
                type="button"
                :disabled="saving"
                @click="addTime"
              >
                <Icon icon="solar:add-circle-linear" />{{ t('codexSchedule.addTime') }}
              </button>
            </div>
            <div class="time-list">
              <div v-for="(_, index) in draft.times" :key="index" class="time-item">
                <NTimePicker
                  :formatted-value="draft.times[index]"
                  format="HH:mm"
                  value-format="HH:mm"
                  size="small"
                  input-readonly
                  :clearable="false"
                  :bordered="false"
                  :disabled="saving"
                  :actions="['confirm']"
                  :theme-overrides="timePickerTheme"
                  placement="bottom-start"
                  :aria-label="t('codexSchedule.timeLabel', { index: index + 1 })"
                  @update:formatted-value="updateTime(index, $event)"
                >
                  <template #icon><Icon icon="solar:alt-arrow-down-linear" /></template>
                </NTimePicker>
                <button
                  class="remove-time"
                  type="button"
                  :disabled="saving || draft.times.length === 1"
                  :aria-label="t('codexSchedule.removeTime')"
                  :title="t('codexSchedule.removeTime')"
                  @click="draft.times.splice(index, 1)"
                >
                  ×
                </button>
              </div>
            </div>
            <div class="section-heading days-heading">{{ t('codexSchedule.days') }}</div>
            <div class="weekday-list">
              <button
                v-for="day in 7"
                :key="day"
                type="button"
                :aria-pressed="draft.weekdays.includes(day)"
                :disabled="saving"
                :class="{ selected: draft.weekdays.includes(day) }"
                @click="toggleDay(day)"
              >
                {{ t(`codexSchedule.day${day}`) }}
              </button>
            </div>
          </fieldset>

          <div class="schedule-group timing-fields">
            <label
              ><span>{{ t('codexSchedule.interval') }}</span
              ><NSelect
                v-model:value="draft.checkIntervalSeconds"
                size="small"
                :disabled="saving"
                :options="intervalOptions"
                :aria-label="t('codexSchedule.interval')"
            /></label>
            <label
              ><span>{{ t('codexSchedule.maxWait') }}</span
              ><NInputNumber
                v-model:value="draft.maxWaitMinutes"
                size="small"
                :disabled="saving"
                :min="5"
                :max="60"
                :precision="0"
                :aria-label="t('codexSchedule.maxWait')"
                ><template #suffix>{{ t('codexSchedule.minutes') }}</template></NInputNumber
              ></label
            >
          </div>
          <details class="schedule-help">
            <summary>{{ t('codexSchedule.howItWorks') }}</summary>
            <p>{{ t('codexSchedule.safeguards') }}</p>
            <div class="request-example">
              <span>{{ t('codexSchedule.requestContent') }}</span
              ><code>Reply exactly OK.</code>
            </div>
          </details>
        </form>

        <section class="schedule-history" :aria-label="t('codexSchedule.history')">
          <div class="history-heading">
            <h3>
              {{ t('codexSchedule.history')
              }}<span v-if="data?.runs.length" class="history-count">{{ data.runs.length }}</span>
            </h3>
            <span>{{ timezone }}</span>
          </div>
          <p v-if="!data?.runs.length" class="history-empty">{{ t('codexSchedule.empty') }}</p>
          <ol v-else tabindex="0" :aria-label="t('codexSchedule.history')">
            <li v-for="run in data.runs" :key="run.id">
              <div class="run-heading">
                <time>{{ formatTime(run.scheduledAt) }}</time>
                <span class="run-status" :data-status="run.status">{{
                  t(`codexSchedule.status.${run.status}`)
                }}</span>
              </div>
              <p>{{ t(`codexSchedule.detail.${run.detail}`) }}</p>
              <span class="run-meta"
                >{{ t('codexSchedule.checkCount', { count: run.checks })
                }}<template v-if="isActive(run) && run.nextCheckAt">
                  ·
                  {{
                    t('codexSchedule.nextCheck', { time: formatTime(run.nextCheckAt, true) })
                  }}</template
                ></span
              >
            </li>
          </ol>
        </section>
      </div>
      <footer v-if="draft" class="schedule-footer">
        <p v-if="error" class="schedule-error" role="alert">{{ error }}</p>
        <div class="schedule-actions">
          <WorkbenchButton :disabled="saving || busy || dirty" @click="warmNow">{{
            t('codexSchedule.warmNow')
          }}</WorkbenchButton>
          <span class="save-feedback" role="status">{{ notice }}</span>
          <WorkbenchButton
            v-if="busy || data?.settings.enabled"
            :disabled="saving"
            @click="pause"
            >{{ t('codexSchedule.pause') }}</WorkbenchButton
          >
          <WorkbenchButton
            form="codex-schedule-form"
            type="submit"
            variant="primary"
            :disabled="saving"
            :shortcut="saveShortcut"
            >{{ saving ? t('common.saving') : t('common.saveAndApply') }}</WorkbenchButton
          >
        </div>
      </footer>
      <div v-else class="schedule-loading">
        <p v-if="error" role="alert">{{ error }}</p>
        <span v-else>{{ t('common.loading') }}</span
        ><WorkbenchButton v-if="error" @click="initialize">{{
          t('codexSchedule.retry')
        }}</WorkbenchButton>
      </div>
    </section>
  </NModal>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { NInputNumber, NModal, NSelect, NSwitch, NTimePicker } from 'naive-ui'
import { Icon } from '@iconify/vue'
import { useI18n } from 'vue-i18n'
import WorkbenchButton from '@/components/workbench/WorkbenchButton.vue'
import WorkbenchCloseButton from '@/components/workbench/WorkbenchCloseButton.vue'
import { formatPrimaryShortcut, hasPrimaryModifier } from '@/utils/platform-shortcuts'
import {
  loadCodexSchedule,
  runCodexWarmupNow,
  saveCodexSchedule,
  type CodexScheduleData,
  type CodexScheduleRun,
  type CodexScheduleSettings,
} from '@/services/quota/quota-scheduler'

const props = defineProps<{ show: boolean }>()
const emit = defineEmits<{ (event: 'update:show', value: boolean): void }>()
const { t, locale } = useI18n()
const data = ref<CodexScheduleData | null>(null)
const draft = ref<CodexScheduleSettings | null>(null)
const error = ref('')
const notice = ref('')
const saving = ref(false)
const saveShortcut = formatPrimaryShortcut('s')
const timePickerTheme = {
  borderRadius: '8px',
  panelColor: 'var(--lumina-surface-elevated)',
  panelBoxShadow: 'var(--lumina-shadow-md)',
  panelDividerColor: 'var(--lumina-separator)',
  itemColorHover: 'var(--lumina-control-hover)',
  itemTextColor: 'var(--lumina-text)',
  itemTextColorActive: 'var(--lumina-primary)',
  itemBorderRadius: '5px',
  itemHeight: '28px',
  itemWidth: '46px',
  itemFontSize: '13px',
}
const timezone = Intl.DateTimeFormat().resolvedOptions().timeZone
let timer: ReturnType<typeof setTimeout> | undefined
let generation = 0
let mutationVersion = 0
const dirty = computed(
  () => JSON.stringify(draft.value) !== JSON.stringify(data.value?.settings ?? null)
)
watch(dirty, changed => {
  if (changed) notice.value = ''
})
const isActive = (run: CodexScheduleRun) =>
  ['waiting', 'checking', 'warming', 'verifying', 'backoff'].includes(run.status)
const busy = computed(() => data.value?.runs.some(isActive) ?? false)
const intervalOptions = computed(() =>
  [30, 60, 120, 300].map(value => ({
    value,
    label:
      value < 60
        ? t('codexSchedule.secondsValue', { count: value })
        : t('codexSchedule.minutesValue', { count: value / 60 }),
  }))
)

function cloneSettings(settings: CodexScheduleSettings): CodexScheduleSettings {
  return { ...settings, times: [...settings.times], weekdays: [...settings.weekdays] }
}
function explain(cause: unknown) {
  const code = typeof cause === 'string' && /^[a-zA-Z]+$/.test(cause) ? cause : 'unknown'
  const key = `codexSchedule.detail.${code}`
  const text = t(key)
  return text === key ? t('codexSchedule.detail.unknown') : text
}
function toggleDay(day: number) {
  if (!draft.value) return
  draft.value.weekdays = draft.value.weekdays.includes(day)
    ? draft.value.weekdays.filter(value => value !== day)
    : [...draft.value.weekdays, day].sort()
}
function updateTime(index: number, value: string | null) {
  if (draft.value && value && /^\d{2}:\d{2}$/.test(value)) draft.value.times[index] = value
}
function addTime() {
  if (!draft.value || draft.value.times.length >= 8) return
  const available = ['08:00', '13:00', '18:00', '19:00', '09:00', '12:00', '17:00', '20:00']
  const next = available.find(time => !draft.value!.times.includes(time))
  if (next) draft.value.times.push(next)
}
function handleShortcut(event: KeyboardEvent) {
  if (hasPrimaryModifier(event) && event.key.toLowerCase() === 's' && !event.isComposing) {
    event.preventDefault()
    event.stopPropagation()
    if (draft.value && !saving.value) void save()
  }
}
function formatTime(timestamp: number, short = false) {
  return new Date(timestamp * 1000).toLocaleString(
    locale.value,
    short
      ? { hour: '2-digit', minute: '2-digit', second: '2-digit' }
      : { month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' }
  )
}
async function update(action: () => Promise<CodexScheduleData>, successMessage = '') {
  mutationVersion++
  saving.value = true
  error.value = ''
  notice.value = ''
  try {
    const result = await action()
    data.value = result
    draft.value = cloneSettings(result.settings)
    notice.value = successMessage
  } catch (cause) {
    error.value = explain(cause)
  } finally {
    saving.value = false
  }
}
async function save() {
  if (!draft.value) return
  if (
    !Number.isInteger(draft.value.maxWaitMinutes) ||
    draft.value.maxWaitMinutes < 5 ||
    draft.value.maxWaitMinutes > 60
  ) {
    error.value = t('codexSchedule.detail.invalidSettings')
    return
  }
  const settings = cloneSettings(draft.value)
  await update(() => saveCodexSchedule(settings), t('common.savedSuccessfully'))
}
async function pause() {
  if (data.value) await update(() => saveCodexSchedule({ ...data.value!.settings, enabled: false }))
}
async function warmNow() {
  await update(runCodexWarmupNow)
}

async function poll(version: number) {
  if (!props.show || version !== generation) return
  try {
    if (!saving.value) {
      const revision = mutationVersion
      const result = await loadCodexSchedule()
      if (version === generation && revision === mutationVersion && !saving.value)
        data.value = result
    }
  } catch (cause) {
    if (version === generation) error.value = explain(cause)
  }
  if (props.show && version === generation)
    timer = setTimeout(() => {
      void poll(version)
    }, 3000)
}
async function initialize() {
  clearTimeout(timer)
  const version = ++generation
  error.value = ''
  notice.value = ''
  draft.value = null
  try {
    const result = await loadCodexSchedule()
    if (version !== generation || !props.show) return
    data.value = result
    draft.value = cloneSettings(result.settings)
    timer = setTimeout(() => {
      void poll(version)
    }, 3000)
  } catch (cause) {
    if (version === generation) error.value = explain(cause)
  }
}
watch(
  () => props.show,
  show => {
    if (show) void initialize()
    else {
      generation++
      clearTimeout(timer)
    }
  },
  { immediate: true }
)
onBeforeUnmount(() => {
  generation++
  clearTimeout(timer)
})
</script>

<style scoped lang="scss">
.schedule-panel {
  background: var(--lumina-window-bg);
  border: 0.5px solid var(--lumina-separator-strong);
  border-radius: 12px;
  box-shadow: var(--lumina-shadow-lg);
  color: var(--lumina-text);
  display: flex;
  flex-direction: column;
  font-family: var(--lumina-font-sans);
  max-height: min(760px, calc(100vh - 56px));
  overflow: hidden;
  width: min(560px, calc(100vw - 40px));
}
.schedule-header {
  align-items: center;
  background: var(--lumina-toolbar-bg);
  border-bottom: 0.5px solid var(--lumina-separator);
  display: flex;
  justify-content: space-between;
  padding: 18px 20px;
}
.header-identity {
  align-items: center;
  display: flex;
  gap: 12px;
  min-width: 0;
}
.schedule-symbol {
  align-items: center;
  background: var(--lumina-primary-soft);
  border-radius: 9px;
  color: var(--lumina-primary);
  display: flex;
  flex: 0 0 34px;
  font-size: 20px;
  height: 34px;
  justify-content: center;
}
.schedule-header h2 {
  font-size: 15px;
  font-weight: 600;
  letter-spacing: -0.02em;
  line-height: 20px;
  margin: 0 0 3px;
}
.schedule-header p {
  color: var(--lumina-text-secondary);
  font-size: 12px;
  line-height: 17px;
  margin: 0;
}
.schedule-body {
  min-height: 0;
  overflow-y: auto;
  overscroll-behavior: contain;
  padding: 16px 20px;
}
.schedule-form {
  display: grid;
  gap: 12px;
}
.enable-row,
.schedule-group {
  background: var(--lumina-surface-elevated);
  border: 0.5px solid var(--lumina-separator);
  border-radius: 9px;
}
.enable-row {
  align-items: center;
  display: flex;
  gap: 16px;
  justify-content: space-between;
  padding: 12px 14px;
}
.enable-row strong {
  display: block;
  font-size: 13px;
  font-weight: 500;
  line-height: 18px;
}
.enable-row span {
  color: var(--lumina-text-secondary);
  display: block;
  font-size: 11px;
  line-height: 16px;
  margin-top: 3px;
}
fieldset {
  margin: 0;
  min-width: 0;
}
.time-group {
  padding: 12px 14px 14px;
}
.sr-only {
  clip-path: inset(50%);
  height: 1px;
  overflow: hidden;
  position: absolute;
  white-space: nowrap;
  width: 1px;
}
.section-heading {
  align-items: center;
  display: flex;
  font-size: 12px;
  font-weight: 500;
  justify-content: space-between;
  line-height: 22px;
  margin-bottom: 9px;
}
.add-time {
  align-items: center;
  background: transparent;
  border: 0;
  border-radius: 4px;
  color: var(--lumina-primary);
  cursor: pointer;
  display: inline-flex;
  font-size: 11px;
  font-weight: 500;
  gap: 4px;
  padding: 1px 3px;
}
.add-time :deep(svg) {
  font-size: 14px;
}
.add-time:hover:not(:disabled) {
  background: var(--lumina-primary-soft);
}
.time-list {
  display: grid;
  gap: 8px;
  grid-template-columns: repeat(auto-fit, minmax(118px, 1fr));
}
.time-item {
  align-items: center;
  background: var(--lumina-input-bg);
  border: 0.5px solid var(--lumina-input-border);
  border-radius: 6px;
  display: flex;
  min-width: 0;
  transition:
    background 150ms ease,
    box-shadow 150ms ease;
}
.time-item:hover {
  background: var(--lumina-control-bg);
}
.time-item:focus-within {
  box-shadow: 0 0 0 3px var(--lumina-accent-ring);
}
.time-item :deep(.n-time-picker) {
  flex: 1;
  min-width: 0;
}
.time-item :deep(.n-input) {
  background: transparent;
  cursor: pointer;
}
.time-item :deep(.n-input__input-el) {
  cursor: pointer;
  font-family: var(--lumina-font-mono);
  font-size: 13px;
  font-variant-numeric: tabular-nums;
}
.time-item :deep(.n-input__suffix) {
  color: var(--lumina-text-secondary);
  font-size: 12px;
}
.remove-time {
  align-items: center;
  background: transparent;
  border: 0;
  border-radius: 4px;
  color: var(--lumina-text-secondary);
  cursor: pointer;
  display: flex;
  flex: 0 0 22px;
  font-size: 15px;
  height: 24px;
  justify-content: center;
  margin-right: 3px;
  padding: 0;
  transition:
    background 150ms ease,
    color 150ms ease;
}
.remove-time:hover:not(:disabled) {
  background: var(--lumina-control-hover);
  color: var(--lumina-text);
}
button:disabled {
  cursor: default;
  opacity: 0.4;
}
.days-heading {
  margin-top: 16px;
}
.weekday-list {
  background: var(--lumina-control-bg);
  border-radius: 7px;
  display: flex;
  gap: 3px;
  padding: 3px;
}
.weekday-list button {
  background: transparent;
  border: 0.5px solid transparent;
  border-radius: 5px;
  color: var(--lumina-text-secondary);
  cursor: pointer;
  flex: 1;
  font-size: 12px;
  line-height: 24px;
  min-width: 0;
  padding: 0 3px;
  transition:
    background 150ms ease,
    color 150ms ease,
    box-shadow 150ms ease;
}
.weekday-list button:hover:not(:disabled) {
  color: var(--lumina-text);
}
.weekday-list button.selected {
  background: var(--lumina-surface-elevated);
  box-shadow: var(--lumina-shadow-sm);
  color: var(--lumina-primary);
  font-weight: 600;
}
.timing-fields {
  padding: 0 14px;
}
.timing-fields label {
  align-items: center;
  display: flex;
  gap: 16px;
  justify-content: space-between;
  min-height: 48px;
}
.timing-fields label + label {
  border-top: 0.5px solid var(--lumina-separator);
}
.timing-fields label > span {
  font-size: 12px;
  line-height: 18px;
}
.timing-fields :deep(.n-select),
.timing-fields :deep(.n-input-number) {
  flex: 0 0 152px;
  width: 152px;
}
.schedule-help {
  color: var(--lumina-text-secondary);
  font-size: 11px;
  line-height: 1.7;
  padding: 0 3px;
}
.schedule-help summary {
  border-radius: 4px;
  cursor: pointer;
  width: fit-content;
}
.schedule-help p {
  margin: 8px 0;
}
.request-example {
  align-items: center;
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}
.request-example code {
  background: var(--lumina-control-bg);
  border-radius: 4px;
  color: var(--lumina-text);
  font-family: var(--lumina-font-mono);
  font-size: 11px;
  padding: 2px 6px;
}
.schedule-history {
  margin-top: 22px;
}
.history-heading {
  align-items: center;
  display: flex;
  justify-content: space-between;
  margin-bottom: 9px;
}
.history-heading h3 {
  align-items: center;
  display: flex;
  font-size: 12px;
  font-weight: 500;
  gap: 6px;
  line-height: 18px;
  margin: 0;
}
.history-count {
  background: var(--lumina-control-bg);
  border-radius: 4px;
  color: var(--lumina-text-secondary);
  font-size: 10px;
  font-weight: 400;
  padding: 0 5px;
}
.history-heading > span,
.run-meta {
  color: var(--lumina-text-secondary);
  font-size: 10px;
}
.history-empty {
  color: var(--lumina-text-secondary);
  font-size: 12px;
  line-height: 1.6;
  margin: 0;
  padding: 12px 0;
}
.schedule-history ol {
  background: var(--lumina-surface-elevated);
  border: 0.5px solid var(--lumina-separator);
  border-radius: 8px;
  list-style: none;
  margin: 0;
  max-height: 168px;
  overflow-y: auto;
  padding: 0 12px;
}
.schedule-history li {
  padding: 10px 0;
}
.schedule-history li + li {
  border-top: 0.5px solid var(--lumina-separator);
}
.run-heading {
  align-items: center;
  display: flex;
  justify-content: space-between;
}
.run-heading time {
  font-size: 11px;
  font-variant-numeric: tabular-nums;
}
.run-status {
  align-items: center;
  color: var(--lumina-text-secondary);
  display: inline-flex;
  font-size: 10px;
  gap: 5px;
}
.run-status::before {
  background: currentColor;
  border-radius: 50%;
  content: '';
  height: 4px;
  width: 4px;
}
.run-status[data-status='success'],
.run-status[data-status='ready'] {
  color: var(--lumina-success);
}
.run-status[data-status='failed'],
.run-status[data-status='expired'] {
  color: var(--lumina-danger);
}
.run-status[data-status='checking'],
.run-status[data-status='warming'],
.run-status[data-status='verifying'] {
  color: var(--lumina-primary);
}
.schedule-history li p {
  color: var(--lumina-text-secondary);
  font-size: 11px;
  line-height: 1.6;
  margin: 4px 0 2px;
}
.schedule-footer {
  background: var(--lumina-toolbar-bg);
  border-top: 0.5px solid var(--lumina-separator);
  padding: 12px 20px;
}
.schedule-actions {
  align-items: center;
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.schedule-actions :deep(.workbench-button) {
  font-size: 12px;
  height: 28px;
}
.save-feedback {
  color: var(--lumina-text-secondary);
  flex: 1;
  font-size: 11px;
}
.schedule-error {
  color: var(--lumina-danger);
  font-size: 12px;
  line-height: 1.5;
  margin: 0 0 10px;
}
.schedule-loading {
  display: grid;
  gap: 12px;
  padding: 24px;
}
button:focus-visible,
summary:focus-visible,
.schedule-history ol:focus-visible {
  outline: 2px solid var(--lumina-primary);
  outline-offset: 2px;
}
@media (prefers-reduced-motion: reduce) {
  .schedule-panel * {
    transition: none !important;
  }
}
@media (max-width: 480px) {
  .schedule-actions {
    justify-content: flex-end;
  }
  .save-feedback {
    flex-basis: 100%;
    order: 1;
  }
  .schedule-actions :deep(.workbench-shortcut) {
    display: none;
  }
}
</style>
