import { markRaw, reactive } from 'vue'

import type { CommitSummary } from '../api'

/** A list of commits, newest first, that a Commit View can step through. */
export interface CommitSequence {
  /** The commits loaded so far. */
  readonly commits: readonly CommitSummary[]
  /** Whether older commits are still to be loaded. */
  readonly hasMore: boolean
  /** Loads the next page of older commits. */
  loadMore(): Promise<void>
}

export type View =
  | { kind: 'main' }
  | { kind: 'fileHistory'; path: string }
  | { kind: 'commit'; id: string; fromFile: string | null; sequence: CommitSequence | null }
  | { kind: 'pending' }
  | { kind: 'stash'; id: string }
  | { kind: 'file'; commit: string; shortId: string; path: string }

export type MainTab = 'commits' | 'files' | 'stash' | 'authors'

export interface OpenCommitOptions {
  /** The file whose history the commit was opened from. */
  fromFile?: string | null
  /** The list the commit was opened from, to step to its neighbors. */
  sequence?: CommitSequence | null
}

/**
 * Creates a project's stack of views: the main view at the bottom, then file
 * history, commit, stash and file views. Views below the top stay mounted, so going
 * back restores their scroll position and state.
 */
export function createNavigation() {
  const navigation = reactive({
    stack: [{ kind: 'main' }] as View[],
    tab: 'commits' as MainTab,
    directory: '',
    /** A search started outside the Commits tab, which runs and clears it. */
    commitSearch: null as string | null,
  })

  /** Shows the Commits tab with `query` in its search field, already run. */
  function searchCommits(query: string): void {
    navigation.commitSearch = query
    navigation.tab = 'commits'
  }

  function openCommit(id: string, { fromFile = null, sequence = null }: OpenCommitOptions = {}): void {
    // Kept raw, as the list it reads from is reactive on its own.
    navigation.stack.push({ kind: 'commit', id, fromFile, sequence: sequence && markRaw(sequence) })
  }

  /**
   * Shows commit `to` in the Commit View on top while it still shows `from`,
   * e.g. to step through its list, so going back still returns to the list.
   */
  function replaceCommit(from: string, to: string): void {
    const top = navigation.stack.length - 1
    const view = navigation.stack[top]
    if (view.kind === 'commit' && view.id === from) {
      navigation.stack.splice(top, 1, { ...view, id: to })
    }
  }

  function openPendingChanges(): void {
    navigation.stack.push({ kind: 'pending' })
  }

  function openStash(id: string): void {
    navigation.stack.push({ kind: 'stash', id })
  }

  /** Shows the file at `path` as it is in commit `commit`. */
  function openFileAt(commit: string, shortId: string, path: string): void {
    navigation.stack.push({ kind: 'file', commit, shortId, path })
  }

  function openFileHistory(path: string): void {
    // From a commit opened in this file's history, going back keeps that
    // history's place instead of stacking a second copy of it.
    const top = navigation.stack.length - 1
    const below = navigation.stack[top - 1] as View | undefined
    if (navigation.stack[top].kind === 'commit' && below?.kind === 'fileHistory' && below.path === path) {
      goBack()
      return
    }
    navigation.stack.push({ kind: 'fileHistory', path })
  }

  function goBack(): void {
    if (navigation.stack.length > 1) {
      navigation.stack.pop()
    }
  }

  /** Drops every view above the main one, e.g. after a branch switch. */
  function returnToMain(): void {
    navigation.stack.splice(1)
  }

  return {
    navigation,
    searchCommits,
    openCommit,
    replaceCommit,
    openPendingChanges,
    openStash,
    openFileAt,
    openFileHistory,
    goBack,
    returnToMain,
  }
}
