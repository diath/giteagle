import { shallowRef } from 'vue'

import { api, type FileStatus, type StatusEntry } from '../api'

const statusByPath = shallowRef(new Map<string, FileStatus>())
const changedDirectories = shallowRef(new Set<string>())

export function applyWorkingTreeStatus(entries: StatusEntry[]): void {
  const byPath = new Map<string, FileStatus>()
  const directories = new Set<string>()
  for (const entry of entries) {
    byPath.set(entry.path, entry.status)
    const segments = entry.path.split('/')
    for (let length = 1; length < segments.length; length += 1) {
      directories.add(segments.slice(0, length).join('/'))
    }
  }
  statusByPath.value = byPath
  changedDirectories.value = directories
}

export async function refreshWorkingTree(): Promise<void> {
  try {
    applyWorkingTreeStatus(await api.workingTreeStatus())
  } catch {
    // The watcher pushes a fresh status every few seconds anyway.
  }
}

export function statusOf(path: string): FileStatus | undefined {
  return statusByPath.value.get(path)
}

export function directoryHasChanges(path: string): boolean {
  return changedDirectories.value.has(path)
}
