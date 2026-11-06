export type AiProviderType = 'openai-compatible' | 'ollama'

export type AiTaskType = 'commit-message' | 'change-summary'

export interface AiModelConfig {
  id: string
  name: string
  provider: AiProviderType
  baseUrl: string
  apiKey?: string
  model: string
  enabled: boolean
}

export interface AiSettings {
  defaultModelId: string
  models: AiModelConfig[]
  taskModelMap: Record<AiTaskType, string>
}

