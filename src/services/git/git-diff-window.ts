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
let unlistenRequestInit: UnlistenFn | null = null
let pendingRequest: GitDiffRequest | null = null

export async function openGitDiffWindow(request: GitDiffRequest) {
  pendingRequest = request

  if (diffWindow) {
    try {
      await diffWindow.setFocus()
      await sendRequest()
      return
    } catch {
      diffWindow = null
    }
  }

  diffWindow = new WebviewWindow('git-diff', {
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

  diffWindow.once('tauri://error', () => {
    diffWindow = null
  })
  diffWindow.once('tauri://destroyed', () => {
    diffWindow = null
    cleanupListener()
  })

  await setupRequestListener()
  setTimeout(() => void sendRequest(), 500)
}

async function sendRequest() {
  if (pendingRequest) await emit('git-diff-init', pendingRequest)
}

function cleanupListener() {
  unlistenRequestInit?.()
  unlistenRequestInit = null
}

async function setupRequestListener() {
  cleanupListener()
  unlistenRequestInit = await listen('git-diff-request-init', () => void sendRequest())
}
