import { invoke } from '@tauri-apps/api/core'

export type ProviderType =
  | 'codex'
  | 'claude'
  | 'gemini'
  | 'deepseek'
  | 'openrouter'
  | 'opencode'
  | 'workbuddy'
  | 'siliconflow'
  | 'moonshot'
  | 'zhipu'
  | 'qwen'
  | 'minimax'
  | 'cursor'
  | 'qcode'
  | 'trae'
  | 'zcode'
  | 'custom'

export interface AccountConfig {
  id: string
  providerType: ProviderType
  name: string
  apiKey?: string
  baseUrl?: string
  enabled: boolean
  autoDiscovered: boolean
}

export type QuotaKind =
  | {
      type: 'balance'
      currency: string
      toppedUp: number
      granted: number
      totalRemaining: number
    }
  | {
      type: 'rateLimit'
      periodLabel: string
      usedPercent: number
      resetsAt?: number
      resetsInSeconds?: number
    }
  | {
      type: 'credits'
      label?: string
      remaining: number
      total?: number
      expiresAt?: string
      expiresAtTimestamp?: number
      remainingDays?: number
      cycleType?: string
      unit?: string
    }

export type PaceLevel = 'onPace' | 'tight' | 'overPace' | 'unknown'

export interface PaceStatus {
  level: PaceLevel
  projectedUsagePercent?: number
  message: string
}

export interface ResetCreditItem {
  id: string
  status: string
  title?: string
  grantedAt?: string
  expiresAt?: string
  expiresAtTimestamp?: number
  expiresInSeconds?: number
}

export interface ResetCredits {
  availableCount: number
  applicableAvailableCount?: number
  nearestExpiresAt?: number
  nearestExpiresInSeconds?: number
  items?: ResetCreditItem[]
}

export interface ProviderQuota {
  id: string
  accountId: string
  providerType: ProviderType
  name: string
  plan?: string
  quotas: QuotaKind[]
  pace?: PaceStatus
  resetCredits?: ResetCredits
  lastUpdated: number
  isHealthy: boolean
  errorMessage?: string
  officialDashboardUrl?: string
}

export interface QuotaSummary {
  totalCnyBalance: number
  totalUsdBalance: number
  totalCredits: number
  activeAccountsCount: number
  warningAccountsCount: number
}

export interface ClientLaunchResult {
  providerType: ProviderType
  appName: string
  launched: boolean
}

export async function loadAllQuotas(): Promise<[ProviderQuota[], QuotaSummary]> {
  return await invoke<[ProviderQuota[], QuotaSummary]>('load_all_quotas')
}

export async function refreshSingleQuota(account: AccountConfig): Promise<ProviderQuota> {
  return await invoke<ProviderQuota>('refresh_single_quota', { account })
}

export async function loadQuotaAccounts(): Promise<AccountConfig[]> {
  return await invoke<AccountConfig[]>('load_quota_accounts')
}

export async function saveQuotaAccounts(accounts: AccountConfig[]): Promise<void> {
  await invoke('save_quota_accounts', { accounts })
}

export async function discoverLocalAiAccounts(): Promise<AccountConfig[]> {
  return await invoke<AccountConfig[]>('discover_local_ai_accounts')
}

export async function launchAiClient(providerType: ProviderType): Promise<ClientLaunchResult> {
  return await invoke<ClientLaunchResult>('launch_ai_client', { providerType })
}
