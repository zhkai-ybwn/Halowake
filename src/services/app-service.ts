import { getVersion } from '@tauri-apps/api/app'
import { invoke } from '@tauri-apps/api/core'

export async function getApplicationVersion(): Promise<string> {
  return await getVersion()
}

export async function openExternalUrl(url: string): Promise<void> {
  await invoke('open_project_url', { url })
}
