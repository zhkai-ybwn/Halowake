<template>
  <div>
    <!-- macOS Prompt Template Drawer -->
    <WorkbenchDrawer
      v-if="show"
      :title="t('codexReport.promptTitle')"
      :description="t('codexReport.promptHint')"
      :close-label="t('common.close')"
      size="normal"
      @close="emit('update:show', false)"
    >
      <div class="prompt-drawer-container">
        <!-- Dynamic Template Switcher Strip -->
        <div class="prompt-preset-row">
          <span class="preset-label">{{ t('codexReport.presetTemplateSelect') }}:</span>
          <div class="preset-buttons">
            <button
              v-for="tpl in promptTemplates"
              :key="tpl.id"
              type="button"
              class="preset-chip"
              :class="{ 'is-active': activeTemplateId === tpl.id }"
              @click="selectTemplate(tpl)"
            >
              <span>{{ tpl.name }}</span>
              <span v-if="tpl.isBuiltin" class="chip-tag">{{ t('codexReport.builtinBadge') }}</span>
              <span
                v-if="!tpl.isBuiltin && activeTemplateId === tpl.id"
                class="chip-del"
                :title="t('common.delete')"
                @click.stop="handleDeleteTemplate(tpl)"
              >
                ×
              </span>
            </button>
            <button
              type="button"
              class="preset-chip new-template-btn"
              @click="openCreateTemplateModal"
            >
              + {{ t('codexReport.createTemplate') }}
            </button>
          </div>
        </div>

        <NInput
          v-model:value="promptDraft"
          type="textarea"
          :autosize="false"
          class="prompt-editor-textarea"
          :placeholder="t('codexReport.templateContentPlaceholder')"
        />

        <footer class="drawer-footer">
          <div class="drawer-footer-left">
            <WorkbenchButton
              v-if="currentActiveTemplate?.isBuiltin"
              @click="handleRestoreBuiltin"
            >
              {{ t('codexReport.restoreDefault') }}
            </WorkbenchButton>
            <WorkbenchButton
              v-else-if="currentActiveTemplate"
              variant="secondary"
              @click="handleDeleteTemplate(currentActiveTemplate)"
            >
              {{ t('common.delete') }}
            </WorkbenchButton>
          </div>
          <div class="drawer-footer-right">
            <WorkbenchButton variant="primary" @click="saveCurrentPrompt">
              {{ t('common.save') }}
            </WorkbenchButton>
          </div>
        </footer>
      </div>
    </WorkbenchDrawer>

    <!-- Create Custom Template Modal -->
    <NModal
      v-model:show="showCreateTemplateModal"
      class="create-template-modal"
      :mask-closable="true"
      :auto-focus="false"
      :trap-focus="false"
    >
      <WorkbenchSheet
        size="normal"
        icon="solar:document-add-linear"
        :title="t('codexReport.createTemplateTitle')"
        :description="t('codexReport.promptHint')"
        :close-label="t('common.close')"
        @close="showCreateTemplateModal = false"
      >
        <div class="create-template-form">
          <div class="form-field">
            <span class="field-label">{{ t('codexReport.templateName') }}</span>
            <NInput
              v-model:value="newTemplateName"
              :placeholder="t('codexReport.templateNamePlaceholder')"
              maxlength="30"
            />
          </div>
          <div class="form-field">
            <span class="field-label">{{ t('codexReport.templateContent') }}</span>
            <NInput
              v-model:value="newTemplateContent"
              type="textarea"
              :rows="10"
              :placeholder="t('codexReport.templateContentPlaceholder')"
            />
          </div>
        </div>

        <template #footer>
          <div class="modal-footer-actions">
            <WorkbenchButton @click="showCreateTemplateModal = false">
              {{ t('common.cancel') }}
            </WorkbenchButton>
            <WorkbenchButton variant="primary" @click="handleCreateTemplate">
              {{ t('common.save') }}
            </WorkbenchButton>
          </div>
        </template>
      </WorkbenchSheet>
    </NModal>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { NInput, NModal, useDialog, useMessage } from 'naive-ui'
import WorkbenchDrawer from '@/components/workbench/WorkbenchDrawer.vue'
import WorkbenchSheet from '@/components/workbench/WorkbenchSheet.vue'
import WorkbenchButton from '@/components/workbench/WorkbenchButton.vue'
import {
  deleteCodexReportTemplate,
  loadCodexReportTemplates,
  resetBuiltinCodexReportTemplates,
  saveCodexReportTemplate,
  type CodexReportPromptTemplate,
} from '@/services/codex-report-service'
import { useLocale } from '@/composables/useLocale'
import { DEFAULT_WEB_AI_PROMPT } from '@/utils/codex-report'

const ACTIVE_TEMPLATE_KEY = 'lumina.codex-report.active-template.v1'

const props = defineProps<{
  show: boolean
  activePrompt: string
}>()

const emit = defineEmits<{
  (e: 'update:show', value: boolean): void
  (e: 'update:activePrompt', value: string): void
}>()

const { t } = useLocale()
const message = useMessage()
const dialog = useDialog()

const promptTemplates = ref<CodexReportPromptTemplate[]>([])
const activeTemplateId = ref<string>('builtin-standard')
const promptDraft = ref('')
const showCreateTemplateModal = ref(false)
const newTemplateName = ref('')
const newTemplateContent = ref('')

const currentActiveTemplate = computed(() =>
  promptTemplates.value.find(tpl => tpl.id === activeTemplateId.value) || promptTemplates.value[0] || null
)

watch(
  () => props.show,
  isOpen => {
    if (isOpen) {
      const active = currentActiveTemplate.value
      promptDraft.value = active ? active.content : props.activePrompt
    }
  }
)

onMounted(() => {
  void initPromptTemplates()
})

async function initPromptTemplates() {
  try {
    const templates = await loadCodexReportTemplates()
    promptTemplates.value = templates
    const savedActiveId = localStorage.getItem(ACTIVE_TEMPLATE_KEY)
    if (savedActiveId && templates.some(t => t.id === savedActiveId)) {
      activeTemplateId.value = savedActiveId
    } else if (templates.length > 0) {
      activeTemplateId.value = templates[0].id
    }
    const active = templates.find(t => t.id === activeTemplateId.value)
    if (active) {
      emit('update:activePrompt', active.content)
      promptDraft.value = active.content
    }
  } catch (err) {
    console.error('Failed to load prompt templates:', err)
  }
}

function selectTemplate(template: CodexReportPromptTemplate) {
  activeTemplateId.value = template.id
  try {
    localStorage.setItem(ACTIVE_TEMPLATE_KEY, template.id)
  } catch {
    // Ignore storage errors
  }
  promptDraft.value = template.content
}

async function saveCurrentPrompt() {
  const active = currentActiveTemplate.value
  if (!active) return
  const updated: CodexReportPromptTemplate = {
    ...active,
    content: promptDraft.value.trim() || active.content,
    updatedAt: Date.now(),
  }
  try {
    await saveCodexReportTemplate(updated)
    emit('update:activePrompt', updated.content)
    const index = promptTemplates.value.findIndex(t => t.id === updated.id)
    if (index >= 0) {
      promptTemplates.value[index] = updated
    }
    message.success(t('codexReport.promptSaved'))
    emit('update:show', false)
  } catch (err) {
    message.error(String(err))
  }
}

function openCreateTemplateModal() {
  newTemplateName.value = ''
  newTemplateContent.value = promptDraft.value || DEFAULT_WEB_AI_PROMPT
  showCreateTemplateModal.value = true
}

async function handleCreateTemplate() {
  const name = newTemplateName.value.trim()
  if (!name) {
    message.warning(t('codexReport.templateNameRequired'))
    return
  }
  const newTemplate: CodexReportPromptTemplate = {
    id: `custom-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
    name,
    content: newTemplateContent.value.trim() || DEFAULT_WEB_AI_PROMPT,
    isBuiltin: false,
    sortOrder: promptTemplates.value.length + 1,
    createdAt: Date.now(),
    updatedAt: Date.now(),
  }
  try {
    await saveCodexReportTemplate(newTemplate)
    promptTemplates.value.push(newTemplate)
    selectTemplate(newTemplate)
    emit('update:activePrompt', newTemplate.content)
    showCreateTemplateModal.value = false
    message.success(t('common.savedSuccessfully'))
  } catch (err) {
    message.error(String(err))
  }
}

function handleDeleteTemplate(template: CodexReportPromptTemplate) {
  if (template.isBuiltin) return
  dialog.warning({
    title: t('common.delete') || '删除模板',
    content: t('codexReport.deleteTemplateConfirm', { name: template.name }),
    positiveText: t('common.confirm') || '确定',
    negativeText: t('common.cancel') || '取消',
    onPositiveClick: async () => {
      try {
        await deleteCodexReportTemplate(template.id)
        promptTemplates.value = promptTemplates.value.filter(t => t.id !== template.id)
        if (activeTemplateId.value === template.id) {
          const next = promptTemplates.value[0]
          if (next) {
            selectTemplate(next)
            emit('update:activePrompt', next.content)
          }
        }
        message.success(t('codexReport.deleteTemplateSuccess'))
      } catch (err) {
        message.error(String(err))
      }
    },
  })
}

function handleRestoreBuiltin() {
  dialog.warning({
    title: t('codexReport.restoreDefault'),
    content: t('codexReport.restoreBuiltinConfirm'),
    positiveText: t('common.confirm') || '确定',
    negativeText: t('common.cancel') || '取消',
    onPositiveClick: async () => {
      try {
        const refreshed = await resetBuiltinCodexReportTemplates()
        promptTemplates.value = refreshed
        const curr = refreshed.find(t => t.id === activeTemplateId.value) || refreshed[0]
        if (curr) {
          selectTemplate(curr)
          emit('update:activePrompt', curr.content)
        }
        message.success(t('codexReport.restoreBuiltinSuccess'))
      } catch (err) {
        message.error(String(err))
      }
    },
  })
}
</script>

<style scoped lang="scss">
.prompt-drawer-container {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 12px;
  height: 100%;
  min-height: 0;
}

.prompt-preset-row {
  align-items: flex-start;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.preset-label {
  color: var(--lumina-text-secondary);
  font-size: 11.5px;
  font-weight: 500;
  white-space: nowrap;
}

.preset-buttons {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.preset-chip {
  align-items: center;
  background: var(--lumina-surface-3);
  border: 0.5px solid var(--lumina-separator);
  border-radius: var(--lumina-radius-xs);
  color: var(--lumina-text);
  cursor: pointer;
  display: inline-flex;
  font-size: 11px;
  font-weight: 500;
  gap: 5px;
  padding: 3px 8px;
  transition: all var(--lumina-duration-fast) var(--lumina-ease-out);

  &:hover {
    background: var(--lumina-surface-elevated);
    border-color: var(--lumina-primary);
    color: var(--lumina-primary);
  }

  &.is-active {
    background: var(--lumina-primary-soft);
    border-color: var(--lumina-primary);
    color: var(--lumina-primary);
    font-weight: 600;
  }
}

.chip-tag {
  background: var(--lumina-surface-2);
  border-radius: 2px;
  font-size: 9.5px;
  opacity: 0.75;
  padding: 0 3px;
}

.chip-del {
  border-radius: 50%;
  color: var(--lumina-text-tertiary);
  font-size: 13px;
  height: 14px;
  line-height: 13px;
  text-align: center;
  width: 14px;

  &:hover {
    background: var(--lumina-surface-3);
    color: var(--lumina-danger, #ef4444);
  }
}

.new-template-btn {
  border-style: dashed;
  color: var(--lumina-text-secondary);

  &:hover {
    border-style: solid;
    color: var(--lumina-primary);
  }
}

.prompt-editor-textarea {
  flex: 1;
  min-height: 0;
}

.prompt-drawer-container :deep(.prompt-editor-textarea .n-input-wrapper),
.prompt-drawer-container :deep(.prompt-editor-textarea .n-input__textarea-el) {
  height: 100%;
}

.drawer-footer {
  align-items: center;
  display: flex;
  justify-content: space-between;
  width: 100%;
}

.create-template-form {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 8px 0;
}

.form-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.field-label {
  color: var(--lumina-text-secondary);
  font-size: 12px;
  font-weight: 500;
}

.modal-footer-actions {
  align-items: center;
  display: flex;
  gap: 8px;
}
</style>
