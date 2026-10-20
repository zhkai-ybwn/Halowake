export function formatRelativeTime(
  timestamp: number,
  t?: (key: string, ...args: unknown[]) => string
): string {
  if (!timestamp || timestamp <= 0) return ''
  const diff = Date.now() - timestamp
  const mins = Math.floor(diff / 60000)
  if (mins < 1) return t ? t('quota.justNow') : '刚刚'
  if (mins < 60) return `${mins} ${t ? t('quota.minsAgo') : '分钟前'}`
  const hours = Math.floor(mins / 60)
  return `${hours} ${t ? t('quota.hoursAgo') : '小时前'}`
}
