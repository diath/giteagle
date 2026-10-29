import { ref, shallowRef } from 'vue'

import { api, type FileStatus, type PendingSummary, type StatusEntry } from '../api'

const statusByPath = shallowRef(new Map<string, FileStatus>())
const changedDirectories = shallowRef(new Set<string>())

/** What is uncommitted, staged or not; null until it first loads. */
export const pendingSummary = shallowRef<PendingSummary | null>(null)
/** Bumped on every update, even an unchanged one, since edits that keep the
 * totals still change the diff. Views of the changes reload on it. */
export const pendingRevision = ref(0)

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

export function applyPendingSummary(summary: PendingSummary): void {
  pendingSummary.value = summary
  pendingRevision.value += 1
}

export async function refreshWorkingTree(): Promise<void> {
  // Failures are left alone; the watcher pushes fresh copies of both every
  // few seconds anyway.
  const [entries, summary] = await Promise.allSettled([api.workingTreeStatus(), api.pendingSummary()])
  if (entries.status === 'fulfilled') {
    applyWorkingTreeStatus(entries.value)
  }
  if (summary.status === 'fulfilled') {
    applyPendingSummary(summary.value)
  }
}

export function statusOf(path: string): FileStatus | undefined {
  return statusByPath.value.get(path)
}

export function directoryHasChanges(path: string): boolean {
  return changedDirectories.value.has(path)
}
