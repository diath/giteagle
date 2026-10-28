import { computed, reactive } from 'vue'

import { api, errorMessage, type FetchStatus, type Overview } from '../api'
import { showToast } from './toasts'
import { refreshWorkingTree } from './workingTree'

export type BusyOperation = 'switch' | 'pull' | 'push' | 'delete' | 'reload'

export const repo = reactive({
  overview: null as Overview | null,
  error: null as string | null,
  fetch: { inProgress: false, lastSuccess: null, lastError: null } as FetchStatus,
  busy: null as BusyOperation | null,
  /** Bumped whenever every view should reload from scratch. */
  revision: 0,
  /** Bumped after a fetch, when branch metadata may have changed. */
  metadataRevision: 0,
  reloadPromptVisible: false,
})

export async function refreshOverview(): Promise<void> {
  try {
    repo.overview = await api.overview()
    repo.error = null
  } catch (error) {
    repo.error = errorMessage(error)
  }
}

/** Reloads every view, e.g. after our own changes or an external one. */
export async function refreshEverything(): Promise<void> {
  await refreshOverview()
  repo.revision += 1
  await refreshWorkingTree()
}

export async function reloadRepository(): Promise<void> {
  repo.reloadPromptVisible = false
  repo.busy = 'reload'
  try {
    await api.reload()
    await refreshEverything()
  } catch (error) {
    showToast('error', `Reloading failed: ${errorMessage(error)}`)
    repo.reloadPromptVisible = true
  } finally {
    repo.busy = null
  }
}

export function applyFetchStatus(status: FetchStatus): void {
  // A changed success time also counts, because the start event of the
  // first fetch can arrive before the UI listens for it.
  const finished =
    !status.inProgress &&
    (repo.fetch.inProgress || status.lastSuccess !== repo.fetch.lastSuccess)
  repo.fetch = status
  if (finished && status.lastError === null) {
    repo.metadataRevision += 1
    void refreshOverview()
  }
}

const NON_CLEAN_LABELS: Record<Exclude<Overview['state'], 'clean'>, string> = {
  merge: 'Merge in progress',
  rebase: 'Rebase in progress',
  cherryPick: 'Cherry-pick in progress',
  revert: 'Revert in progress',
  bisect: 'Bisect in progress',
  applyMailbox: 'Patch application in progress',
}

/** Which branch bar actions the repository state allows (§5.4, §5.7). */
export const capabilities = computed(() => {
  const overview = repo.overview
  const idle = repo.busy === null
  if (!overview) {
    return { canSwitch: false, canPull: false, canPush: false, canDelete: false, notice: null }
  }
  const clean = overview.state === 'clean'
  const onBranch = overview.head.kind === 'branch'
  const hasRemotes = overview.remotes.length > 0
  const hasUpstream = overview.head.upstream !== null
  return {
    canSwitch: idle && clean && overview.branchCount > 0,
    canPull: idle && clean && onBranch && hasRemotes && hasUpstream,
    canPush: idle && clean && onBranch && hasRemotes && (hasUpstream || overview.hasOrigin),
    canDelete: idle && clean && onBranch,
    notice: overview.state === 'clean' ? null : NON_CLEAN_LABELS[overview.state],
  }
})
