import { ref, shallowRef } from 'vue'

import type { FileStatus, PendingSummary, ProjectApi, StatusEntry } from '../api'

/** Creates the store of a project's uncommitted changes, which the watcher keeps current. */
export function createWorkingTree(api: ProjectApi) {
  const statusByPath = shallowRef(new Map<string, FileStatus>())
  const changedDirectories = shallowRef(new Set<string>())

  /** What is uncommitted, staged or not; null until it first loads. */
  const pendingSummary = shallowRef<PendingSummary | null>(null)
  /** Bumped on every update, even an unchanged one, since edits that keep the
   * totals still change the diff. Views of the changes reload on it. */
  const pendingRevision = ref(0)

  function applyWorkingTreeStatus(entries: StatusEntry[]): void {
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

  function applyPendingSummary(summary: PendingSummary): void {
    pendingSummary.value = summary
    pendingRevision.value += 1
  }

  async function refreshWorkingTree(): Promise<void> {
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

  function statusOf(path: string): FileStatus | undefined {
    return statusByPath.value.get(path)
  }

  function directoryHasChanges(path: string): boolean {
    return changedDirectories.value.has(path)
  }

  return {
    pendingSummary,
    pendingRevision,
    applyWorkingTreeStatus,
    applyPendingSummary,
    refreshWorkingTree,
    statusOf,
    directoryHasChanges,
  }
}
