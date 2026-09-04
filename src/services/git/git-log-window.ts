import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
import { emit, listen, type UnlistenFn } from '@tauri-apps/api/event'

let logWindow: WebviewWindow | null = null
let unlistenRequestInit: UnlistenFn | null = null
let pendingData: { repoPath: string; filePath: string; branch: string } | null = null

export async function openGitLogWindow(repoPath: string, filePath: string, branch: string) {
  // Store the data to send
  pendingData = { repoPath, filePath, branch }

  // If window already exists, focus it and send data
  if (logWindow) {
    try {
      await logWindow.setFocus()
      await sendInitData()
      return
    } catch {
      // Window was closed, create a new one
      logWindow = null
    }
  }

  try {
    logWindow = new WebviewWindow('git-log', {
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

    // Handle window events
    logWindow.once('tauri://error', (e) => {
      console.error('Log window error:', e)
      logWindow = null
    })

    logWindow.once('tauri://destroyed', () => {
      logWindow = null
      cleanupListener()
    })

    // Setup event listener for requests from the log window
    setupRequestListener()

    // Try to send data after a delay (window might need time to load)
    setTimeout(async () => {
      await sendInitData()
    }, 500)

  } catch (err) {
    console.error('Failed to create log window:', err)
    logWindow = null
  }
}

async function sendInitData() {
  if (!pendingData) return
  try {
    await emit('git-log-init', pendingData)
  } catch (err) {
    console.error('Failed to send init data:', err)
  }
}

function cleanupListener() {
  if (unlistenRequestInit) {
    unlistenRequestInit()
    unlistenRequestInit = null
  }
}

async function setupRequestListener() {
  cleanupListener()
  unlistenRequestInit = await listen('git-log-request-init', async () => {
    await sendInitData()
  })
}

export async function openGitLogFileDiff(repoPath: string, filePath: string) {
  await emit('git-log-open-file-diff', { repoPath, filePath })
}
