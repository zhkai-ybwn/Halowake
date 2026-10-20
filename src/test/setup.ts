import { afterEach, beforeEach, vi } from 'vitest'

type InvokeHandler = (cmd: string, args?: Record<string, unknown>) => unknown | Promise<unknown>

function defaultInvokeHandler(cmd: string): unknown {
  switch (cmd) {
    case 'load_skill_registry':
      return null
    case 'save_skill_registry':
      return undefined
    case 'rebuild_skill_registry_fts':
      return true
    case 'search_skill_registry':
      return {
        hits: [],
        totalMatched: 0,
        matchTier: 'none',
        fallbackNotice: null,
        parsedTerms: [],
        usedFallbackEngine: false,
        elapsedMs: 1,
      }
    case 'scan_agent_skills':
    case 'check_agent_skills_updates':
    case 'list_skill_install_history':
      return []
    case 'uninstall_agent_skill':
      return true
    default:
      throw new Error(`Unmocked Tauri invoke command in unit test: ${cmd}`)
  }
}

let currentInvokeHandler: InvokeHandler = defaultInvokeHandler

export const invokeMock = vi.fn(async (cmd: string, args?: Record<string, unknown>) => {
  return await currentInvokeHandler(cmd, args)
})

export function setTauriInvokeHandler(handler: InvokeHandler): void {
  currentInvokeHandler = handler
}

export class MockChannel<T = unknown> {
  onmessage: (message: T) => void

  constructor(onmessage?: (message: T) => void) {
    this.onmessage = onmessage ?? (() => {})
  }

  send(message: T): void {
    this.onmessage(message)
  }
}

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (cmd: string, args?: Record<string, unknown>) => invokeMock(cmd, args),
  Channel: MockChannel,
}))

export const dialogOpenMock = vi.fn(async () => null)

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: (...args: unknown[]) => dialogOpenMock(...(args as [])),
}))

type FetchHandler = (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>

const blockedFetchHandler: FetchHandler = async (input: RequestInfo | URL) => {
  throw new Error(`Real network fetch is blocked in unit tests: ${String(input)}`)
}

let currentFetchHandler: FetchHandler = blockedFetchHandler

export const fetchMock = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
  return await currentFetchHandler(input, init)
})

export function setFetchHandler(handler: FetchHandler): void {
  currentFetchHandler = handler
}

Object.defineProperty(globalThis, 'fetch', {
  value: fetchMock,
  writable: true,
  configurable: true,
})
if (typeof window !== 'undefined') {
  Object.defineProperty(window, 'fetch', {
    value: fetchMock,
    writable: true,
    configurable: true,
  })
}

beforeEach(() => {
  localStorage.clear()
  sessionStorage.clear()
  // Prevent background auto-sync on store creation unless a test explicitly tests stale syncTime
  localStorage.setItem('lumina.skills.lastSyncTime', String(Date.now()))
  invokeMock.mockClear()
  dialogOpenMock.mockClear()
  fetchMock.mockClear()
  currentInvokeHandler = defaultInvokeHandler
  currentFetchHandler = blockedFetchHandler
})

afterEach(() => {
  currentInvokeHandler = defaultInvokeHandler
  currentFetchHandler = blockedFetchHandler
  localStorage.clear()
  sessionStorage.clear()
})
