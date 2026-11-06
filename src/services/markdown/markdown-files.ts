import { invoke } from '@tauri-apps/api/core'
import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
import { emit } from '@tauri-apps/api/event'

export interface MarkdownFileInfo {
  path: string
  name: string
  content: string
  size: number
  modifiedAt: number | null
}

export interface MarkdownFileMetadata {
  path: string
  name: string
  size: number
  modifiedAt: number | null
}

export async function readMarkdownFile(filePath: string): Promise<MarkdownFileInfo> {
  return await invoke<MarkdownFileInfo>('read_markdown_file', { filePath })
}

export async function getMarkdownMetadata(filePath: string): Promise<MarkdownFileMetadata> {
  return await invoke<MarkdownFileMetadata>('get_markdown_file_metadata', { filePath })
}

export async function getInitialMarkdownFile(): Promise<string | null> {
  return await invoke<string | null>('get_initial_markdown_file')
}

export async function openMarkdownInEditor(filePath: string): Promise<void> {
  await invoke('open_markdown_in_editor', { filePath })
}

let standaloneWindow: WebviewWindow | null = null

export async function openMarkdownStandaloneWindow(filePath: string): Promise<void> {
  const fileName = filePath.split(/[/\\]/).pop() || 'Markdown Preview'

  if (standaloneWindow) {
    try {
      await standaloneWindow.setFocus()
      await emit('markdown-preview-open', { filePath })
      return
    } catch {
      standaloneWindow = null
    }
  }

  const encodedPath = encodeURIComponent(filePath)
  const windowLabel = 'markdown-preview'

  try {
    const existing = await WebviewWindow.getByLabel(windowLabel)
    if (existing) {
      await existing.setFocus()
      await emit('markdown-preview-open', { filePath })
      standaloneWindow = existing
      return
    }
  } catch {
    // Window doesn't exist, proceed to create
  }

  try {
    standaloneWindow = new WebviewWindow(windowLabel, {
      title: `Halowake - ${fileName}`,
      url: `/#/preview?file=${encodedPath}`,
      width: 1160,
      height: 820,
      minWidth: 640,
      minHeight: 480,
      decorations: true,
      resizable: true,
      center: true,
    })

    standaloneWindow.once('tauri://destroyed', () => {
      standaloneWindow = null
    })
    standaloneWindow.once('tauri://error', () => {
      standaloneWindow = null
    })
  } catch (err) {
    console.error('Failed to open standalone markdown window:', err)
  }
}
