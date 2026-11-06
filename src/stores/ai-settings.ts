import { defineStore } from 'pinia'
import { loadAiSettingsFile, saveAiSettingsFile } from '@/services/ai-settings-service'
import type { AiModelConfig, AiSettings, AiTaskType } from '@/types/ai-settings'

const STORAGE_KEY = 'lumina.ai.settings.v1'

const DEFAULT_SETTINGS: AiSettings = {
  defaultModelId: '',
  models: [],
  taskModelMap: {
    'commit-message': '',
    'change-summary': '',
  },
}

function cloneSettings(settings: AiSettings): AiSettings {
  return {
    defaultModelId: settings.defaultModelId,
    models: settings.models.map(model => ({ ...model })),
    taskModelMap: { ...settings.taskModelMap },
  }
}

function normalizeSettings(settings: Partial<AiSettings>): AiSettings {
  const models = Array.isArray(settings.models) ? settings.models.filter(isValidModelConfig) : []
  const validModelIds = new Set(models.map(model => model.id))
  const defaultModelId = settings.defaultModelId && validModelIds.has(settings.defaultModelId)
    ? settings.defaultModelId
    : models[0]?.id ?? ''

  return {
    defaultModelId,
    models,
    taskModelMap: {
      'commit-message': resolveTaskModel(settings.taskModelMap?.['commit-message'], validModelIds, defaultModelId),
      'change-summary': resolveTaskModel(settings.taskModelMap?.['change-summary'], validModelIds, defaultModelId),
    },
  }
}

function readLegacyLocalSettings(): AiSettings {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (!raw) return cloneSettings(DEFAULT_SETTINGS)

    const parsed = JSON.parse(raw) as Partial<AiSettings>
    return normalizeSettings(parsed)
  } catch (error) {
    console.error(error)
    return cloneSettings(DEFAULT_SETTINGS)
  }
}

function isValidModelConfig(model: unknown): model is AiModelConfig {
  if (!model || typeof model !== 'object') return false
  const value = model as Partial<AiModelConfig>
  return Boolean(
    value.id &&
      value.name &&
      value.model &&
      value.baseUrl &&
      (value.provider === 'openai-compatible' || value.provider === 'ollama'),
  )
}

function resolveTaskModel(modelId: string | undefined, validModelIds: Set<string>, defaultModelId: string) {
  return modelId && validModelIds.has(modelId) ? modelId : defaultModelId
}

function createModelId() {
  return `model-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`
}

export const AI_TASKS: Array<{ key: AiTaskType; i18nKey: string }> = [
  { key: 'commit-message', i18nKey: 'settings.aiRouting.tasks.commitMessage' },
  { key: 'change-summary', i18nKey: 'settings.aiRouting.tasks.changeSummary' },
]

export const useAiSettingsStore = defineStore('ai-settings', {
  state: () => cloneSettings(DEFAULT_SETTINGS),

  getters: {
    enabledModels: state => state.models.filter(model => model.enabled),
    defaultModel: state => state.models.find(model => model.id === state.defaultModelId && model.enabled) ?? null,
  },

  actions: {
    async initSettings() {
      try {
        const fileSettings = await loadAiSettingsFile()
        if (fileSettings) {
          Object.assign(this, normalizeSettings(fileSettings))
          return
        }
      } catch (error) {
        console.error(error)
      }

      const legacySettings = readLegacyLocalSettings()
      Object.assign(this, legacySettings)
      if (legacySettings.models.length) {
        this.persist()
      }
    },

    persist() {
      const settings = {
        defaultModelId: this.defaultModelId,
        models: this.models,
        taskModelMap: this.taskModelMap,
      }

      void saveAiSettingsFile(settings).catch(error => {
        console.error(error)
      })
    },

    createModel() {
      const model: AiModelConfig = {
        id: createModelId(),
        name: 'OpenAI Compatible',
        provider: 'openai-compatible',
        baseUrl: 'https://api.example.com/v1',
        apiKey: '',
        model: '',
        enabled: true,
      }

      this.models = [...this.models, model]
      if (!this.defaultModelId) {
        this.defaultModelId = model.id
        for (const task of AI_TASKS) {
          this.taskModelMap[task.key] = model.id
        }
      }
      this.persist()
      return model.id
    },

    updateModel(modelId: string, patch: Partial<AiModelConfig>) {
      this.models = this.models.map(model => (model.id === modelId ? { ...model, ...patch } : model))
      this.persist()
    },

    deleteModel(modelId: string) {
      this.models = this.models.filter(model => model.id !== modelId)
      const fallbackId = this.models.find(model => model.enabled)?.id ?? this.models[0]?.id ?? ''

      if (this.defaultModelId === modelId) {
        this.defaultModelId = fallbackId
      }

      for (const task of AI_TASKS) {
        if (this.taskModelMap[task.key] === modelId) {
          this.taskModelMap[task.key] = this.defaultModelId || fallbackId
        }
      }

      this.persist()
    },

    setDefaultModel(modelId: string) {
      if (modelId && !this.models.some(model => model.id === modelId)) return
      this.defaultModelId = modelId
      this.persist()
    },

    setTaskModel(task: AiTaskType, modelId: string) {
      if (modelId && !this.models.some(model => model.id === modelId)) return
      this.taskModelMap = {
        ...this.taskModelMap,
        [task]: modelId,
      }
      this.persist()
    },

    getModelForTask(task: AiTaskType) {
      const modelId = this.taskModelMap[task] || this.defaultModelId
      return this.models.find(model => model.id === modelId && model.enabled) ?? this.defaultModel
    },
  },
})
