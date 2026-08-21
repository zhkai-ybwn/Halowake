function currentPlatform() {
  if (typeof navigator === 'undefined') return ''
  const navigatorWithUserAgentData = navigator as Navigator & {
    userAgentData?: { platform?: string }
  }
  return navigatorWithUserAgentData.userAgentData?.platform || navigator.platform || navigator.userAgent
}

export const isMacPlatform = /Mac|iPhone|iPad/i.test(currentPlatform())
export const primaryModifierLabel = isMacPlatform ? '⌘' : 'Ctrl'

export function formatPrimaryShortcut(key: string) {
  const normalizedKey = key.toLowerCase() === 'enter'
    ? (isMacPlatform ? '↵' : 'Enter')
    : key.toUpperCase()
  return isMacPlatform ? `${primaryModifierLabel}${normalizedKey}` : `${primaryModifierLabel}+${normalizedKey}`
}

export function hasPrimaryModifier(event: KeyboardEvent) {
  return isMacPlatform ? event.metaKey : event.ctrlKey
}
