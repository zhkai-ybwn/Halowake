import type { GitFileStatus } from '@/types/git'

export function normalizePath(path: string) {
  return path.replace(/\\/g, '/')
}

export function getFileName(path: string) {
  const normalized = normalizePath(path)
  const parts = normalized.split('/')
  return parts[parts.length - 1] || path
}

export function getDirName(path: string) {
  const normalized = normalizePath(path)
  const index = normalized.lastIndexOf('/')
  return index === -1 ? '' : normalized.slice(0, index)
}

export function getFileExtension(path: string) {
  const name = getFileName(path)
  const index = name.lastIndexOf('.')
  return index === -1 ? '' : name.slice(index + 1).toLowerCase()
}

export function getRepoDisplayName(path: string) {
  const normalized = normalizePath(path)
  const parts = normalized.split('/').filter(Boolean)
  return parts[parts.length - 1] || path
}

export function parseGitLogDate(date: string) {
  const timestamp = Date.parse(date.replace(/\//g, '-'))
  return Number.isNaN(timestamp) ? null : timestamp
}

export function startOfDay(timestamp: number) {
  const date = new Date(timestamp)
  date.setHours(0, 0, 0, 0)
  return date.getTime()
}

export function endOfDay(timestamp: number) {
  const date = new Date(timestamp)
  date.setHours(23, 59, 59, 999)
  return date.getTime()
}

export function formatCommitLineCount(value: number | null) {
  return value === null ? '-' : String(value)
}

export function formatHistoryTime(timestamp: number) {
  return new Date(timestamp).toLocaleString()
}

export function formatLogDate(date: string) {
  const parsed = new Date(date)
  if (Number.isNaN(parsed.getTime())) return date
  return parsed.toLocaleString()
}

export function mapCommitStatusToFileType(status: string): GitFileStatus['type'] {
  const code = status.slice(0, 1)
  if (code === 'A') return 'added'
  if (code === 'D') return 'deleted'
  if (code === 'R') return 'renamed'
  if (code === 'C') return 'copied'
  if (code === 'M') return 'modified'
  return 'unknown'
}
