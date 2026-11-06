import { onActivated, onDeactivated, onMounted, onUnmounted, watch } from 'vue'

// KeepAlive and window visibility are independent. An in-flight request is never duplicated.
export function useVisiblePolling(
  task: () => Promise<unknown>, interval: number, enabled: () => boolean = () => true, immediate = true,
) {
  let active = false
  let busy = false
  let timer: ReturnType<typeof setInterval> | undefined
  const canRun = () => active && !document.hidden && enabled()
  async function run() {
    if (busy || !canRun()) return
    busy = true
    try { await task() } catch (error) { console.error('Polling failed:', error) }
    finally { busy = false }
  }
  function stop() { clearInterval(timer); timer = undefined }
  function sync() {
    stop()
    if (!canRun()) return
    if (immediate) void run()
    timer = setInterval(() => { void run() }, interval)
  }
  function activate() { if (!active) { active = true; sync() } }
  function deactivate() { active = false; stop() }
  watch(enabled, sync)
  onMounted(() => { document.addEventListener('visibilitychange', sync); activate() })
  onActivated(activate)
  onDeactivated(deactivate)
  onUnmounted(() => { deactivate(); document.removeEventListener('visibilitychange', sync) })
  return { refresh: run }
}
