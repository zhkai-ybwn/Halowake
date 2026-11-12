import { emit, listen, type UnlistenFn } from '@tauri-apps/api/event'
import { WebviewWindow } from '@tauri-apps/api/webviewWindow'

export type GitDiffRequest =
  | {
      kind: 'working-tree'
      repoPath: string
      filePath: string
      mode: 'unstaged' | 'staged' | 'head'
    }
  | {
      kind: 'commit'
      repoPath: string
      filePath: string
      hash: string
    }

let diffWindow: WebviewWindow | null = null
let initListener: Promise<UnlistenFn> | null = null
let ready = false
let pendingRequest: GitDiffRequest | null = null

export async function openGitDiffWindow(request: GitDiffRequest) {
  pendingRequest = request

  if (diffWindow) {
    const current = diffWindow
    try {
      await current.setFocus()
      await sendRequest()
      return
    } catch {
      if (diffWindow === current) resetWindow()
    }
  }

  initListener ??= listen('git-diff-request-init', () => {
    ready = true
    void sendRequest().catch(error => console.error(error))
  })
  try {
    await initListener
  } catch (error) {
    resetWindow()
    throw error
  }
  if (diffWindow) return

  const window = new WebviewWindow('git-diff', {
    title: 'Halowake - Git Diff',
    url: '/#/diff',
    width: 1200,
    height: 800,
    minWidth: 900,
    minHeight: 600,
    decorations: true,
    resizable: true,
    center: true,
  })
  diffWindow = window
  const resetCurrentWindow = () => {
    if (diffWindow === window) resetWindow()
  }
  void window.once('tauri://error', resetCurrentWindow)
  void window.once('tauri://destroyed', resetCurrentWindow)
}

async function sendRequest() {
  if (!ready || !pendingRequest) return
  const request = pendingRequest
  pendingRequest = null
  await emit('git-diff-init', request)
}

function resetWindow() {
  diffWindow = null
  ready = false
  void initListener?.then(unlisten => unlisten(), () => {})
  initListener = null
}
