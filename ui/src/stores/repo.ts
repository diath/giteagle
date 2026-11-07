import { computed, reactive } from 'vue'

import { errorMessage, type FetchStatus, type Overview, type ProjectApi } from '../api'
import { showToast } from './toasts'

export type BusyOperation =
  | 'switch'
  | 'pull'
  | 'push'
  | 'delete'
  | 'reload'
  | 'stash'
  | 'commit'
  | 'cherryPick'
  | 'revert'
  | 'reword'
  | 'squash'
  | 'drop'

const NON_CLEAN_LABELS: Record<Exclude<Overview['state'], 'clean'>, string> = {
  merge: 'Merge in progress',
  rebase: 'Rebase in progress',
  cherryPick: 'Cherry-pick in progress',
  revert: 'Revert in progress',
  bisect: 'Bisect in progress',
  applyMailbox: 'Patch application in progress',
}

/**
 * Creates the store of a project's repository: its overview, the operation
 * running on it and the revisions that tell views to reload.
 */
export function createRepo(api: ProjectApi, refreshWorkingTree: () => Promise<void>) {
  const repo = reactive({
    overview: null as Overview | null,
    error: null as string | null,
    fetch: { inProgress: false, lastSuccess: null, lastError: null } as FetchStatus,
    busy: null as BusyOperation | null,
    /** Bumped whenever every view should reload from scratch. */
    revision: 0,
    /** Bumped after a fetch, when branch metadata may have changed. */
    metadataRevision: 0,
    /** Bumped when the stash changed, here or outside GitEagle. */
    stashRevision: 0,
    reloadPromptVisible: false,
  })

  async function refreshOverview(): Promise<void> {
    try {
      repo.overview = await api.overview()
      repo.error = null
    } catch (error) {
      repo.error = errorMessage(error)
    }
  }

  /** Reloads every view, e.g. after our own changes or an external one. */
  async function refreshEverything(): Promise<void> {
    await refreshOverview()
    repo.revision += 1
    await refreshWorkingTree()
  }

  async function reloadRepository(): Promise<void> {
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

  function applyFetchStatus(status: FetchStatus): void {
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


  /** Which branch bar actions the repository state allows (§5.4, §5.7). */
  const capabilities = computed(() => {
    const overview = repo.overview
    const idle = repo.busy === null
    if (!overview) {
      return {
        canSwitch: false,
        canPull: false,
        canPush: false,
        canDelete: false,
        canStash: false,
        canDropStash: false,
        canCommit: false,
        canCherryPick: false,
        canRevert: false,
        canRewrite: false,
        notice: null,
      }
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
      // Stashing, applying and popping need a commit to work against; dropping
      // leaves the working tree alone.
      canStash: idle && clean && overview.head.kind !== 'unborn',
      canDropStash: idle,
      // A commit on a detached HEAD is easily lost by switching away from it.
      canCommit: idle && clean && overview.head.kind !== 'detached',
      // Both act on the pushed commits of the checked-out branch.
      canCherryPick: idle && clean && onBranch,
      canRevert: idle && clean && onBranch,
      // Rewording, squashing and dropping act on its unpushed commits.
      canRewrite: idle && clean && onBranch,
      notice: overview.state === 'clean' ? null : NON_CLEAN_LABELS[overview.state],
    }
  })

  return {
    repo,
    refreshOverview,
    refreshEverything,
    reloadRepository,
    applyFetchStatus,
    capabilities,
  }
}
