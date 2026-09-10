import { defineStore } from 'pinia'
import {
  loadAllQuotas,
  refreshSingleQuota,
  loadQuotaAccounts,
  type ProviderQuota,
  type QuotaSummary,
  type QuotaKind,
} from '@/services/quota/quota-service'

export type OverallStatusTone = 'healthy' | 'warning' | 'danger' | 'idle'

function recalculateSummary(quotas: ProviderQuota[]): QuotaSummary {
  let totalCnyBalance = 0
  let totalUsdBalance = 0
  let totalCredits = 0
  let activeAccountsCount = 0
  let warningAccountsCount = 0

  for (const q of quotas) {
    if (q.isHealthy) {
      activeAccountsCount++
    }
    if (!q.isHealthy || q.errorMessage) {
      warningAccountsCount++
    }
    for (const item of q.quotas) {
      if (item.type === 'balance') {
        if (item.currency === 'USD') {
          totalUsdBalance += item.totalRemaining
        } else {
          totalCnyBalance += item.totalRemaining
        }
      } else if (item.type === 'credits') {
        totalCredits += item.remaining
      }
    }
  }

  return {
    totalCnyBalance,
    totalUsdBalance,
    totalCredits,
    activeAccountsCount,
    warningAccountsCount,
  }
}

export const useQuotaStore = defineStore('quota', {
  state: () => ({
    quotas: [] as ProviderQuota[],
    summary: {
      totalCnyBalance: 0,
      totalUsdBalance: 0,
      totalCredits: 0,
      activeAccountsCount: 0,
      warningAccountsCount: 0,
    } as QuotaSummary,
    loading: false,
    refreshing: false,
    lastUpdated: 0,
  }),

  getters: {
    /**
     * 根据所有活跃账号的实际用量百分比、配速与余额，综合评估整体健康度：
     * 1. 严重告警 (danger / 红点): 任意活跃账号限额剩余 <= 10%，或配速预计超标 (overPace)，或余额已耗尽 (<= 0)
     * 2. 偏紧告警 (warning / 黄点): 任意活跃账号限额剩余 <= 30%，或配速用量偏紧 (tight)
     * 3. 正常健康 (healthy / 绿点): 活跃账号百分比均 > 30% 且配速平稳
     * 4. 闲置 (idle / 灰点): 暂未接入任何账号
     */
    statusTone(state): OverallStatusTone {
      const list = state.quotas
      if (!list.length) return 'idle'

      const healthyList = list.filter(q => q.isHealthy)
      if (!healthyList.length) {
        // 如果全部账号均连接失败/不可用，才显示黄色警告
        return state.summary.warningAccountsCount > 0 ? 'warning' : 'idle'
      }

      let hasDanger = false
      let hasWarning = false

      for (const q of healthyList) {
        // 1. 检查用量配速预测
        if (q.pace?.level === 'overPace') {
          hasDanger = true
        } else if (q.pace?.level === 'tight') {
          hasWarning = true
        }

        // 2. 检查周期滑动窗口限额百分比 (5h / weekly)
        const rateLimits = q.quotas.filter(
          (item): item is Extract<QuotaKind, { type: 'rateLimit' }> => item.type === 'rateLimit'
        )
        for (const rl of rateLimits) {
          const remaining = Math.max(0, 100 - rl.usedPercent)
          if (remaining <= 10) {
            hasDanger = true
          } else if (remaining <= 30) {
            hasWarning = true
          }
        }

        // 3. 检查法币余额耗尽情况
        const balances = q.quotas.filter(
          (item): item is Extract<QuotaKind, { type: 'balance' }> => item.type === 'balance'
        )
        for (const b of balances) {
          if (b.totalRemaining <= 0.0) {
            hasWarning = true
          }
        }
      }

      if (hasDanger) return 'danger'
      if (hasWarning) return 'warning'

      return 'healthy'
    },

    statusDotClass(): string {
      return this.statusTone
    },
  },

  actions: {
    async fetchAll(silent = false): Promise<[ProviderQuota[], QuotaSummary]> {
      if (!silent) this.loading = true
      this.refreshing = true
      try {
        const [list, sum] = await loadAllQuotas()
        this.quotas = list
        this.summary = sum
        this.lastUpdated = Date.now()
        return [list, sum]
      } catch (err) {
        console.warn('Failed to fetch all quotas in store:', err)
        throw err
      } finally {
        if (!silent) this.loading = false
        this.refreshing = false
      }
    },

    async refreshSingle(accountId: string): Promise<ProviderQuota | null> {
      try {
        const accounts = await loadQuotaAccounts()
        const target = accounts.find(a => a.id === accountId)
        if (!target) return null

        const updated = await refreshSingleQuota(target)
        const idx = this.quotas.findIndex(q => q.accountId === accountId)
        if (idx !== -1) {
          this.quotas[idx] = updated
        } else {
          this.quotas.push(updated)
        }
        this.summary = recalculateSummary(this.quotas)
        this.lastUpdated = Date.now()
        return updated
      } catch (err) {
        console.warn('Failed to refresh single quota:', err)
        throw err
      }
    },

    async ensureFresh(maxAgeMs = 15000): Promise<void> {
      if (Date.now() - this.lastUpdated > maxAgeMs && !this.loading && !this.refreshing) {
        await this.fetchAll(true)
      }
    },
  },
})
