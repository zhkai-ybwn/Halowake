import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
import { emit, listen, type UnlistenFn } from '@tauri-apps/api/event'

let logWindow: WebviewWindow | null = null
let initListener: Promise<UnlistenFn> | null = null
let ready = false
let pendingData: { repoPath: string; filePath: string; branch: string } | null = null

export async function openGitLogWindow(repoPath: string, filePath: string, branch: string) {
  pendingData = { repoPath, filePath, branch }

  if (logWindow) {
    const current = logWindow
    try {
      await current.setFocus()
      await sendRequest()
      return
    } catch {
      if (logWindow === current) resetWindow()
    }
  }

  initListener ??= listen('git-log-request-init', () => {
    ready = true
    void sendRequest().catch(error => console.error(error))
  })
  try {
    await initListener
  } catch (error) {
    resetWindow()
    throw error
  }
  if (logWindow) return

  const window = new WebviewWindow('git-log', {
    title: 'Halowake - Git Log',
    url: '/#/log',
    width: 1200,
    height: 800,
    minWidth: 900,
    minHeight: 600,
    decorations: true,
    resizable: true,
    center: true,
  })
  logWindow = window
  const resetCurrentWindow = () => {
    if (logWindow === window) resetWindow()
  }
  void window.once('tauri://error', resetCurrentWindow)
  void window.once('tauri://destroyed', resetCurrentWindow)
}

async function sendRequest() {
  if (!ready || !pendingData) return
  const request = pendingData
  pendingData = null
  await emit('git-log-init', request)
}

function resetWindow() {
  logWindow = null
  ready = false
  void initListener?.then(unlisten => unlisten(), () => {})
  initListener = null
}

export async function openGitLogFileDiff(repoPath: string, filePath: string) {
  await emit('git-log-open-file-diff', { repoPath, filePath })
}
